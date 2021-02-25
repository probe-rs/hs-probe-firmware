use crate::rcc::Clocks;
use core::sync::atomic::{AtomicU32, Ordering};
use stm32ral::syst;
use stm32ral::{modify_reg, read_reg, write_reg};

const SYST_CSR_ENABLE: u32 = 1 << 0;
const SYST_CSR_TICKINT: u32 = 1 << 1;
const SYST_CSR_CLKSOURCE: u32 = 1 << 2;
const SYST_CSR_COUNTFLAG: u32 = 1 << 16;

pub struct Delay {
    systick: syst::Instance,
    base_clock: AtomicU32,
}

impl Delay {
    pub fn new(systick: syst::Instance) -> Self {
        // Set clock source to processor clock
        modify_reg!(syst, systick, CSR, |r| (r | SYST_CSR_CLKSOURCE));

        // Set reload and current values
        write_reg!(syst, systick, RVR, 0xffffff);
        write_reg!(syst, systick, CVR, 0);

        // Enable the counter
        modify_reg!(syst, systick, CSR, |r| (r | SYST_CSR_ENABLE));

        Delay {
            systick,
            base_clock: AtomicU32::new(0),
        }
    }

    pub fn set_sysclk(&self, clocks: &Clocks) {
        self.base_clock.store(clocks.hclk(), Ordering::SeqCst);
    }

    pub fn delay_us(&self, us: u32) {
        assert!(us < 10_000);

        let base_clock = self.base_clock.load(Ordering::SeqCst);
        assert!(base_clock > 0);

        let ticks = (us as u64) * (base_clock as u64) / 1_000_000;
        self.delay_ticks(ticks as u32);
    }

    pub fn calc_period_ticks(&self, frequency: u32) -> u32 {
        let base_clock = self.base_clock.load(Ordering::SeqCst);
        assert!(base_clock > 0);

        base_clock / frequency
    }

    pub fn delay_ticks(&self, mut ticks: u32) {
        let mut last = self.get_current();
        loop {
            let now = self.get_current();
            let delta = last.wrapping_sub(now) & 0xffffff;

            if delta >= ticks {
                break;
            } else {
                ticks -= delta;
                last = now;
            }
        }
    }

    pub fn delay_ticks_from_last(&self, mut ticks: u32, mut last: u32) -> u32 {
        loop {
            let now = self.get_current();
            let delta = last.wrapping_sub(now) & 0xffffff;

            if delta >= ticks {
                break now;
            } else {
                ticks -= delta;
                last = now;
            }
        }
    }

    #[inline(always)]
    pub fn get_current(&self) -> u32 {
        read_reg!(syst, self.systick, CVR)
    }
}
