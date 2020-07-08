//! CDC-ACM serial port example using polling in a busy loop.
//!
//! WARNING! FullSpeed setup assumes you have a bodge wire hack:
//! * LED2 & LED3 removed
//! * D2 pads connected to R6 and R7 pads

#![no_std]
#![no_main]


use panic_rtt_target as _;

use cortex_m_rt::entry;
use stm32f7xx_hal::prelude::*;
use stm32f7xx_hal::pac;
use stm32f7xx_hal::rcc::{HSEClock, HSEClockMode};
#[cfg(feature = "fs")]
use stm32f7xx_hal::otg_fs::{USB, UsbBus};
#[cfg(feature = "hs")]
use stm32f7xx_hal::otg_hs::{USB, UsbBus};
use usb_device::prelude::*;
use embedded_hal::digital::v2::OutputPin;

#[entry]
fn main() -> ! {
    rtt_target::rtt_init_print!();

    let dp = pac::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr
        .hse(HSEClock::new(12.mhz(), HSEClockMode::Bypass))
        .sysclk(216.mhz())
        .freeze();

    let gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc10.into_open_drain_output();
    led.set_high().ok(); // Turn off

    #[cfg(feature = "fs")]
    let gpioa = dp.GPIOA.split();
    #[cfg(feature = "hs")]
    let gpiob = dp.GPIOB.split();

    #[cfg(feature = "fs")]
    let usb = USB {
        usb_global: dp.OTG_FS_GLOBAL,
        usb_device: dp.OTG_FS_DEVICE,
        usb_pwrclk: dp.OTG_FS_PWRCLK,
        pin_dm: gpioa.pa11.into_alternate_af10(),
        pin_dp: gpioa.pa12.into_alternate_af10(),
        hclk: clocks.hclk(),
    };
    #[cfg(feature = "hs")]
    let usb = USB {
        usb_global: dp.OTG_HS_GLOBAL,
        usb_device: dp.OTG_HS_DEVICE,
        usb_pwrclk: dp.OTG_HS_PWRCLK,
        pin_dm: gpiob.pb14.into_alternate_af12(),
        pin_dp: gpiob.pb15.into_alternate_af12(),
        hclk: clocks.hclk(),
    };

    static mut EP_MEMORY: [u32; 1024] = [0; 1024];
    let usb_bus = UsbBus::new(usb, unsafe { &mut EP_MEMORY });

    let mut serial = usbd_serial::SerialPort::new(&usb_bus);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    loop {
        if !usb_dev.poll(&mut [&mut serial]) {
            continue;
        }

        let mut buf = [0u8; 64];

        match serial.read(&mut buf) {
            Ok(count) if count > 0 => {
                led.set_low().ok(); // Turn on

                // Echo back in upper case
                for c in buf[0..count].iter_mut() {
                    if 0x61 <= *c && *c <= 0x7a {
                        *c &= !0x20;
                    }
                }

                let mut write_offset = 0;
                while write_offset < count {
                    match serial.write(&buf[write_offset..count]) {
                        Ok(len) if len > 0 => {
                            write_offset += len;
                        },
                        _ => {},
                    }
                }
            }
            _ => {}
        }

        led.set_high().ok(); // Turn off
    }
}
