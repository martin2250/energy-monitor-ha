#[derive(Copy, Clone, Debug)]
#[allow(non_camel_case_types, unused)]
pub enum Reg {
    /// DSP control register 1
    DSP_CR1 = 0,
    /// DSP control register 2
    DSP_CR2 = 1,
    /// DSP control register 3
    DSP_CR3 = 2,
    /// DSP control register 4
    DSP_CR4 = 3,
    /// DSP control register 5
    DSP_CR5 = 4,
    /// DSP control register 6
    DSP_CR6 = 5,
    /// DSP control register 7
    DSP_CR7 = 6,
    /// DSP control register 8
    DSP_CR8 = 7,
    /// DSP control register 9
    DSP_CR9 = 8,
    /// DSP control register 10
    DSP_CR10 = 9,
    /// DSP control register 11
    DSP_CR11 = 10,
    /// DSP control register 12
    DSP_CR12 = 11,
    /// digital front end control register 1
    DFE_CR1 = 12,
    /// digital front end control register 2
    DFE_CR2 = 13,
    /// DSP interrupt control mask register 1
    DSP_IRQ1 = 14,
    /// DSP interrupt control mask register 2
    DSP_IRQ2 = 15,
    /// DSP status register 1
    DSP_SR1 = 16,
    /// DSP status register 2
    DSP_SR2 = 17,
    /// UART/SPI control register 1
    US_REG1 = 18,
    /// UART/SPI control register 2
    US_REG2 = 19,
    /// UART/SPI control register 3
    US_REG3 = 20,
    /// DSP live event 1
    DSP_EV1 = 21,
    /// DSP live event 2
    DSP_EV2 = 22,

    // DSP PH1, PH2 Period
    DSP_REG1 = 23,
    // V1 Data
    DSP_REG2 = 24,
    // C1 Data
    DSP_REG3 = 25,
    // V2 Data
    DSP_REG4 = 26,
    // C2 Data
    DSP_REG5 = 27,
    // V1 Fundamental
    DSP_REG6 = 28,
    // C1 Fundamental
    DSP_REG7 = 29,
    // V2 Fundamental
    DSP_REG8 = 30,
    // C2 Fundamental
    DSP_REG9 = 31,
    // C1, V1 RMS Data
    DSP_REG14 = 36,
    // C2, V2 RMS Data
    DSP_REG15 = 37,
    // V1 Sag Time, Swell Time
    DSP_REG16 = 38,
    // C1 Phase Angle, Swell Time
    DSP_REG17 = 39,
    // V2 Sag Time, Swell Time
    DSP_REG18 = 40,
    // C2 Phase Angle, Swell Time
    DSP_REG19 = 41,

    // PH1 Active Energy
    PH1_REG1 = 42,
    // PH1 Fundamental Energy
    PH1_REG2 = 43,
    // PH1 Reactive Energy
    PH1_REG3 = 44,
    // PH1 Apparent Energy
    PH1_REG4 = 45,
    // PH1 Active Power
    PH1_REG5 = 46,
    // PH1 Fundamental Power
    PH1_REG6 = 47,
    // PH1 Reactive Power
    PH1_REG7 = 48,
    // PH1 Apparent RMS Power
    PH1_REG8 = 49,
    // PH1 Apparent Vectorial Power
    PH1_REG9 = 50,
    // PH1 Momentary Active Power
    PH1_REG10 = 51,
    // PH1 Momentary Fundamental Power
    PH1_REG11 = 52,
    // PH1 Ampere Hours Accumulated
    PH1_REG12 = 53,

    // PH2 Active Energy
    PH2_REG1 = 54,
    // PH2 Fundamental Energy
    PH2_REG2 = 55,
    // PH2 Reactive Energy
    PH2_REG3 = 56,
    // PH2 Apparent Energy
    PH2_REG4 = 57,
    // PH2 Active Power
    PH2_REG5 = 58,
    // PH2 Fundamental Power
    PH2_REG6 = 59,
    // PH2 Reactive Power
    PH2_REG7 = 60,
    // PH2 Apparent RMS Power
    PH2_REG8 = 61,
    // PH2 Apparent Vectorial Power
    PH2_REG9 = 62,
    // PH2 Momentary Active Power
    PH2_REG10 = 63,
    // PH2 Momentary Fundamental Power
    PH2_REG11 = 64,
    // PH2 Ampere Hours Accumulated
    PH2_REG12 = 65,

    // Total Active Energy
    TOT_REG1 = 66,
    // Total Fundamental Energy
    TOT_REG2 = 67,
    // Total Reactive Energy
    TOT_REG3 = 68,
    // Total Apparent Energy
    TOT_REG4 = 69,
}

impl Reg {
    pub fn addr(&self) -> u8 {
        2 * (*self as u8)
    }
}
