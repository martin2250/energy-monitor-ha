mod registers;
pub use registers::Reg;

mod reader;
pub use reader::Reader;

mod configuration;
pub use configuration::*;

use super::driver::StpmDriver;

pub struct Stpm<'a, D: StpmDriver> {
    pub driver: &'a mut D,
}

impl<'a, D: StpmDriver> Stpm<'a, D> {
    pub fn new(driver: &'a mut D) -> Self {
        Self { driver }
    }

    /// uses two driver transactions to read a register. for more efficient read access, use `read_registers()`
    #[allow(unused)]
    pub async fn read_register(&mut self, reg: Reg) -> Result<u32, D::Error> {
        self.driver.transaction(Some(reg.addr()), None).await?;
        self.driver.transaction(None, None).await
    }

    pub async fn write_register_16_lsw(&mut self, reg: Reg, value: u16) -> Result<(), D::Error> {
        let addr = reg.addr();
        self.driver.transaction(None, Some((addr, value))).await?;
        Ok(())
    }

    pub async fn write_register_16_msw(&mut self, reg: Reg, value: u16) -> Result<(), D::Error> {
        let addr = reg.addr() + 1;
        self.driver.transaction(None, Some((addr, value))).await?;
        Ok(())
    }

    pub async fn write_register_32(&mut self, reg: Reg, value: u32) -> Result<(), D::Error> {
        self.write_register_16_lsw(reg, (value & 0xffff) as u16)
            .await?;
        self.write_register_16_msw(reg, (value >> 16) as u16).await
    }

    pub async fn configure(&mut self, config: &StpmConfiguration) -> Result<(), D::Error> {
        self.write_register_32(Reg::DSP_CR1, 0x040000a0).await?;
        self.write_register_32(Reg::DSP_CR2, 0x240000a0).await?;

        let mut dsp_cr3 = 0x000004e0 | ((config.line_frequency as u32) << 27);
        dsp_cr3 |= 1 << 16; // enable ZCR output
        self.write_register_32(Reg::DSP_CR3, dsp_cr3).await?;

        // phase compensation
        let mut dsp_cr4 = 0;
        dsp_cr4 |= config.channels[1].current_phase_comp as u32 & 0x3ff;
        dsp_cr4 |= (config.channels[1].voltage_phase_comp as u32 & 0x3) << 10;
        dsp_cr4 |= (config.channels[0].current_phase_comp as u32 & 0x3ff) << 12;
        dsp_cr4 |= (config.channels[0].voltage_phase_comp as u32 & 0x3) << 22;
        self.write_register_32(Reg::DSP_CR4, dsp_cr4).await?;

        // voltage / current channels
        let dsp_cr5_cr8_chans = [
            (Reg::DSP_CR5, Reg::DSP_CR6, 0),
            (Reg::DSP_CR7, Reg::DSP_CR8, 1),
        ];
        for (reg_a, reg_b, idx) in dsp_cr5_cr8_chans {
            let mut val_a = 0;
            val_a |= config.channels[idx].voltage_calibration as u32 & 0xFFF;
            val_a |= (config.channels[idx].voltage_swell_threshold as u32 & 0x3FF) << 12;
            val_a |= (config.channels[idx].voltage_sag_threshold as u32 & 0x3FF) << 22;
            self.write_register_32(reg_a, val_a).await?;

            let mut val_b = 0;
            val_b |= config.channels[idx].current_calibration as u32 & 0xFFF;
            val_b |= (config.channels[idx].current_swell_threshold as u32 & 0x3FF) << 12;
            self.write_register_32(reg_b, val_b).await?;
        }

        // Ah accumulation -> only relevant for tamper detection
        self.write_register_32(Reg::DSP_CR9, 0x00000fff).await?;
        self.write_register_32(Reg::DSP_CR10, 0x00000fff).await?;
        self.write_register_32(Reg::DSP_CR11, 0x00000fff).await?;
        self.write_register_32(Reg::DSP_CR12, 0x00000fff).await?;

        // enable current and voltage channels 1+2
        let dfe_cr1 = 0x03270327 | (config.channels[0].current_gain as u32) << 26;
        let dfe_cr2 = 0x03270327 | (config.channels[1].current_gain as u32) << 26;
        self.write_register_32(Reg::DFE_CR1, dfe_cr1).await?;
        self.write_register_32(Reg::DFE_CR2, dfe_cr2).await?;

        // disable IRQs
        self.write_register_32(Reg::DSP_IRQ1, 0x00000000).await?;
        self.write_register_32(Reg::DSP_IRQ2, 0x00000000).await?;

        // CRC: enabled, poly=0x07, MSB first
        self.write_register_32(Reg::US_REG1, 0x00004007).await?;
        // UART baud rate + delay
        self.write_register_32(Reg::US_REG2, 0x00000683).await?;
        // disable all communication-related interrupts
        self.write_register_16_lsw(Reg::US_REG3, 0x0000).await?;

        Ok(())
    }
}
