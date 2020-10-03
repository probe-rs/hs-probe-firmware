//! USB OTG high-speed peripheral

use stm32ral::{write_reg, modify_reg, read_reg};
use stm32ral::{otg_hs_global, otg_hs_device, otg_hs_pwrclk, usbphyc, rcc};
use synopsys_usb_otg::{UsbPeripheral, PhyType};
pub use synopsys_usb_otg::UsbBus;

pub struct USB {
    pub usb_phy: usbphyc::Instance,
    pub usb_global: otg_hs_global::Instance,
    pub usb_device: otg_hs_device::Instance,
    pub usb_pwrclk: otg_hs_pwrclk::Instance,
    pub hclk: u32,
}

// We only store peripheral instances to enforce ownership,
// so it's safe to share the USB object
unsafe impl Send for USB {}
unsafe impl Sync for USB {}

unsafe impl UsbPeripheral for USB {
    const REGISTERS: *const () = otg_hs_global::OTG_HS_GLOBAL as *const ();

    const HIGH_SPEED: bool = true;
    const FIFO_DEPTH_WORDS: usize = 1024;
    const ENDPOINT_COUNT: usize = 9;

    fn enable() {
        cortex_m::interrupt::free(|_| {
            let rcc = unsafe { &*rcc::RCC };

            // Enable and reset USB peripheral
            modify_reg!(rcc, rcc, AHB1ENR, OTGHSEN: Enabled);
            modify_reg!(rcc, rcc, AHB1RSTR, OTGHSRST: Reset);
            modify_reg!(rcc, rcc, AHB1RSTR, OTGHSRST: 0);

            // Enable and reset HS PHY
            modify_reg!(rcc, rcc, AHB1ENR, OTGHSULPIEN: Enabled);
            modify_reg!(rcc, rcc, APB2ENR, USBPHYCEN: Enabled);
            modify_reg!(rcc, rcc, APB2RSTR, USBPHYCRST: Reset);
            modify_reg!(rcc, rcc, APB2RSTR, USBPHYCRST: 0);
        });
    }

    #[inline(always)]
    fn ahb_frequency_hz(&self) -> u32 {
        self.hclk
    }

    #[inline(always)]
    fn phy_type(&self) -> PhyType {
        PhyType::InternalHighSpeed
    }

    fn setup_internal_hs_phy(&self) {
        let phy = unsafe { &*usbphyc::USBPHYC };

        // Turn on LDO
        // For some reason setting the bit enables the LDO
        modify_reg!(usbphyc, phy, LDO, LDO_DISABLE: 1);
        while read_reg!(usbphyc, phy, LDO, LDO_STATUS) == 0 {}

        // Setup PLL
        write_reg!(usbphyc, phy, PLL1,
            PLL1SEL: 0b000 // A value for 12MHz HSE
        );
        modify_reg!(usbphyc, phy, TUNE, |r| r | 0xF13);
        modify_reg!(usbphyc, phy, PLL1, PLL1EN: 1);

        // 2ms Delay required to get internal phy clock stable
        cortex_m::asm::delay(432000);
    }
}

pub type UsbBusType = UsbBus<USB>;
