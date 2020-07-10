#![no_std]
#![no_main]

use panic_rtt_target as _;
use cortex_m_rt::entry;
use rtt_target::{rtt_init_print, rprintln};
use hs_probe_bsp::rcc::{RCC, CoreFrequency};


#[entry]
fn main() -> ! {
    rtt_init_print!();

    let rcc = RCC::new(stm32ral::rcc::RCC::take().unwrap());
    unsafe {
        rcc.setup(CoreFrequency::F72MHz);
    }

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
