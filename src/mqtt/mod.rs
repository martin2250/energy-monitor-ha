mod sensor;

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_futures::select::{select3, Either3};
use embassy_net::{dns::DnsQueryType, tcp::TcpSocket, Ipv4Address};
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};
use esp_println::println;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use heapless::String;
use rand_core::RngCore;
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::{publish_packet::QualityOfService::QoS0, reason_codes::ReasonCode},
    utils::rng_generator::CountingRng,
};
use serde::Serialize;

use crate::{
    config::{CalibrationConfig, MqttConfig, CONFIG_CALIBRATION, CONFIG_MQTT},
    stpm::{
        calibration::{ConversionParameters, IntCalibration},
        SAMPLES,
    },
};

use self::sensor::{Device, Sensor, SensorDeviceClass};

type Stack = embassy_net::Stack<WifiDevice<'static, WifiStaDevice>>;

pub static MQTT_CONNECTED: AtomicBool = AtomicBool::new(false);

const TCP_BUFFER_LEN: usize = 4096;
const MQTT_BUFFER_LEN: usize = 1024;

#[embassy_executor::task]
pub async fn run_mqtt(stack: &'static Stack) {
    let mut tcp_buf_rx = [0; TCP_BUFFER_LEN];
    let mut tcp_buf_tx = [0; TCP_BUFFER_LEN];
    let mut mqtt_buf_rx = [0; MQTT_BUFFER_LEN];
    let mut mqtt_buf_tx = [0; MQTT_BUFFER_LEN];
    let mut scratch_buffer = [0; MQTT_BUFFER_LEN];

    let mut config = CONFIG_MQTT.wait().await;
    let mut cal = to_mqtt_cal(&CONFIG_CALIBRATION.wait().await);

    loop {
        let res = once_mqtt(
            stack,
            &mut config,
            &mut cal,
            &mut tcp_buf_rx,
            &mut tcp_buf_tx,
            &mut mqtt_buf_rx,
            &mut mqtt_buf_tx,
            &mut scratch_buffer,
        )
        .await;

        MQTT_CONNECTED.store(false, Ordering::SeqCst);

        if res == None {
            // error: retry after x seconds
            Timer::after_secs(3).await;
        }
    }
}

