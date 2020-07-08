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
        .sysclk(216.mhz())
        .freeze();

    rprintln!("sysclk: {}", clocks.sysclk().0);
    rprintln!("hclk: {}", clocks.hclk().0);

    let rcc = unsafe { &*stm32f7xx_hal::pac::RCC::ptr() };
    let cfg = rcc.pllcfgr.read();
    let pllm = cfg.pllm().bits() as u32;
    let plln = cfg.plln().bits() as u32;
    let pllq = cfg.pllq().bits() as u32;
    rprintln!("PLL settings: m={}, n={}, p={:#b}, q={}", pllm, plln, cfg.pllp().bits(), pllq);
    rprintln!("VCO: {}", 12_000_000 * plln / pllm);
    rprintln!("PLLQ: {}", 12_000_000 * plln / pllm / pllq);
    rprintln!("CFGR: {:08x}", rcc.cfgr.read().bits());

    loop {
        continue;
    }
}
