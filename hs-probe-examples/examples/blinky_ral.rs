#![no_std]
#![no_main]

use panic_rtt_target as _;
use cortex_m_rt::entry;
use rtt_target::{rtt_init_print, rprintln};
use hs_probe_bsp::rcc::{RCC, CoreFrequency};
use hs_probe_bsp::gpio::GPIO;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let rcc = RCC::new(stm32ral::rcc::RCC::take().unwrap());
    rcc.setup(CoreFrequency::F48MHz);

    let gpioc = GPIO::new(stm32ral::gpio::GPIOC::take().unwrap());
    let led = gpioc.pin(10);
    // Open-drain output to LED (active low).
    led
        .set_high()
        .set_otype_opendrain()
        .set_ospeed_low()
        .set_mode_output();

    rprintln!("Starting blinky...");

    loop {
        cortex_m::asm::delay(16_000_000);
        led.set_low();

        cortex_m::asm::delay(16_000_000);
        led.set_high();
    }
}