async fn once_mqtt(
    stack: &'static Stack,
    config: &mut MqttConfig,
    cal: &mut [IntCalibration; 2],
    tcp_buf_rx: &mut [u8],
    tcp_buf_tx: &mut [u8],
    mqtt_buf_rx: &mut [u8],
    mqtt_buf_tx: &mut [u8],
    buffer: &mut [u8],
) -> Option<()> {
    // parse address string to IPv4 address
    let address = match core::net::Ipv4Addr::parse_ascii(config.broker_address.as_bytes()) {
        Ok(a) => Ipv4Address::from_bytes(&a.octets()).into(),
        Err(_) => {
            match stack
                .dns_query(&config.broker_address, DnsQueryType::A)
                .await
            {
                Ok(a) => a[0],
                Err(e) => {
                    println!("MQTT DNS query failed {e:?}");
                    return None;
                }
            }
        }
    };

    // create socket and connect
    let mut socket = TcpSocket::new(&stack, tcp_buf_rx, tcp_buf_tx);
    socket.set_timeout(Some(Duration::from_secs(60)));

    if let Err(e) = socket.connect((address, config.broker_port)).await {
        println!("mqtt tcp connect error: {:?}", e);
        return None;
    }

    println!("mqtt tcp connected!");

    // set up mqtt
    let mut mqtt_config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );

    mqtt_config.add_max_subscribe_qos(QoS0);

    if config.mqtt_client_id.len() > 0 {
        mqtt_config.add_client_id(&config.mqtt_client_id);
    }

    if config.mqtt_username.len() > 0 {
        mqtt_config.add_username(&config.mqtt_username);
    }
    
    if config.mqtt_password.len() > 0 {
        mqtt_config.add_password(&config.mqtt_password);
    }

    // not sure if this actually matters
    mqtt_config.max_packet_size = MQTT_BUFFER_LEN as u32 + 20;

    let mut client = MqttClient::<_, 5, _>::new(
        socket,
        mqtt_buf_tx,
        MQTT_BUFFER_LEN,
        mqtt_buf_rx,
        MQTT_BUFFER_LEN,
        mqtt_config,
    );

    match client.connect_to_broker().await {
        Ok(()) => {}
        Err(e) => match e {
            ReasonCode::NetworkError => {
                println!("MQTT Network Error");
                return None;
            }
            _ => {
                println!("Other MQTT Error: {:?}", e);
                return None;
            }
        },
    }

    MQTT_CONNECTED.store(true, Ordering::SeqCst);

    // subscribe to home assistant status (birth and will)
    let mut topic: String<128> = String::new();
    let _ = topic.push_str(&config.ha_discovery_prefix);
    let _ = topic.push_str("/status");
    client.subscribe_to_topic(&topic).await.ok()?;

    // set topic to state publish topic
    topic.clear();
    let _ = topic.push_str(&config.ha_discovery_prefix);
    let _ = topic.push_str("/sensor/");
    let _ = topic.push_str(&config.ha_unique_id);
    let _ = topic.push_str("/state");

    // publish configurations at the very start
    let mut publish_config = true;
    // publish new samples as they arrive
    loop {
        if publish_config {
            publish_configurations(&mut client, &config, &topic, buffer).await?;
            println!("mqtt publish config");
            publish_config = false;
        }

        let fut_mqtt = client.receive_message();
        let fut_samples = SAMPLES.wait();
        let fut_config = CONFIG_MQTT.wait();

        match select3(fut_mqtt, fut_samples, fut_config).await {
            Either3::First(Ok((topic, msg))) => {
                println!("mqtt rx: {topic:?}");
                // don't check the topic, we only subscribed to home assistant status
                if msg == b"online" {
                    publish_config = true;
                }
            }
            Either3::First(Err(e)) => {
                println!("error receiving mqtt message: {e:?}");
                return None;
            }
            Either3::Second(samples) => {
                // update calibration?
                if CONFIG_CALIBRATION.signaled() {
                    *cal = to_mqtt_cal(&CONFIG_CALIBRATION.wait().await);
                }

                // apply cal
                let samples: [_; 2] = core::array::from_fn(|i| cal[i].apply(samples[i]));

                let mut ms: MqttSample = Default::default();

                // set only the values that are configured
                if config.channel_enable[0].frequency || config.channel_enable[1].frequency {
                    ms.frequency = Some(super::zcr::get_frequency());
                }
                if config.channel_enable[0].voltage {
                    ms.ch1_voltage_rms = Some(samples[0].voltage_rms);
                }
                if config.channel_enable[0].current {
                    ms.ch1_current_rms = Some(samples[0].current_rms);
                }
                if config.channel_enable[0].active_power {
                    ms.ch1_power_active = Some(samples[0].power_active);
                }
                if config.channel_enable[0].reactive_power {
                    ms.ch1_power_reactive = Some(samples[0].power_reactive);
                }
                if config.channel_enable[0].energy {
                    ms.ch1_energy_active = Some(samples[0].energy_active);
                }
                if config.channel_enable[1].voltage {
                    ms.ch2_voltage_rms = Some(samples[1].voltage_rms);
                }
                if config.channel_enable[1].current {
                    ms.ch2_current_rms = Some(samples[1].current_rms);
                }
                if config.channel_enable[1].active_power {
                    ms.ch2_power_active = Some(samples[1].power_active);
                }
                if config.channel_enable[1].reactive_power {
                    ms.ch2_power_reactive = Some(samples[1].power_reactive);
                }
                if config.channel_enable[1].energy {
                    ms.ch2_energy_active = Some(samples[1].energy_active);
                }

                // send sample
                let n = serde_json_core::to_slice(&ms, buffer).unwrap();

                if let Err(e) = client
                    .send_message(topic.as_str(), &buffer[..n], QoS0, false)
                    .await
                {
                    println!("mqtt publish failed {e:?}");
                    return None;
                }
            }
            Either3::Third(new_config) => {
                // client holds references to config
                core::mem::drop(client);
                *config = new_config;
                return Some(());
            }
        };
    }
}

