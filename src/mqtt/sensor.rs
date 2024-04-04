use heapless::String;
use serde::{ser::SerializeMap, Serialize, Serializer};

pub struct Device<'a> {
    pub identifiers: &'a str,
    pub name: &'a str,
}

impl<'a> Serialize for Device<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_map(Some(2))?;
        st.serialize_entry("ids", self.identifiers)?;
        st.serialize_entry("name", self.name)?;
        st.end()
    }
}

pub struct Sensor<'a> {
    pub state_topic: &'a str,
    pub device: Device<'a>,
    pub expire_after: u32,
    pub icon: Option<&'a str>,
    pub device_class: SensorDeviceClass,
    pub unit_of_measurement: &'a str,
    pub suggested_display_precision: Option<u8>,
    pub json_name: &'a str,
    pub json_conv: &'a str,
    pub name: String<64>,
}

impl<'a> Serialize for Sensor<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut buffer: String<64> = String::new();

        let mut st = s.serialize_map(None)?;

        st.serialize_entry("stat_t", self.state_topic)?;
        st.serialize_entry("dev", &self.device)?;
        // other common stuff
        if let Some(icon) = self.icon.as_ref() {
            st.serialize_entry("ic", icon)?;
        }
        if self.device_class != SensorDeviceClass::None {
            st.serialize_entry("dev_cla", &self.device_class)?;
        }
        if self.expire_after != 0 {
            st.serialize_entry("exp_aft", &self.expire_after)?;
        }
        if let Some(prec) = self.suggested_display_precision {
            st.serialize_entry("sug_dsp_prc", &prec)?;
        }
        st.serialize_entry("unit_of_meas", self.unit_of_measurement)?;
        st.serialize_entry("name", self.name.as_str())?;

        // value template
        buffer.clear();
        buffer.push_str("{{ value_json.").unwrap();
        buffer.push_str(self.json_name).unwrap();
        buffer.push_str(self.json_conv).unwrap();
        buffer.push_str(" }}").unwrap();
        st.serialize_entry("val_tpl", buffer.as_str())?;
        
        // unique id
        buffer.clear();
        buffer.push_str(self.device.identifiers).unwrap();
        buffer.push('.').unwrap();
        buffer.push_str(self.json_name).unwrap();
        st.serialize_entry("uniq_id", buffer.as_str())?;

        st.end()
    }
}

#[allow(dead_code)]
#[derive(Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorDeviceClass {
    /// : Generic sensor. This is the default and doesn’t need to be set.
    #[default]
    None,
    /// : Apparent power in VA.
    ApparentPower,
    /// : Air Quality Index (unitless).
    Aqi,
    /// : Atmospheric pressure in cbar, bar, hPa, inHg, kPa, mbar, Pa or psi
    AtmosphericPressure,
    /// : Percentage of battery that is left in %
    Battery,
    /// : Carbon Dioxide in CO2 (Smoke) in ppm
    CarbonDioxide,
    /// : Carbon Monoxide in CO (Gas CNG/LPG) in ppm
    CarbonMonoxide,
    /// : Current in A, mA
    Current,
    /// : Data rate in bit/s, kbit/s, Mbit/s, Gbit/s, B/s, kB/s, MB/s, GB/s, KiB/s, MiB/s or GiB/s
    DataRate,
    /// : Data size in bit, kbit, Mbit, Gbit, B, kB, MB, GB, TB, PB, EB, ZB, YB, KiB, MiB, GiB, TiB, PiB, EiB, ZiB or YiB
    DataSize,
    /// : Date string (ISO 8601)
    Date,
    /// : Generic distance in km, m, cm, mm, mi, yd, or in
    Distance,
    /// : Duration in d, h, min, or s
    Duration,
    /// : Energy in Wh, kWh, MWh, MJ, or GJ
    Energy,
    /// : Stored energy in Wh, kWh, MWh, MJ, or GJ
    EnergyStorage,
    /// : Has a limited set of (non-numeric) states
    Enum,
    /// : Frequency in Hz, kHz, MHz, or GHz
    Frequency,
    /// : Gasvolume in m³, ft³ or CCF
    Gas,
    /// : Percentage of humidity in the air in %
    Humidity,
    /// : The current light level in lx
    Illuminance,
    /// : Irradiance in W/m² or BTU/(h⋅ft²)
    Irradiance,
    /// : Percentage of water in a substance in %
    Moisture,
    /// : The monetary value (ISO 4217)
    Monetary,
    /// : Concentration of Nitrogen Dioxide in µg/m³
    NitrogenDioxide,
    /// : Concentration of Nitrogen Monoxide in µg/m³
    NitrogenMonoxide,
    /// : Concentration of Nitrous Oxide in µg/m³
    NitrousOxide,
    /// : Concentration of Ozone in µg/m³
    Ozone,
    /// : Concentration of particulate matter less than 1 micrometer in µg/m³
    Pm1,
    /// : Concentration of particulate matter less than 2.5 micrometers in µg/m³
    Pm25,
    /// : Concentration of particulate matter less than 10 micrometers in µg/m³
    Pm10,
    /// : Power factor (unitless), unit may be None or %
    PowerFactor,
    /// : Power in W or kW
    Power,
    /// : Accumulated precipitation in cm, in or mm
    Precipitation,
    /// : Precipitation intensity in in/d, in/h, mm/d or mm/h
    PrecipitationIntensity,
    /// : Pressure in Pa, kPa, hPa, bar, cbar, mbar, mmHg, inHg or psi
    Pressure,
    /// : Reactive power in var
    ReactivePower,
    /// : Signal strength in dB or dBm
    SignalStrength,
    /// : Sound pressure in dB or dBA
    SoundPressure,
    /// : Generic speed in ft/s, in/d, in/h, km/h, kn, m/s, mph or mm/d
    Speed,
    /// : Concentration of sulphur dioxide in µg/m³
    SulphurDioxide,
    /// : Temperature in °C, °F or K
    Temperature,
    /// : Datetime object or timestamp string (ISO 8601)
    Timestamp,
    /// : Concentration of volatile organic compounds in µg/m³
    VolatileOrganicCompounds,
    /// : Ratio of volatile organic compounds in ppm or ppb
    VolatileOrganicCompoundsParts,
    /// : Voltage in V, mV
    Voltage,
    /// : Generic volume in L, mL, gal, fl. oz., m³, ft³, or CCF
    Volume,
    /// : Generic stored volume in L, mL, gal, fl. oz., m³, ft³, or CCF
    VolumeStorage,
    /// : Water consumption in L, gal, m³, ft³, or CCF
    Water,
    /// : Generic mass in kg, g, mg, µg, oz, lb, or st
    Weight,
    /// : Wind speed in ft/s, km/h, kn, m/s, or mph
    WindSpeed,
}
