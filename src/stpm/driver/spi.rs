use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;
use esp_hal::gpio::{AnyPin, Output, PushPull};

use super::{StpmDriver, CRC_STPM};

#[derive(Debug)]
pub enum StpmSpiError {
    Spi,
    CrcErrorRx { expected: u8, received: u8 },
}

pub struct StpmSpiDriver<SPI: SpiBus> {
    pub spi_device: SPI,
    pub pin_scs: AnyPin<Output<PushPull>>,
    pub pin_en: AnyPin<Output<PushPull>>,
    pub pin_syn: AnyPin<Output<PushPull>>,
}

impl<SPI: SpiBus> StpmDriver for StpmSpiDriver<SPI> {
    type Error = StpmSpiError;

    async fn transaction(
        &mut self,
        next_read_addr: Option<u8>,
        write: Option<(u8, u16)>,
    ) -> Result<u32, StpmSpiError> {
        let mut buf_tx = [0u8; 5];
        buf_tx[0] = next_read_addr.unwrap_or(0xff);
        if let Some((addr, val)) = write {
            buf_tx[1] = addr;
            let val_buf = val.to_le_bytes();
            buf_tx[2] = val_buf[0];
            buf_tx[3] = val_buf[1];
        } else {
            buf_tx[1] = 0xff;
        }
        buf_tx[4] = CRC_STPM.checksum(&buf_tx[..4]);

        let _ = self.pin_scs.set_low();
        Timer::after_micros(1).await;
        let mut buf_rx = [0u8; 5];
        if let Err(_) = self.spi_device.transfer(&mut buf_rx, &buf_tx).await {
            return Err(StpmSpiError::Spi);
        }
        let _ = self.pin_scs.set_high();

        let crc_calc_rx = CRC_STPM.checksum(&buf_rx[..4]);

        if crc_calc_rx != buf_rx[4] {
            Err(StpmSpiError::CrcErrorRx {
                expected: crc_calc_rx,
                received: buf_rx[4],
            })
        } else {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&buf_rx[..4]);
            Ok(u32::from_le_bytes(buf))
        }
    }

    async fn hardware_reset(&mut self) -> Result<(), Self::Error> {
        let _ = self.pin_syn.set_high();
        // reset sequence for SPI
        let _ = self.pin_en.set_low();
        let _ = self.pin_scs.set_low();
        Timer::after_millis(5).await;
        let _ = self.pin_en.set_high();
        Timer::after_millis(5).await;
        let _ = self.pin_scs.set_high();
        // perform global reset
        Timer::after_millis(35).await;
        for _ in 0..3 {
            // syn_pulse but with more delay
            let _ = self.pin_syn.set_low();
            Timer::after_millis(5).await;
            let _ = self.pin_syn.set_high();
            Timer::after_millis(5).await;
        }
        let _ = self.pin_scs.set_low();
        Timer::after_millis(5).await;
        let _ = self.pin_scs.set_high();
        Ok(())
    }

    async fn syn_pulse(&mut self) -> Result<(), Self::Error> {
        // SYN timings from table 4 (page 10)
        // minimum pulse width: t_lpw = 4 us
        // minimum pulse spacing: t_w = 4 us
        let _ = self.pin_syn.set_low();
        Timer::after_micros(10).await;
        let _ = self.pin_syn.set_high();
        Timer::after_micros(10).await;

        Ok(())
    }
}
