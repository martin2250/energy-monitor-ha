use super::{chip::StpmCurrentGain, RawSampleApp};

// these are settings of the chip that we don't change
const VOLTAGE_REFERENCE: f32 = 1.18;
const VOLTAGE_CALIBRATION: u16 = 0x800;
const CURRENT_CALIBRATION: u16 = 0x800;

pub const FIXED_DECIMALS: u32 = 15;

#[derive(Clone, Copy, Debug)]
pub struct ConversionParameters {
    // /// ADC reference voltage, 1.18 V by default
    // pub voltage_reference: f32,
    // /// analog gain factor selected in register DFE_CRx
    // pub current_gain: StpmCurrentGain,

    // /// 12 bit calibration factor corresponding to factors 0.75 to 1.0
    // pub voltage_calibration: u16,
    // /// 12 bit calibration factor corresponding to factors 0.75 to 1.0
    // pub current_calibration: u16,
    /// resistive divider factor for voltage measurement (1 + R2 / R1)
    pub voltage_divider_factor: f32,
    /// current measuring shunt resistor (in ohms)
    pub current_shunt: f32,
    /// oscillator calibration factor (1.0 = 16MHz)
    pub oscillator_factor: f32,
}

impl Default for ConversionParameters {
    fn default() -> Self {
        Self {
            // voltage_reference: 1.18,
            // current_gain: StpmCurrentGain::X2,
            // voltage_calibration: 0x800,
            // current_calibration: 0x800,
            voltage_divider_factor: 1700.0,
            current_shunt: 0.005,
            oscillator_factor: 1.0,
        }
    }
}

impl ConversionParameters {
    pub fn to_float_cal(&self) -> FloatCalibration {
        let cal_voltage = 0.75 + VOLTAGE_CALIBRATION as f32 * (0.25 / 0x1000 as f32);
        let cal_current = 0.75 + CURRENT_CALIBRATION as f32 * (0.25 / 0x1000 as f32);

        let gain_voltage = 2.0;
        let gain_current = 1.0;
        // let gain_current = match self.current_gain {
        //     StpmCurrentGain::X2 => 2.0,
        //     StpmCurrentGain::X4 => 4.0,
        //     StpmCurrentGain::X8 => 8.0,
        //     StpmCurrentGain::X16 => 16.0,
        // };

        // decimation clock in Hz
        let dclk = 7812.5 * self.oscillator_factor;
        // integration factor, not used
        let kint = 1.0;

        FloatCalibration {
            voltage_rms_lsb: VOLTAGE_REFERENCE * self.voltage_divider_factor
                / (cal_voltage * 2.0 * (1 << (15 - FIXED_DECIMALS)) as f32),
            current_rms_lsb: VOLTAGE_REFERENCE
                / (self.current_shunt * cal_current * gain_current * (1 << (17 - FIXED_DECIMALS)) as f32),
            power_lsb: VOLTAGE_REFERENCE * VOLTAGE_REFERENCE * self.voltage_divider_factor
                / (kint
                    * gain_voltage
                    * gain_current
                    * self.current_shunt
                    * cal_voltage
                    * cal_current
                    * (1 << (28 - FIXED_DECIMALS)) as f32),
            energy_lsb: VOLTAGE_REFERENCE * VOLTAGE_REFERENCE * self.voltage_divider_factor
                / (dclk
                    * kint
                    * gain_voltage
                    * gain_current
                    * self.current_shunt
                    * cal_voltage
                    * cal_current
                    * (1 << (17 - FIXED_DECIMALS)) as f32),
        }
    }
}

// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct FloatCalibration {
    /// in volts
    pub voltage_rms_lsb: f32,
    /// in amperes
    pub current_rms_lsb: f32,
    /// in watts
    pub power_lsb: f32,
    /// in watt seconds
    pub energy_lsb: f32,
}

impl FloatCalibration {
    pub fn to_int_cal(&self) -> IntCalibration {
        IntCalibration {
            voltage_rms_lsb: (1e3 * self.voltage_rms_lsb) as u32,
            current_rms_lsb: (1e4 * self.current_rms_lsb) as u32,
            power_lsb: (1e3 * self.power_lsb) as i32,
            energy_lsb: (1e3 * self.energy_lsb) as i64,
        }
    }
}

// impl FloatCalibration {
//     pub fn apply(&self, sample: RawSampleChip, num_samples: usize) -> FloatCalibratedSample {
//         let factor = 1.0 / num_samples as f32;
//         FloatCalibratedSample {
//             voltage_rms: sample.voltage_rms as f32 * self.voltage_rms_lsb * factor,
//             current_rms: sample.current_rms as f32 * self.current_rms_lsb * factor,
//             power_active: sample.power_active as f32 * self.power_lsb * factor,
//             power_reactive: sample.power_reactive as f32 * self.power_lsb * factor,
//             energy_active: sample.energy_active as f32 * self.energy_lsb,
//         }
//     }
// }

// #[derive(Clone, Copy, Debug)]
// pub struct FloatCalibratedSample {
//     pub voltage_rms: f32,
//     pub current_rms: f32,
//     pub power_active: f32,
//     pub power_reactive: f32,
//     pub energy_active: f32,
// }

// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct IntCalibration {
    /// in volts
    pub voltage_rms_lsb: u32,
    /// in amperes
    pub current_rms_lsb: u32,
    /// in watts
    pub power_lsb: i32,
    /// in watt seconds
    pub energy_lsb: i64,
}

impl IntCalibration {
    pub fn apply(&self, sample: RawSampleApp) -> IntCalibratedSample {
        let current_gain = match sample.current_gain {
            StpmCurrentGain::X2 => 2,
            StpmCurrentGain::X4 => 4,
            StpmCurrentGain::X8 => 8,
            StpmCurrentGain::X16 => 16,
        };
        let divisor = sample.num_samples as u64 * current_gain;
        IntCalibratedSample {
            // voltage: ignore current gain
            voltage_rms: (sample.voltage_rms * self.voltage_rms_lsb as u64) / (sample.num_samples as u64) / (1 << FIXED_DECIMALS),
            // current and power: use num_samples and current gain
            current_rms: (sample.current_rms * self.current_rms_lsb as u64) / divisor / (1 << FIXED_DECIMALS),
            power_active: (sample.power_active * self.power_lsb as i64) / divisor as i64 / (1 << FIXED_DECIMALS),
            power_reactive: (sample.power_reactive * self.power_lsb as i64) / divisor as i64 / (1 << FIXED_DECIMALS),
            // energy: ignore num_samples
            energy_active: (sample.energy_active * self.energy_lsb) / current_gain as i64 / (1 << FIXED_DECIMALS),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct IntCalibratedSample {
    pub voltage_rms: u64, // 3 decimal places
    pub current_rms: u64, // 4 decimal places
    pub power_active: i64, // 3 decimal places
    pub power_reactive: i64, // 3 decimal places
    pub energy_active: i64, // 3 decimal places
}
