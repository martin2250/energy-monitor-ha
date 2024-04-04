use crate::stpm::chip::Reader;

use super::{chip::Stpm, driver::StpmDriver};

#[derive(Copy, Clone, Debug, Default)]
pub struct RawSampleChip {
    pub voltage_rms: u32,
    pub current_rms: u32,
    pub power_active: i32,
    pub power_reactive: i32,
    pub energy_active: u32,
}

pub async fn read_samples<'a, D: StpmDriver>(chip: &mut Stpm<'a, D>, sample: &mut [RawSampleChip; 2]) -> Result<(), D::Error> {
    chip.driver.syn_pulse().await?;
    
    
    let (mut ph1_rms, mut ph2_rms) = (0, 0);

    let [ph1, ph2] = sample;

    let reader = Reader::create(chip);
    
    use super::chip::Reg::*;
    reader
        .read_u32(DSP_REG14, &mut ph1_rms).await?
        .read_u32(DSP_REG15, &mut ph2_rms).await?
        .read_u32(PH1_REG1, &mut ph1.energy_active).await?
        .read_i32(PH1_REG5, &mut ph1.power_active).await?
        .read_i32(PH1_REG7, &mut ph1.power_reactive).await?
        .read_u32(PH2_REG1, &mut ph2.energy_active).await?
        .read_i32(PH2_REG5, &mut ph2.power_active).await?
        .read_i32(PH2_REG7, &mut ph2.power_reactive).await?
        .end().await?;

    ph1.voltage_rms = ph1_rms & ((1 << 15) - 1);
    ph2.voltage_rms = ph2_rms & ((1 << 15) - 1);

    ph1.current_rms = (ph1_rms >> 15) & ((1 << 17) - 1);
    ph2.current_rms = (ph2_rms >> 15) & ((1 << 17) - 1);

    Ok(())
}
