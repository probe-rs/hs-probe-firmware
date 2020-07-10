//! CDC-ACM serial port example using polling in a busy loop.
//!
//! WARNING! FullSpeed setup assumes you have a bodge wire hack:
//! * LED2 & LED3 removed
//! * D2 pads connected to R6 and R7 pads

#![no_std]
#![no_main]

use panic_rtt_target as _;

use cortex_m_rt::entry;
use hs_probe_bsp::rcc::{RCC, CoreFrequency};
#[cfg(feature = "fs")]
use hs_probe_bsp::otg_fs::{USB, UsbBus};
#[cfg(feature = "hs")]
use hs_probe_bsp::otg_hs::{USB, UsbBus};
use usb_device::prelude::*;
use hs_probe_bsp::gpio::GPIO;


#[entry]
fn main() -> ! {
    rtt_target::rtt_init_print!();

    let rcc = RCC::new(stm32ral::rcc::RCC::take().unwrap());
    let clocks = unsafe { rcc.setup(CoreFrequency::F72MHz) };

    let gpioc = GPIO::new(stm32ral::gpio::GPIOC::take().unwrap());
    let led = gpioc.pin(10);
    // Open-drain output to LED (active low).
    led
        .set_high()
        .set_otype_opendrain()
        .set_ospeed_low()
        .set_mode_output();

    #[cfg(feature = "fs")]
    let gpioa = GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    #[cfg(feature = "hs")]
    let gpiob = GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());

    #[cfg(feature = "fs")]
    let usb = USB::setup(&gpioa, &clocks);
    #[cfg(feature = "hs")]
    let usb = USB::setup(&gpiob, &clocks);

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
                led.set_low(); // Turn on

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

        led.set_high(); // Turn off
    }
}
