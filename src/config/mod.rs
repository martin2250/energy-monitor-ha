pub mod server;

pub mod json_body;
mod structs;

use crc::{Crc, CRC_32_ISCSI};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use esp_hal::{i2c::I2C, peripherals::I2C0};
use esp_println::println;
pub use structs::*;

mod button;
pub use button::*;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal};

type Signal<T> = signal::Signal<CriticalSectionRawMutex, T>;

pub static CONFIG_MQTT: Signal<MqttConfig> = Signal::new();
pub static CONFIG_WIFI: Signal<WifiConfig> = Signal::new();
pub static CONFIG_STPM: Signal<StpmConfig> = Signal::new();
pub static CONFIG_CALIBRATION: Signal<CalibrationConfig> = Signal::new();

type AppI2C = I2C<'static, I2C0>;

static EEPROM_I2C: Mutex<CriticalSectionRawMutex, Option<AppI2C>> = Mutex::new(None);

/// returns force_ap -> wifi config was not loaded, enable AP in any case
pub async fn run_config(mut i2c: AppI2C) {
    // if everything goes well, do nothing
    if read_config(&mut i2c).await == Ok(()) {
        println!("loaded config from EEPROM");
        return;
    }

    *EEPROM_I2C.lock().await = Some(i2c);

    println!("using default config");

    CONFIG_MQTT.signal(Default::default());
    CONFIG_WIFI.signal(Default::default());
    CONFIG_STPM.signal(Default::default());
    CONFIG_CALIBRATION.signal(Default::default());

    set_ap(true);
}

const CRC: Crc<u32> = Crc::<u32>::new(&CRC_32_ISCSI);
const EEPROM_ADDR: u8 = 0b101_0000;

async fn read_config(i2c: &mut AppI2C) -> Result<(), ()> {
    let mut buffer = [0u8; 4096];

    // read into buffer in loop, i2c driver supports max read of 32 bytes
    const PAGE_SIZE: usize = 16;

    // read with address setup first
    // we have a 32K EEPROM -> two address bytes
    let buffer_write = 0u16.to_be_bytes();
    let res = i2c
        .write_read(EEPROM_ADDR, &buffer_write, &mut buffer[..PAGE_SIZE])
        .await;
    if let Err(e) = res {
        println!("error reading config from i2c {e:?}");
        return Err(());
    }

    // read remaining buffer without address setup
    let mut addr = PAGE_SIZE;
    while addr < buffer.len() {
        let res = i2c
            .read(EEPROM_ADDR, &mut buffer[addr..addr + PAGE_SIZE])
            .await;
        if let Err(e) = res {
            println!("error reading config from i2c {e:?}");
            return Err(());
        }
        addr += PAGE_SIZE;
    }

    let Ok((mqtt, buffer)) = postcard::take_from_bytes_crc32::<MqttConfig>(&buffer, CRC.digest())
    else {
        println!("error deserializing mqtt");
        return Err(());
    };

    let Ok((wifi, buffer)) = postcard::take_from_bytes_crc32::<WifiConfig>(&buffer, CRC.digest())
    else {
        println!("error deserializing wifi");
        return Err(());
    };

    let Ok((stpm, buffer)) = postcard::take_from_bytes_crc32::<StpmConfig>(&buffer, CRC.digest())
    else {
        println!("error deserializing wifi");
        return Err(());
    };

    let Ok((calibration, _)) =
        postcard::take_from_bytes_crc32::<CalibrationConfig>(&buffer, CRC.digest())
    else {
        println!("error deserializing wifi");
        return Err(());
    };

    if !mqtt.validate() {
        println!("error validating mqtt");
        return Err(());
    }

    CONFIG_MQTT.signal(mqtt);
    CONFIG_WIFI.signal(wifi);
    CONFIG_STPM.signal(stpm);
    CONFIG_CALIBRATION.signal(calibration);

    Ok(())
}

pub async fn save_config() -> Option<()> {
    // start at byte two -> keep two address bytes for i2c eeprom
    let mut buffer = [0u8; 4096 + 2];
    let mut n = 2;

    {
        let state = server::STATE.lock().await;
        let state = state.as_ref().unwrap();
        n += postcard::to_slice_crc32(&state.mqtt, &mut buffer[n..], CRC.digest())
            .ok()?
            .len();
        n += postcard::to_slice_crc32(&state.wifi, &mut buffer[n..], CRC.digest())
            .ok()?
            .len();
        n += postcard::to_slice_crc32(&state.stpm, &mut buffer[n..], CRC.digest())
            .ok()?
            .len();
        n += postcard::to_slice_crc32(&state.calibration, &mut buffer[n..], CRC.digest())
            .ok()?
            .len();
    }

    let mut i2c = EEPROM_I2C.lock().await;
    let i2c = i2c.as_mut().unwrap();

    println!("start saving to EEPROM, {n} bytes");

    // ESP32 i2c can only transfer 31 bytes at once with the current i2c implementation
    // if this restriction is lifted, increase page size to 32 (eeprom)
    const PAGE_SIZE: usize = 16;

    let mut buffer = &mut buffer[..n];
    let mut addr = 0u16;

    while buffer.len() > 2 {
        println!("writing data at addr {addr}");

        // update address in buffer
        let addr_bytes = addr.to_be_bytes();
        buffer[..2].copy_from_slice(&addr_bytes);

        // send i2c
        let len = buffer.len().min(PAGE_SIZE + 2);
        if let Err(e) = i2c.write(EEPROM_ADDR, &buffer[..len]).await {
            println!("error writing to i2c EEPROM {e:?}");
            return None;
        }

        // wait for write to finish internally
        Timer::after_millis(5).await;

        addr += PAGE_SIZE as u16;
        buffer = &mut buffer[PAGE_SIZE..];
    }

    println!("end saving to EEPROM");

    Some(())
}
