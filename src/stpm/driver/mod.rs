pub mod spi;

use crc::{Crc, CRC_8_SMBUS};
const CRC_STPM: Crc<u8> = Crc::<u8>::new(&CRC_8_SMBUS);

pub trait StpmDriver {
    type Error;

    async fn transaction(
        &mut self,
        next_read_addr: Option<u8>,
        write: Option<(u8, u16)>,
    ) -> Result<u32, Self::Error>;

    // After POR, to ensure a correct initialization, it is necessary to perform a reset of DSP and communication
    // peripherals through three SYN pulses (see Figure 29. Global startup reset) and a single SCS pulse, as shown in
    // the figure below. SCS pulse can be performed before or after SYN pulses, but minimum startup time before reset
    // (as indicated in Table 4) has to be respected.
    async fn hardware_reset(&mut self) -> Result<(), Self::Error>;

    async fn syn_pulse(&mut self) -> Result<(), Self::Error>;
}
