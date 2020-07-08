#![no_std]
#![no_main]

use panic_rtt_target as _;
use cortex_m_rt::entry;
use rtt_target::{rtt_init_print, rprintln};
use stm32f7xx_hal::prelude::*;
use stm32f7xx_hal::rcc::{HSEClock, HSEClockMode};
use stm32f7xx_hal::delay::Delay;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let cp = cortex_m::Peripherals::take().unwrap();
    let p = stm32f7xx_hal::pac::Peripherals::take().unwrap();

    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr
        .hse(HSEClock::new(12.mhz(), HSEClockMode::Bypass))
        .sysclk(72.mhz())
        .freeze();

    let mut delay = Delay::new(cp.SYST, clocks);

    let gpioa = p.GPIOA.split();
    let gpioc = p.GPIOC.split();
    let mut led1 = gpioc.pc10.into_open_drain_output();
    let mut led2 = gpioa.pa12.into_open_drain_output();
    let mut led3 = gpioa.pa11.into_open_drain_output();
    led1.set_high().ok();
    led2.set_high().ok();
    led3.set_high().ok();

    rprintln!("Starting blinky...");

    loop {
        led1.set_high().ok();
        led2.set_high().ok();
        led3.set_high().ok();
        delay.delay_ms(500u32);

        led1.set_low().ok();
        led2.set_high().ok();
        led3.set_high().ok();
        delay.delay_ms(500u32);

        led1.set_high().ok();
        led2.set_low().ok();
        led3.set_high().ok();
        delay.delay_ms(500u32);

        led1.set_high().ok();
        led2.set_high().ok();
        led3.set_low().ok();
        delay.delay_ms(500u32);
    }
}
