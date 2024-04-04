use heapless::{String, Vec};
use serde::{Deserialize, Serialize};

use crate::stpm::StpmCurrentGain;

#[derive(Serialize, Deserialize, Clone)]
pub struct MqttChannelEnables {
    pub frequency: bool,
    pub voltage: bool,
    pub current: bool,
    pub active_power: bool,
    pub reactive_power: bool,
    pub energy: bool,
}

impl Default for MqttChannelEnables {
    fn default() -> Self {
        Self {
            frequency: false,
            voltage: false,
            current: false,
            active_power: true,
            reactive_power: false,
            energy: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MqttConfig {
    // tcp
    pub broker_address: String<128>,
    pub broker_port: u16,
    // mqtt
    pub mqtt_username: String<32>,
    pub mqtt_password: String<32>,
    pub mqtt_client_id: String<32>,
    // home assistant
    pub ha_unique_id: String<32>,
    pub ha_discovery_prefix: String<32>,
    pub ha_device_name: String<32>,
    pub channel_names: [String<32>; 2],
    pub channel_enable: [MqttChannelEnables; 2],
}

impl MqttConfig {
    pub fn validate(&self) -> bool {
        // [a-zA-Z0-9_-]
        for s in [self.mqtt_client_id.as_str(), self.ha_unique_id.as_str(), self.ha_discovery_prefix.as_str()] {
            for c in s.chars() {
                match c {
                    'a'..='z' => (),
                    'A'..='Z' => (),
                    '0'..='9' => (),
                    '_' | '-' => (),
                    _ => return false,
                }
            }
        }
        if self.ha_unique_id.len() < 1 || self.ha_discovery_prefix.len() < 1 {
            return false;
        }
        true
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        // make unique ID by default
        let mut unique_id = String::new();
        let _ = unique_id.push_str("energy_monitor_");

        for digit in esp_hal::efuse::Efuse::get_mac_address() {
            const HEX: &[u8] = b"0123456789ABCDEF";
            let low = digit as usize & 0x0f;
            let high = digit as usize >> 4;

            let _ = unique_id.push(HEX[low] as char);
            let _ = unique_id.push(HEX[high] as char);
        }

        Self {
            broker_address: String::new(),
            broker_port: 1883,
            mqtt_username: String::new(),
            mqtt_password: String::new(),
            mqtt_client_id: unique_id.clone(),
            ha_unique_id: unique_id.clone(),
            ha_discovery_prefix: String::try_from("homeassistant").unwrap(),
            ha_device_name: String::try_from("Energy Monitor").unwrap(),
            channel_names: [
                String::try_from("Channel 1").unwrap(),
                String::try_from("Channel 2").unwrap(),
            ],
            channel_enable: Default::default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub enum NetworkConfiguration {
    #[default]
    Dhcp,
    Static {
        address: [u8; 4],
        prefix_len: u8,
        gateway: Option<[u8; 4]>,
        dns_servers: Vec<[u8; 4], 3>,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WifiConfig {
    pub wifi_ssid: String<32>,
    pub wifi_key: String<64>,
    pub config: NetworkConfiguration,
}

impl Default for WifiConfig {
    fn default() -> Self {
        Self {
            wifi_ssid: String::new(),
            wifi_key: String::new(),
            config: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StpmConfig {
    // how many samples to average
    pub samples_stpm: usize,
    // channel current gain
    pub current_gain: [StpmCurrentGain; 2],
}

impl Default for StpmConfig {
    fn default() -> Self {
        Self {
            samples_stpm: 20,
            current_gain: [StpmCurrentGain::X2; 2],
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CalibrationChannelConfig {
    pub voltage_divider_factor: f32,
    pub current_shunt: f32,
    pub current_gain: StpmCurrentGain,
}

impl Default for CalibrationChannelConfig {
    fn default() -> Self {
        Self {
            voltage_divider_factor: 1700.0,
            current_shunt: 0.005,
            current_gain: StpmCurrentGain::X2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CalibrationConfig {
    pub frequency_esp_adjust: f32,
    pub frequency_stpm_adjust: f32,
    pub channels: [CalibrationChannelConfig; 2],
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            frequency_esp_adjust: 1.0,
            frequency_stpm_adjust: 1.0,
            channels: Default::default(),
        }
    }
}
