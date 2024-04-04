use core::sync::atomic::Ordering;
use embassy_time::Timer;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};
use esp_hal::gpio::{GpioPin, Output, PushPull};
use crate::{config::{is_ap_enabled, CONFIG_SERVER_ENABLE_A}, mqtt::MQTT_CONNECTED, wifi::StackSta};


#[embassy_executor::task]
pub async fn run_led_green(stack_sta: &'static StackSta, mut led_green: GpioPin<Output<PushPull>, 23>) {
    loop {
        Timer::after_millis(500).await;
        
        if stack_sta.is_link_up() {
            let mqtt = MQTT_CONNECTED.load(Ordering::SeqCst);

            if mqtt {
                let _  = led_green.set_high();
            } else {
                let _  = led_green.toggle();
            }
        } else {
            let _  = led_green.set_low();
        }
    }
}

#[embassy_executor::task]
pub async fn run_led_red(mut led_red: GpioPin<Output<PushPull>, 13>) {
    // AP can only be disabled by rebooting, stop task in that case
    if is_ap_enabled() {
        let _ = led_red.set_high();
        return;
    }
    // check if config server is enabled
    loop {
        let enabled = CONFIG_SERVER_ENABLE_A.load(Ordering::SeqCst);
        if enabled {
            let _ = led_red.toggle();
        } else {
            let _ = led_red.set_low();
        }
        Timer::after_millis(200).await;
    }
}
