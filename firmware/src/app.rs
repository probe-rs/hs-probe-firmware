use hs_probe_bsp as bsp;
use hs_probe_bsp::rcc::CoreFrequency;

pub enum Request {
    DAP1Command(([u8; 64], usize)),
    DAP2Command(([u8; 64], usize)),
}

pub struct App<'a> {
    rcc: &'a bsp::rcc::RCC,
    pins: &'a bsp::gpio::Pins<'a>,
    usb: &'a mut crate::usb::USB,
    dap: &'a mut crate::dap::DAP<'a>,
}

impl<'a> App<'a> {
    pub fn new(rcc: &'a bsp::rcc::RCC,
               pins: &'a bsp::gpio::Pins<'a>,
               usb: &'a mut crate::usb::USB,
               dap: &'a mut crate::dap::DAP<'a>,
    ) -> Self {
        App {
            rcc,
            pins,
            usb,
            dap,
        }
    }

    /// Unsafety: this function should be called from the main context.
    /// No other contexts should be active at the same time.
    pub unsafe fn setup(&mut self) {
        // Configure system clock
        let clocks = self.rcc.setup(CoreFrequency::F72MHz);
        // Configure GPIOs
        self.pins.setup();
        // Configure USB peripheral and connect to host
        self.usb.setup(&clocks);
    }

    pub fn poll(&mut self) {
        if let Some(req) = self.usb.interrupt() {
            self.process_request(req);
        }
    }

    fn process_request(&mut self, req: Request) {
    }
}
