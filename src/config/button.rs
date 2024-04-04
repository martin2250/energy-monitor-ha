use core::sync::atomic::{AtomicBool, Ordering};

use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Instant, Timer};
use embedded_hal_async::digital::Wait;
use esp_hal::{
    gpio::{GpioPin, Input, PullUp},
    macros::ram,
    reset::software_reset,
};

// this value is kept between reboots
#[ram(rtc_fast, uninitialized)]
static mut RTC_DATA: u32 = 0;

// if RTC_DATA matches this, enable config mode
const FLAG_VALUE: u32 = 0x2f1f5511;

pub static CONFIG_SERVER_ENABLE: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static CONFIG_SERVER_ENABLE_A: AtomicBool = AtomicBool::new(false);

#[embassy_executor::task]
pub async fn run_button(mut pin: GpioPin<Input<PullUp>, 22>) -> ! {
    let mut server_enable = is_ap_enabled();
    CONFIG_SERVER_ENABLE.signal(server_enable);
    CONFIG_SERVER_ENABLE_A.store(server_enable, Ordering::SeqCst);

    // at boot, wait for button to be released initially
    let _ = pin.wait_for_high().await;
    Timer::after_secs(3).await;

    // wait for button to be pressed for X seconds
    loop {
        let _ = pin.wait_for_low().await;

        // wait for 100ms button press -> toggle server
        // wait for 1s button press -> toggle access point
        match select(Timer::after_millis(100), pin.wait_for_high()).await {
            Either::First(_) => (),        // register press
            Either::Second(_) => continue, // ignore
        }

        // if AP is enabled, disable AP immediately
        if is_ap_enabled() {
            break;
        }

        // wait for full 1 second press
        match select(Timer::after_millis(900), pin.wait_for_high()).await {
            Either::First(_) => break, // reboot device
            Either::Second(_) => {
                // toggle config server
                server_enable = !server_enable;
                CONFIG_SERVER_ENABLE.signal(server_enable);
                CONFIG_SERVER_ENABLE_A.store(server_enable, Ordering::SeqCst);
                continue;
            }
        }
    }

    // toggle config mode
    set_ap(!is_ap_enabled());

    // reset
    software_reset();
    loop {
        Timer::at(Instant::MAX).await
    }
}

pub fn is_ap_enabled() -> bool {
    unsafe { core::ptr::read_volatile(&RTC_DATA) == FLAG_VALUE }
}

pub fn set_ap(enable: bool) {
    unsafe {
        if enable {
            core::ptr::write_volatile(&mut RTC_DATA, FLAG_VALUE);
        } else {
            core::ptr::write_volatile(&mut RTC_DATA, 0);
        }
    }
}
