use stm32ral::{
    usbphyc,
    otg_hs_global,
    otg_hs_device,
    otg_hs_pwrclk
};
use crate::app::Request;
use hs_probe_bsp::rcc::Clocks;
use hs_probe_bsp::otg_hs::{UsbBusType, UsbBus};
use usb_device::prelude::*;
use usb_device::bus::UsbBusAllocator;
use usbd_serial::SerialPort;

mod winusb;
mod dap_v1;
mod dap_v2;
mod dfu;

use winusb::MicrosoftDescriptors;
use dap_v1::CmsisDapV1;
use dap_v2::CmsisDapV2;
use dfu::DfuRuntime;


struct UninitializedUSB {
    phy: usbphyc::Instance,
    global: otg_hs_global::Instance,
    device: otg_hs_device::Instance,
    pwrclk: otg_hs_pwrclk::Instance,
}

struct InitializedUSB {
    device: UsbDevice<'static, UsbBusType>,
    device_state: UsbDeviceState,
    winusb: MicrosoftDescriptors,
    dap_v1: CmsisDapV1<'static, UsbBusType>,
    dap_v2: CmsisDapV2<'static, UsbBusType>,
    serial: SerialPort<'static, UsbBusType>,
    dfu: DfuRuntime,
}

enum State {
    Uninitialized(UninitializedUSB),
    Initialized(InitializedUSB),
    Initializing,
}

impl State {
    pub fn as_initialized(&self) -> &InitializedUSB {
        if let State::Initialized(initialized) = self {
            return initialized;
        } else {
            panic!("USB is not initialized yet");
        }
    }

    pub fn as_initialized_mut(&mut self) -> &mut InitializedUSB {
        if let State::Initialized(initialized) = self {
            return initialized;
        } else {
            panic!("USB is not initialized yet");
        }
    }
}

static mut EP_MEMORY: [u32; 4096] = [0; 4096];
static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

/// USB stack interface
pub struct USB {
    state: State,
}

impl USB {
    /// Create a new USB object from the peripheral instance
    pub fn new(
        phy: usbphyc::Instance,
        global: otg_hs_global::Instance,
        device: otg_hs_device::Instance,
        pwrclk: otg_hs_pwrclk::Instance,
    ) -> Self {
        let usb = UninitializedUSB {
            phy,
            global,
            device,
            pwrclk
        };
        USB {
            state: State::Uninitialized(usb)
        }
    }

    /// Initialise the USB peripheral ready to start processing packets
    pub fn setup(&mut self, clocks: &Clocks, serial_string: &'static str) {
        let state = core::mem::replace(&mut self.state, State::Initializing);
        if let State::Uninitialized(usb) = state {
            cortex_m::interrupt::free(|_| unsafe {
                let usb = hs_probe_bsp::otg_hs::USB {
                    usb_phy: usb.phy,
                    usb_global: usb.global,
                    usb_device: usb.device,
                    usb_pwrclk: usb.pwrclk,
                    hclk: clocks.hclk()
                };

                let usb_bus = UsbBus::new(usb, &mut EP_MEMORY);
                USB_BUS = Some(usb_bus);
                let usb_bus = USB_BUS.as_ref().unwrap();

                let winusb = MicrosoftDescriptors;
                let dap_v1 = CmsisDapV1::new(&usb_bus);
                let dap_v2 = CmsisDapV2::new(&usb_bus);
                let serial = SerialPort::new(&usb_bus);
                let dfu = DfuRuntime::new(&usb_bus);

                let device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x1209, 0x4853))
                    .manufacturer("Probe-rs development team")
                    .product("HS-Probe with CMSIS-DAP Support")
                    .serial_number(serial_string)
                    .device_class(0)
                    .max_packet_size_0(64)
                    .max_power(500)
                    .build();
                let device_state = device.state();

                let usb = InitializedUSB {
                    device,
                    device_state,
                    winusb,
                    dap_v1,
                    dap_v2,
                    serial,
                    dfu,
                };
                self.state = State::Initialized(usb)
            });
        } else {
            panic!("Invalid state");
        }
    }

    /// Process a pending USB interrupt.
    ///
    /// Call this function when a USB interrupt occurs.
    ///
    /// Returns Some(Request) if a new request has been received
    /// from the host.
    ///
    /// This function will clear the interrupt bits of all interrupts
    /// it processes; if any are unprocessed the USB interrupt keeps
    /// triggering until all are processed.
    pub fn interrupt(&mut self) -> Option<Request> {
        let usb = self.state.as_initialized_mut();
        if usb.device.poll(&mut [
            &mut usb.winusb, &mut usb.dap_v1, &mut usb.dap_v2, &mut usb.serial, &mut usb.dfu
        ]) {
            let old_state = usb.device_state;
            let new_state = usb.device.state();
            usb.device_state = new_state;
            if (old_state != new_state) && (new_state != UsbDeviceState::Configured) {
                return Some(Request::Suspend);
            }

            let r = usb.dap_v1.process();
            if r.is_some() {
                return r;
            }

            let r = usb.dap_v2.process();
            if r.is_some() {
                return r;
            }

            // Discard data from the serial interface
            let mut buf = [0; 512];
            let _ = usb.serial.read(&mut buf);
        }
        None
    }

    /// Transmit a DAP report back over the DAPv1 HID interface
    pub fn dap1_reply(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v1.write_packet(data).expect("DAPv1 EP write failed");
    }

    /// Transmit a DAP report back over the DAPv2 bulk interface
    pub fn dap2_reply(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v2.write_packet(data).expect("DAPv2 EP write failed");
    }

    /// Check if SWO endpoint is currently busy transmitting data
    pub fn dap2_swo_is_busy(&self) -> bool {
        let usb = self.state.as_initialized();
        usb.dap_v2.trace_busy()
    }

    /// Transmit SWO streaming data back over the DAPv2 bulk interface
    pub fn dap2_stream_swo(&mut self, data: &[u8]) {
        let usb = self.state.as_initialized_mut();
        usb.dap_v2.trace_write(data).expect("trace EP write failed");
    }
}
