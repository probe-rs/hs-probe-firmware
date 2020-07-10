//! USB OTG full-speed peripheral

use stm32ral::modify_reg;
use stm32ral::{otg_fs_global, otg_fs_device, otg_fs_pwrclk, rcc};
use crate::gpio::GPIO;
use crate::rcc::Clocks;

use synopsys_usb_otg::UsbPeripheral;
pub use synopsys_usb_otg::UsbBus;

pub struct USB {
    pub usb_global: otg_fs_global::Instance,
    pub usb_device: otg_fs_device::Instance,
    pub usb_pwrclk: otg_fs_pwrclk::Instance,
    pub hclk: u32,
}

impl USB {
    pub fn setup(gpioa: &GPIO, clocks: &Clocks) -> USB {
        let usb_global = stm32ral::otg_fs_global::OTG_FS_GLOBAL::take().unwrap();
        let usb_device = stm32ral::otg_fs_device::OTG_FS_DEVICE::take().unwrap();
        let usb_pwrclk = stm32ral::otg_fs_pwrclk::OTG_FS_PWRCLK::take().unwrap();

        // USB D-
        gpioa.set_mode_alternate(11);
        gpioa.set_af(11, 10);

        // USB D+
        gpioa.set_mode_alternate(12);
        gpioa.set_af(12, 10);

        USB {
            usb_global,
            usb_device,
            usb_pwrclk,
            hclk: clocks.hclk()
        }
    }
}

// We only store peripheral instances to enforce ownership,
// so it's safe to share the USB object
unsafe impl Send for USB {}
unsafe impl Sync for USB {}

unsafe impl UsbPeripheral for USB {
    const REGISTERS: *const () = otg_fs_global::OTG_FS_GLOBAL as *const ();

    const HIGH_SPEED: bool = false;
    const FIFO_DEPTH_WORDS: usize = 320;
    const ENDPOINT_COUNT: usize = 6;

    fn enable() {
        cortex_m::interrupt::free(|_| {
            let rcc = unsafe { &*rcc::RCC };

            // Enable USB peripheral
            modify_reg!(rcc, rcc, AHB2ENR, OTGFSEN: Enabled);

            // Reset USB peripheral
            modify_reg!(rcc, rcc, AHB2RSTR, OTGFSRST: Reset);
            modify_reg!(rcc, rcc, AHB2RSTR, OTGFSRST: 0);
        });
    }

    fn ahb_frequency_hz(&self) -> u32 {
        self.hclk
    }
}

pub type UsbBusType = UsbBus<USB>;
