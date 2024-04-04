use serde::{Deserialize, Serialize};


#[allow(unused)]
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub enum StpmCurrentGain {
    #[default]
    X2 = 0,
    X4 = 1,
    X8 = 2,
    X16 = 3,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug, Default)]
pub enum StpmLineFrequency {
    #[default]
    F50 = 0,
    F60 = 1,
}

#[derive(Copy, Clone, Debug)]
pub struct StpmChannelConfiguration {
    pub current_gain: StpmCurrentGain,
    pub voltage_phase_comp: u8,
    pub current_phase_comp: u16,
    pub voltage_calibration: u16,
    pub voltage_swell_threshold: u16,
    pub voltage_sag_threshold: u16,
    pub current_calibration: u16,
    pub current_swell_threshold: u16,
}

impl Default for StpmChannelConfiguration {
    fn default() -> Self {
        Self {
            current_gain: Default::default(),
            voltage_phase_comp: Default::default(),
            current_phase_comp: Default::default(),
            voltage_calibration: 0x800,
            current_calibration: 0x800,
            // To disable sag detection, the SAG_THRx register must be set to zero.
            voltage_sag_threshold: 0x0,
            // To disable swell detection, the registers SWV_THRx and SWC_THRx must have maximum value 0x3FF.
            voltage_swell_threshold: 0x3ff,
            current_swell_threshold: 0x3ff,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct StpmConfiguration {
    pub line_frequency: StpmLineFrequency,
    pub channels: [StpmChannelConfiguration; 2],
}