async fn publish_configurations<T: Read + Write, const MAX_PROPERTIES: usize, R: RngCore>(
    client: &mut MqttClient<'_, T, MAX_PROPERTIES, R>,
    config: &MqttConfig,
    state_topic: &str,
    buffer: &mut [u8],
) -> Option<()> {
    // entities to send
    let enable = &config.channel_enable;
    use SensorDeviceClass::{Current, Energy, Frequency, Power, ReactivePower, Voltage};

    #[rustfmt::skip]
    let entities = [
        // ch 1
        (0, "freq", "/1e4", "Hz", Frequency, "Frequency", enable[0].frequency),
        (0, "volt1", "/1e3", "V", Voltage, "Voltage", enable[0].voltage),
        (0, "curr1", "/1e4", "A", Current, "Current", enable[0].current),
        (0, "powa1", "/1e3", "W", Power, "Power", enable[0].active_power),
        (0, "powr1", "/1e3", "var", ReactivePower, "ReactivePower", enable[0].reactive_power),
        (0, "engy1", "/10", "Ws", Energy, "Energy", enable[0].energy),
        // ch 2
        (1, "freq", "/1e4", "Hz", Frequency, "Frequency", enable[1].frequency),
        (1, "volt2", "/1e3", "V", Voltage, "Voltage", enable[1].voltage),
        (1, "curr2", "/1e4", "A", Current, "Current", enable[1].current),
        (1, "powa2", "/1e3", "W", Power, "Power", enable[1].active_power),
        (1, "powr2", "/1e3", "var", ReactivePower, "ReactivePower", enable[1].reactive_power),
        (1, "engy2", "/10", "Ws", Energy, "Energy", enable[1].energy),
    ];

    // entity template
    let mut sensor = Sensor {
        state_topic,
        device: Device {
            identifiers: &config.ha_unique_id,
            name: &config.ha_device_name,
        },
        expire_after: 10,
        icon: None,
        device_class: Power,
        unit_of_measurement: "",
        suggested_display_precision: None,
        json_name: "",
        json_conv: "",
        name: String::new(),
    };

    // go through all entities
    for (i, json_name, json_conv, unit, class, name, enabled) in entities {
        if !enabled {
            continue;
        }

        // copy details
        sensor.device_class = class;
        sensor.unit_of_measurement = unit;
        sensor.json_name = json_name;
        sensor.json_conv = json_conv;

        sensor.name.clear();
        let _ = sensor.name.push_str(&config.channel_names[i]);
        let _ = sensor.name.push(' ');
        let _ = sensor.name.push_str(name);

        // serialize
        let n = serde_json_core::to_slice(&sensor, buffer).unwrap();

        // config topic
        let mut config_topic: String<128> = String::new();
        let _ = config_topic.push_str(&config.ha_discovery_prefix);
        let _ = config_topic.push_str("/sensor/");
        let _ = config_topic.push_str(&config.ha_unique_id);
        let _ = config_topic.push_str("/");
        let _ = config_topic.push_str(json_name);
        let _ = config_topic.push_str("/config");

        // publish
        client
            .send_message(&config_topic, &buffer[..n], QoS0, false)
            .await
            .ok()?;
    }

    Some(())
}

#[derive(Default, Serialize)]
struct MqttSample {
    #[serde(rename = "freq", skip_serializing_if = "Option::is_none")]
    pub frequency: Option<u64>,

    #[serde(rename = "volt1", skip_serializing_if = "Option::is_none")]
    pub ch1_voltage_rms: Option<u64>,
    #[serde(rename = "curr1", skip_serializing_if = "Option::is_none")]
    pub ch1_current_rms: Option<u64>,
    #[serde(rename = "powa1", skip_serializing_if = "Option::is_none")]
    pub ch1_power_active: Option<i64>,
    #[serde(rename = "powr1", skip_serializing_if = "Option::is_none")]
    pub ch1_power_reactive: Option<i64>,
    #[serde(rename = "engy1", skip_serializing_if = "Option::is_none")]
    pub ch1_energy_active: Option<i64>,

    #[serde(rename = "volt2", skip_serializing_if = "Option::is_none")]
    pub ch2_voltage_rms: Option<u64>,
    #[serde(rename = "curr2", skip_serializing_if = "Option::is_none")]
    pub ch2_current_rms: Option<u64>,
    #[serde(rename = "powa2", skip_serializing_if = "Option::is_none")]
    pub ch2_power_active: Option<i64>,
    #[serde(rename = "powr2", skip_serializing_if = "Option::is_none")]
    pub ch2_power_reactive: Option<i64>,
    #[serde(rename = "engy2", skip_serializing_if = "Option::is_none")]
    pub ch2_energy_active: Option<i64>,
}

fn to_mqtt_cal(cal: &CalibrationConfig) -> [IntCalibration; 2] {
    core::array::from_fn(|i| {
        let param = ConversionParameters {
            voltage_divider_factor: cal.channels[i].voltage_divider_factor,
            current_shunt: cal.channels[i].current_shunt,
            oscillator_factor: cal.frequency_stpm_adjust,
        };
        param.to_float_cal().to_int_cal()
    })
}
