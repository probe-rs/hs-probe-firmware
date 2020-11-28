use hs_probe_bsp as bsp;
use hs_probe_bsp::rcc::CoreFrequency;
use crate::dap::DAPVersion;

pub enum Request {
    Suspend,
    DAP1Command(([u8; 64], usize)),
    DAP2Command(([u8; 512], usize)),
}

pub struct App<'a> {
    rcc: &'a bsp::rcc::RCC,
    dma: &'a bsp::dma::DMA,
    pins: &'a bsp::gpio::Pins<'a>,
    spi: &'a bsp::spi::SPI,
    usb: &'a mut crate::usb::USB,
    dap: &'a mut crate::dap::DAP<'a>,
    delay: &'a bsp::delay::Delay,
}

impl<'a> App<'a> {
    pub fn new(
        rcc: &'a bsp::rcc::RCC,
        dma: &'a bsp::dma::DMA,
        pins: &'a bsp::gpio::Pins<'a>,
        spi: &'a bsp::spi::SPI,
        usb: &'a mut crate::usb::USB,
        dap: &'a mut crate::dap::DAP<'a>,
        delay: &'a bsp::delay::Delay,
    ) -> Self {
        App {
            rcc,
            dma,
            pins,
            spi,
            usb,
            dap,
            delay,
        }
    }

    /// Unsafety: this function should be called from the main context.
    /// No other contexts should be active at the same time.
    pub unsafe fn setup(&mut self, serial: &'static str) {
        // Configure system clock
        #[cfg(not(feature = "turbo"))]
        let clocks = self.rcc.setup(CoreFrequency::F72MHz);
        #[cfg(feature = "turbo")]
        let clocks = self.rcc.setup(CoreFrequency::F216MHz);

        self.delay.set_sysclk(&clocks);

        // Configure DMA for SPI1, SPI2, USART1 and USART2 transfers
        self.dma.setup();

        // Configure GPIOs
        self.pins.setup();
        self.pins.high_impedance_mode();

        self.spi.set_base_clock(&clocks);
        self.spi.disable();

        // Configure USB peripheral and connect to host
        self.usb.setup(&clocks, serial);

        self.pins.led_green.set_low();
    }

    pub fn poll(&mut self) {
        if let Some(req) = self.usb.interrupt() {
            self.process_request(req);
        }

        if self.dap.is_swo_streaming() && !self.usb.dap2_swo_is_busy() {
            // Poll for new UART data when streaming is enabled and
            // the SWO endpoint is ready to transmit more data.
            if let Some(data) = self.dap.poll_swo() {
                self.usb.dap2_stream_swo(data);
            }
        }
    }

    fn process_request(&mut self, req: Request) {
        match req {
            Request::DAP1Command((report, n)) => {
                let response = self.dap.process_command(&report[..n], DAPVersion::V1);
                if let Some(data) = response {
                    self.usb.dap1_reply(data);
                }
            }
            Request::DAP2Command((report, n)) => {
                let response = self.dap.process_command(&report[..n], DAPVersion::V2);
                if let Some(data) = response {
                    self.usb.dap2_reply(data);
                }
            }
            Request::Suspend => {
                self.pins.high_impedance_mode();
                self.pins.led_blue.set_high();
                self.pins.tvcc_en.set_low();
                self.spi.disable();
            }
        }
    }
}
