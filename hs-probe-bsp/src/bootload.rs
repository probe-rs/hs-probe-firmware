// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

#![allow(clippy::zero_ptr, clippy::unreadable_literal)]

use stm32ral::{write_reg, modify_reg, syscfg, scb};

static mut FLAG: u32 = 0;
const FLAG_VALUE: u32 = 0xB00110AD;

/// Call this function at boot in pre_init, before statics are initialised.
///
/// If we reset due to requesting a bootload, this function will jump to
/// the system bootloader.
pub fn check() {
    unsafe {
        // If flag isn't set we just continue with the boot process
        if core::ptr::read_volatile(&FLAG) != FLAG_VALUE {
            return;
        }

        // Otherwise, clear the flag and jump to system bootloader
        core::ptr::write_volatile(&mut FLAG, 0);

        // Get new stack pointer and jump address
        let addr = 0x0010_0000;
        let sp = core::ptr::read_volatile(addr as *const u32);
        let rv = core::ptr::read_volatile((addr+4) as *const u32);
        let bootloader: extern "C" fn() -> ! = core::mem::transmute(rv);

        // Write new stack pointer to MSP and call into system memory
        cortex_m::register::msp::write(sp);
        bootloader();
    }
}

/// Call this function to trigger a reset into the system bootloader
pub fn bootload() -> ! {
    unsafe {
        // Write flag value to FLAG
        core::ptr::write_volatile(&mut FLAG, FLAG_VALUE);

        // Request system reset
        modify_reg!(scb, SCB, AIRCR, VECTKEYSTAT: 0x05FA, SYSRESETREQ: 1);
    }

    // Wait for reset
    loop {
        cortex_m::asm::nop();
    }
}
