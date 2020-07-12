#![no_std]
#![no_main]

mod app;
mod usb;

use panic_rtt_target as _;
use cortex_m_rt::entry;
use rtt_target::{rtt_init_print, rprintln};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    rprintln!("Starting...");

    loop {
        continue;
    }
}
