pub mod dhcp;

use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{Config, ConfigV4, Ipv4Address, Ipv4Cidr, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer};
use esp_hal::peripherals::WIFI;
use esp_println::println;
use esp_wifi::{
    wifi::{
        AccessPointConfiguration, ClientConfiguration, Configuration, WifiApDevice, WifiController,
        WifiDevice, WifiEvent, WifiStaDevice,
    },
    EspWifiInitialization,
};
use heapless::Vec;
use static_cell::make_static;

use crate::config::{NetworkConfiguration, CONFIG_WIFI};

pub type StackSta = Stack<WifiDevice<'static, WifiStaDevice>>;
pub type StackAp = Stack<WifiDevice<'static, WifiApDevice>>;

pub const DEVICE_IP_AP: Ipv4Address = Ipv4Address::new(192, 168, 4, 1);

pub fn init_wifi(
    spawner: &Spawner,
    wifi: WIFI,
    wifi_init: EspWifiInitialization,
    enable_ap: bool,
) -> (&'static StackSta, Option<&'static StackAp>) {
    let (controller, interface_sta, stack_ap) = if enable_ap {
        let (interface_ap, interface_sta, mut controller) =
            esp_wifi::wifi::new_ap_sta(&wifi_init, wifi).unwrap();

        let config_ap = Config::ipv4_static(StaticConfigV4 {
            address: Ipv4Cidr::new(DEVICE_IP_AP, 24),
            gateway: None,
            dns_servers: Default::default(),
        });

        let stack_ap = &*make_static!(Stack::new(
            interface_ap,
            config_ap,
            make_static!(StackResources::<5>::new()),
            0x12345678,
        ));

        spawner.must_spawn(net_task_ap(&stack_ap));
        spawner.must_spawn(dhcp::run_dhcp_server(&stack_ap));

        // set up AP
        let config_ap = Configuration::AccessPoint(AccessPointConfiguration {
            ssid: "ESP Energy Monitor".try_into().unwrap(),
            ..Default::default()
        });
        controller.set_configuration(&config_ap).unwrap();

        (controller, interface_sta, Some(stack_ap))
    } else {
        let (interface_sta, controller) =
            esp_wifi::wifi::new_with_mode(&wifi_init, wifi, WifiStaDevice).unwrap();

        (controller, interface_sta, None)
    };

    let config_sta = Config::dhcpv4(Default::default());

    let stack_sta = &*make_static!(Stack::new(
        interface_sta,
        config_sta,
        make_static!(StackResources::<5>::new()),
        0x12345678,
    ));

    spawner.must_spawn(run_wifi(controller, stack_sta));
    spawner.must_spawn(net_task_sta(&stack_sta));

    (stack_sta, stack_ap)
}

#[embassy_executor::task]
async fn run_wifi(mut controller: WifiController<'static>, stack_sta: &'static StackSta) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());

    let mut config = CONFIG_WIFI.wait().await;

    loop {
        // TODO: update stack config
        let net_config = match config.config {
            NetworkConfiguration::Dhcp => ConfigV4::Dhcp(Default::default()),
            NetworkConfiguration::Static {
                address,
                prefix_len,
                gateway,
                dns_servers,
            } => {
                let mut servers = Vec::new();
                for s in dns_servers {
                    let _ = servers.push(Ipv4Address(s));
                }
                ConfigV4::Static(StaticConfigV4 {
                    address: Ipv4Cidr::new(Ipv4Address(address), prefix_len),
                    gateway: gateway.map(|g| Ipv4Address(g)),
                    dns_servers: servers,
                })
            }
        };
        stack_sta.set_config_v4(net_config);

        // update wifi STA config
        let client_config = Configuration::Client(ClientConfiguration {
            ssid: config.wifi_ssid.clone(),
            password: config.wifi_key.clone(),
            ..Default::default()
        });

        if let Err(e) = controller.set_configuration(&client_config) {
            println!("wifi invalid config: {e:?}");
            config = CONFIG_WIFI.wait().await;
            continue;
        }

        // start wifi
        controller.start().await.unwrap();

        // keep connecting to the same wifi util we have a new config
        loop {
            match controller.connect().await {
                Ok(_) => {
                    println!("wifi connected!");

                    // wait for disconnect or new config
                    let fut_disc = controller.wait_for_event(WifiEvent::StaDisconnected);
                    let fut_config = CONFIG_WIFI.wait();

                    match select(fut_disc, fut_config).await {
                        Either::First(_) => {
                            println!("wifi disconnected!");
                        }
                        Either::Second(new_config) => {
                            config = new_config;
                            // give server time to respond to HTTP post
                            Timer::after_millis(500).await;
                            break;
                        }
                    }
                }
                Err(e) => {
                    println!("failed to connect to wifi: {e:?}");
                }
            }
            // wait a bit after disconnect or connection failure
            Timer::after(Duration::from_secs(5)).await;
        }

        // stop wifi for reconfiguration
        controller.stop().await.unwrap();
    }
}

#[embassy_executor::task]
async fn net_task_sta(stack: &'static StackSta) {
    stack.run().await
}

#[embassy_executor::task]
async fn net_task_ap(stack: &'static StackAp) {
    stack.run().await
}
