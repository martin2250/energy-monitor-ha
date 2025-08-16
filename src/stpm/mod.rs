pub mod calibration;
mod chip;
mod driver;
mod sample;

pub use chip::StpmCurrentGain;
use embassy_futures::select::{select3, Either3};

use crate::{config::{self, StpmConfig, CONFIG_STPM, RESET_ACCUMULATOR}, stpm::sample::{read_samples, RawSampleChip}};
use chip::{Stpm, StpmChannelConfiguration, StpmConfiguration};
use core::fmt::Debug;
use driver::spi::StpmSpiDriver;
use driver::StpmDriver;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Ticker, Timer};
use esp_hal::{
    clock::Clocks,
    dma::{DmaPriority, Spi3DmaChannelCreator},
    dma_descriptors,
    gpio::{GpioPin, Input, Output, PullUp, PushPull},
    peripherals::SPI3,
    prelude::_fugit_RateExtU32,
    spi::{
        master::{dma::WithDmaSpi3, Spi},
        SpiMode,
    },
};
use esp_println::println;

#[derive(Copy, Clone, Debug, Default)]
pub struct RawSampleApp {
    pub voltage_rms: u64,
    // all raw values are scaled to current gain of 8
    pub current_rms: u64,
    pub power_active: i64,
    pub power_reactive: i64,
    pub energy_active: i64,
    pub num_samples: usize,
}

pub static SAMPLES: Signal<CriticalSectionRawMutex, [RawSampleApp; 2]> = Signal::new();

#[embassy_executor::task]
pub async fn run_stpm(
    spi3: SPI3,
    clocks: &'static Clocks<'static>,
    dma_channel: Spi3DmaChannelCreator,
    sclk: GpioPin<Output<PushPull>, 17>,
    mosi: GpioPin<Output<PushPull>, 18>,
    miso: GpioPin<Input<PullUp>, 19>,
    scs: GpioPin<Output<PushPull>, 16>,
    en: GpioPin<Output<PushPull>, 25>,
    syn: GpioPin<Output<PushPull>, 4>,
) {
    let (mut descriptors, mut rx_descriptors) = dma_descriptors!(256);

    let spi = Spi::new(spi3, 2.MHz(), SpiMode::Mode3, &clocks)
        .with_sck(sclk)
        .with_mosi(mosi)
        .with_miso(miso)
        .with_dma(dma_channel.configure(
            false,
            &mut descriptors,
            &mut rx_descriptors,
            DmaPriority::Priority0,
        ));

    let mut driver = StpmSpiDriver {
        spi_device: spi,
        pin_scs: scs.into(),
        pin_en: en.into(),
        pin_syn: syn.into(),
    };

    let mut config = CONFIG_STPM.wait().await;

    let mut energy_accumulator = config::read_accumulator().await.unwrap_or([0i64; 2]);

    loop {
        if None == once_stpm(&mut driver, &mut config, &mut energy_accumulator).await {
            // error -> retry after 0.5 sec
            Timer::after_millis(500).await;
        }
    }
}

async fn once_stpm<D: StpmDriver>(driver: &mut D, config: &mut StpmConfig, energy_accumulator: &mut [i64; 2]) -> Option<()>
where
    D::Error: Debug,
{
    if let Err(e) = driver.hardware_reset().await {
        println!("error during stpm hardware reset {e:?}");
        return None;
    }

    let mut chip = Stpm::new(driver);

    // do not set current / voltage calibration here, as it is only relevant for
    // LED pulse output, instead calibrate everything in software later on
    let stpm_config = StpmConfiguration {
        channels: [
            StpmChannelConfiguration {
                current_gain: config.current_gain[0],
                ..Default::default()
            },
            StpmChannelConfiguration {
                current_gain: config.current_gain[1],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    if let Err(e) = chip.configure(&stpm_config).await {
        println!("error during stpm configuration {e:?}");
        return None;
    }

    println!("stpm successfully configured");

    let anti_current_gain = config.current_gain.map(|g| match g {
        StpmCurrentGain::X2 => 8,
        StpmCurrentGain::X4 => 4,
        StpmCurrentGain::X8 => 2,
        StpmCurrentGain::X16 => 1,
    });

    let mut ticker = Ticker::every(Duration::from_millis(50));
    let mut read_errors = 0u32;

    let mut raw_samples: [RawSampleChip; 2] = Default::default();
    let mut acc_samples: [RawSampleApp; 2] = Default::default();
    let mut sample_cnt = 0;
    let mut energy_last = [0u32; 2];

    let mut accumulator_last_write = Instant::now();

    loop {
        // check if there is a new configuration / wait for ticker
        match select3(CONFIG_STPM.wait(), RESET_ACCUMULATOR.wait(), ticker.next()).await {
            Either3::First(new_config) => {
                *config = new_config;
                return Some(());
            },
            Either3::Second(_) => {
                *energy_accumulator = [0; 2];
                continue;
            },
            Either3::Third(_) => (),
        }

        // try read
        if let Err(e) = read_samples(&mut chip, &mut raw_samples).await {
            if read_errors >= 3 {
                println!("stpm too many error reading samples, restarting: {e:?}");
                return None;
            } else {
                println!("stpm error reading samples: {e:?}");
                read_errors += 1;
                continue;
            }
        }

        read_errors = read_errors.saturating_sub(1);

        // accumulate everything
        for i in 0..2 {
            let raw = &mut raw_samples[i];
            let acc = &mut acc_samples[i];

            acc.current_rms += raw.current_rms as u64;
            acc.voltage_rms += raw.voltage_rms as u64;
            acc.power_active += raw.power_active as i64;
            acc.power_reactive += raw.power_reactive as i64;
            
            // accumulate total energy in external (to this function) variables
            let energy_diff = raw.energy_active.wrapping_sub(energy_last[i]) as i32 as i64;
            energy_last[i] = raw.energy_active;

            energy_accumulator[i] += energy_diff * anti_current_gain[i];
        }

        sample_cnt += 1;
        // enough samples averaged, send to rest of app
        if sample_cnt >= config.samples_stpm {
            // update things
            for i in 0..2 {
                // apply anti-gain
                acc_samples[i].current_rms *= anti_current_gain[i] as u64;
                acc_samples[i].power_active *= anti_current_gain[i];
                acc_samples[i].power_reactive *= anti_current_gain[i];
                // update other values
                acc_samples[i].energy_active = energy_accumulator[i];
                acc_samples[i].num_samples = config.samples_stpm;
            }
            // send to MQTT
            SAMPLES.signal(acc_samples);
            // reset
            sample_cnt = 0;
            acc_samples = Default::default();
        }

        if Instant::now().duration_since(accumulator_last_write).as_millis() > 1000 {
            let _ = config::write_accumulator(energy_accumulator).await;
            accumulator_last_write = Instant::now();
        }
    }
}
