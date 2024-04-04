use esp_hal::{
    gpio::{GpioPin, Input, InputPin, InputSignal, PullUp}, interrupt::{self, Priority}, macros::interrupt, mcpwm::PwmPeripheral, peripherals::{Interrupt, MCPWM0}
};

/// microseconds between x zero crossings
pub static mut ZCR_TDIFF: u32 = 0;
/// maximum number of zero crossings to count
pub const ZCR_COUNT: usize = 64;
/// divide this by ZCR_TDIFF to get frequency with 4 decimal places
pub static mut ZCR_FREQUENCY_NUMERATOR: u64 = 0;

/// holds the timer value at the last x zero crossings
static mut ZCR_TIMES: [u32; ZCR_COUNT] = [0; ZCR_COUNT];
/// next index to be written, pub so we can change the number of ZCRs to count
/// before enabling the interrupt
pub static mut ZCR_HEAD: usize = 0;
/// next index to be read
static mut ZCR_TAIL: usize = 0;

#[interrupt]
unsafe fn MCPWM0() {
    let reg_block = MCPWM0::steal();
    let time: u32 = reg_block.cap_ch0().read().bits();
    reg_block.int_clr().write(|w|w.cap0_int_clr().set_bit());

    let time_last = *ZCR_TIMES.get_unchecked(ZCR_TAIL);
    *ZCR_TIMES.get_unchecked_mut(ZCR_HEAD) = time;

    ZCR_TAIL = (ZCR_TAIL + 1) % ZCR_COUNT;
    ZCR_HEAD = (ZCR_HEAD + 1) % ZCR_COUNT;

    let tdiff = time.wrapping_sub(time_last);
    core::ptr::write_volatile(&mut ZCR_TDIFF, tdiff);
}

/// get current line frequency with 4 decimal places
pub fn get_frequency() -> u64 {
    let zcr_frequency_numerator =
        unsafe { core::ptr::read_volatile(&ZCR_FREQUENCY_NUMERATOR) } as u64;
    // in microseconds
    let zcr_tdiff = unsafe { core::ptr::read_volatile(&ZCR_TDIFF) } as u64;
    // 4 decimal places
    zcr_frequency_numerator / zcr_tdiff
}

pub fn zcr_setup(mut zcr_input: GpioPin<Input<PullUp>, 21>, mcpwm: MCPWM0, num_samples: usize, frequency_adjust_esp: f32)
{
    zcr_input.connect_input_to_peripheral(InputSignal::PWM0_CAP0);
    // ignore destructor
    core::mem::forget(zcr_input);

    unsafe {
        // set up correct number of samples
        core::ptr::write_volatile(&mut ZCR_HEAD, num_samples % ZCR_COUNT);

        // divide by ZCR_TDIFF to get frequency with 4 decimal places
        let zcr_frequency_numerator =
            ((num_samples as f32) * 80e6 * 1e4 / frequency_adjust_esp) as u64;
        core::ptr::write_volatile(&mut ZCR_FREQUENCY_NUMERATOR, zcr_frequency_numerator);

        esp_hal::peripherals::MCPWM0::enable();

        // setup
        mcpwm.cap_ch0_cfg().write(|w| {
            // w.cap0_prescale().bits(79); // divider: 80 -> 1 MHz
            w.cap0_mode().bits(0b10); // trigger on rising edge
            w.cap0_en().set_bit()
        });
        // start timer
        mcpwm.cap_timer_cfg().write(|w| w.cap_timer_en().set_bit());
        // enable interrupt
        mcpwm.int_ena().write(|w|w.cap0_int_ena().set_bit());
        interrupt::enable(Interrupt::MCPWM0, Priority::Priority1).unwrap();
    }
}
