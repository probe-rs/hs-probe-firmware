use stm32ral::{
    otg_fs_global,
    otg_fs_device,
    otg_fs_pwrclk
};

use crate::app::Request;
use hs_probe_bsp::rcc::Clocks;


/// USB stack interface
pub struct USB {
}

impl USB {
    /// Create a new USB object from the peripheral instance
    pub fn new(
        global: otg_fs_global::Instance,
        device: otg_fs_device::Instance,
        pwrclk: otg_fs_pwrclk::Instance,
    ) -> Self {
        todo!()
    }

    /// Initialise the USB peripheral ready to start processing packets
    pub fn setup(&mut self, clocks: &Clocks) {
        todo!()
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
        todo!()
    }

    /// Transmit a DAP report back over the DAPv1 HID interface
    pub fn dap1_reply(&mut self, data: &[u8]) {
        todo!()
    }

    /// Transmit a DAP report back over the DAPv2 bulk interface
    pub fn dap2_reply(&mut self, data: &[u8]) {
        todo!()
    }

    /// Check if SWO endpoint is currently busy transmitting data
    pub fn dap2_swo_is_busy(&self) -> bool {
        todo!()
    }

    /// Transmit SWO streaming data back over the DAPv2 bulk interface
    pub fn dap2_stream_swo(&mut self, data: &[u8]) {
        todo!()
    }

    /// Indicate we can currently receive DAP requests
    pub fn dap_enable(&mut self) {
        todo!()
    }

    // Indicate we cannot currently receive DAP requests
    pub fn dap_disable(&mut self) {
        todo!()
    }
}
