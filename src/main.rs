#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(ip_in_core)]
#![feature(addr_parse_ascii)]
#![feature(asm_experimental_arch)]


mod config;
mod examples_util;
mod wifi;
mod mqtt;
mod stpm;
mod zcr;
mod leds;

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::i2c::I2C;
use esp_println::println;
use esp_wifi::{initialize, EspWifiInitFor};
use esp_hal::clock::ClockControl;
use esp_hal::dma::Dma;
use esp_hal::{embassy, peripherals::Peripherals, prelude::*, timer::TimerGroup};
use esp_hal::{Rng, IO};
use static_cell::make_static;

#[main]
async fn main(spawner: Spawner) -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);

    // -------------------------------------------------------------------------
    // setup

    let peripherals = Peripherals::take();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // clocks
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let clocks = &*make_static!(clocks);

    println!("startup");

    // embassy
    embassy::init(&clocks, TimerGroup::new(peripherals.TIMG0, &clocks));

    // -------------------------------------------------------------------------
    // config

    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio32,
        io.pins.gpio33,
        100u32.kHz(),
        &clocks
    );

    config::run_config(i2c).await;

    spawner.must_spawn(config::run_button(io.pins.gpio22.into()));

    // -------------------------------------------------------------------------
    // wifi

    let wifi_init: esp_wifi::EspWifiInitialization = initialize(
        EspWifiInitFor::Wifi,
        TimerGroup::new(peripherals.TIMG1, &clocks).timer0,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let (stack_sta, stack_ap) = wifi::init_wifi(
        &spawner,
        peripherals.WIFI,
        wifi_init,
        config::is_ap_enabled(),
    );
    
    // -------------------------------------------------------------------------
    // various tasks

    spawner.must_spawn(config::server::run_config_server(stack_sta, stack_ap));

    spawner.must_spawn(mqtt::run_mqtt(&stack_sta));

    spawner.must_spawn(leds::run_led_green(stack_sta, io.pins.gpio23.into()));
    spawner.must_spawn(leds::run_led_red(io.pins.gpio13.into()));

    // -------------------------------------------------------------------------
    // zcr measurement

    zcr::zcr_setup(io.pins.gpio21.into(), peripherals.MCPWM0, 20, 1.0);

    // -------------------------------------------------------------------------
    // stpm measurement

    let dma = Dma::new(peripherals.DMA);

    spawner.must_spawn(stpm::run_stpm(
        peripherals.SPI3,
        &clocks,
        dma.spi3channel,
        io.pins.gpio17.into(),
        io.pins.gpio18.into(),
        io.pins.gpio19.into(),
        io.pins.gpio16.into(),
        io.pins.gpio25.into(),
        io.pins.gpio4.into(),
    ));

    // -------------------------------------------------------------------------
    // do nothing, TODO: replace with some task
    loop {
        Timer::after_secs(107).await;
    }
}
