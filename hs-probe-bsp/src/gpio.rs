use stm32ral::gpio;
use stm32ral::{read_reg, write_reg, modify_reg};

pub struct GPIO {
    p: gpio::Instance,
}

impl<'a> GPIO {
    pub fn new(p: gpio::Instance) -> Self {
        GPIO { p }
    }

    pub fn pin(&'a self, n: u8) -> Pin<'a> {
        assert!(n < 16);
        Pin { n, port: self }
    }

    pub fn set_high(&'a self, n: u8) -> &Self {
        assert!(n < 16);
        write_reg!(gpio, self.p, BSRR, 1 << n);
        self
    }

    pub fn set_low(&'a self, n: u8) -> &Self {
        assert!(n < 16);
        write_reg!(gpio, self.p, BSRR, 1 << (n + 16));
        self
    }

    pub fn toggle(&'a self, n: u8) -> &Self {
        assert!(n < 16);
        let pin = (read_reg!(gpio, self.p, IDR) >> n) & 1;
        if pin == 1 {
            self.set_low(n)
        } else {
            self.set_high(n)
        }
    }

    pub fn set_mode(&'a self, n: u8, mode: u32) -> &Self {
        assert!(n < 16);
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (mode << offset) & mask;
        modify_reg!(gpio, self.p, MODER, |r| (r & !mask) | val);
        self
    }

    pub const fn memoise_mode(n: u8, mode: u32) -> MemoisedMode {
        let n = n & 0xF;
        let offset = n * 2;
        let mask = 0b11 << offset;
        let value = (mode << offset) & mask;
        MemoisedMode { mask: !mask, value }
    }

    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        modify_reg!(gpio, self.p, MODER, |r| (r & mode.mask) | mode.value);
        self
    }

    pub fn set_mode_input(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    pub const fn memoise_mode_input(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    pub fn set_mode_output(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    pub const fn memoise_mode_output(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    pub fn set_mode_alternate(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    pub const fn memoise_mode_alternate(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    pub fn set_mode_analog(&'a self, n: u8) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    pub const fn memoise_mode_analog(n: u8) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    pub fn set_otype(&'a self, n: u8, otype: u32) -> &Self {
        assert!(n < 16);
        let offset = n;
        let mask = 0b1 << offset;
        let val = (otype << offset) & mask;
        modify_reg!(gpio, self.p, OTYPER, |r| (r & !mask) | val);
        self
    }

    pub fn set_otype_opendrain(&'a self, n: u8) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::OpenDrain)
    }

    pub fn set_otype_pushpull(&'a self, n: u8) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::PushPull)
    }

    pub fn set_ospeed(&'a self, n: u8, ospeed: u32) -> &Self {
        assert!(n < 16);
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (ospeed << offset) & mask;
        modify_reg!(gpio, self.p, OSPEEDR, |r| (r & !mask) | val);
        self
    }

    pub fn set_ospeed_low(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::LowSpeed)
    }

    pub fn set_ospeed_medium(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::MediumSpeed)
    }

    pub fn set_ospeed_high(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::HighSpeed)
    }

    pub fn set_ospeed_veryhigh(&'a self, n: u8) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::VeryHighSpeed)
    }

    pub fn set_af(&'a self, n: u8, af: u32) -> &Self {
        assert!(n < 16);
        if n < 8 {
            let offset = n * 4;
            let mask = 0b1111 << offset;
            let val = (af << offset) & mask;
            modify_reg!(gpio, self.p, AFRL, |r| (r & !mask) | val);
        } else {
            let offset = (n - 8) * 4;
            let mask = 0b1111 << offset;
            let val = (af << offset) & mask;
            modify_reg!(gpio, self.p, AFRH, |r| (r & !mask) | val);
        }
        self
    }

    pub fn set_pull(&'a self, n: u8, pull: u32) -> &Self {
        let offset = n * 2;
        let mask = 0b11 << offset;
        let val = (pull << offset) & mask;
        modify_reg!(gpio, self.p, PUPDR, |r| (r & !mask) | val);
        self
    }

    pub fn set_pull_floating(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::Floating)
    }

    pub fn set_pull_up(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullUp)
    }

    pub fn set_pull_down(&'a self, n: u8) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullDown)
    }

    pub fn get_idr(&'a self) -> u32 {
        read_reg!(gpio, self.p, IDR)
    }

    pub fn get_pin_idr(&'a self, n: u8) -> u32 {
        (self.get_idr() & (1 << n)) >> n
    }
}

/// Stores a pre-computed mask and value for quickly changing pin mode
#[derive(Copy, Clone)]
pub struct MemoisedMode {
    mask: u32,
    value: u32,
}

#[repr(u16)]
pub enum PinState {
    Low = 0,
    High = 1,
}

pub struct Pin<'a> {
    n: u8,
    port: &'a GPIO,
}

impl<'a> Pin<'a> {
    pub fn set_high(&self) -> &Self {
        self.port.set_high(self.n);
        self
    }

    pub fn set_low(&self) -> &Self {
        self.port.set_low(self.n);
        self
    }

    pub fn set_state(&self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        };
    }

    pub fn get_state(&self) -> PinState {
        match self.port.get_pin_idr(self.n) {
            0 => PinState::Low,
            1 => PinState::High,
            _ => unreachable!(),
        }
    }

    pub fn is_high(&self) -> bool {
        match self.get_state() {
            PinState::High => true,
            PinState::Low => false,
        }
    }

    pub fn is_low(&self) -> bool {
        match self.get_state() {
            PinState::Low => true,
            PinState::High => false,
        }
    }

    pub fn toggle(&'a self) -> &Self {
        self.port.toggle(self.n);
        self
    }

    pub fn set_mode_input(&'a self) -> &Self {
        self.port.set_mode_input(self.n);
        self
    }

    pub fn set_mode_output(&'a self) -> &Self {
        self.port.set_mode_output(self.n);
        self
    }

    pub fn set_mode_alternate(&'a self) -> &Self {
        self.port.set_mode_alternate(self.n);
        self
    }

    pub fn set_mode_analog(&'a self) -> &Self {
        self.port.set_mode_analog(self.n);
        self
    }

    pub fn memoise_mode_input(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_input(self.n)
    }

    pub fn memoise_mode_output(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_output(self.n)
    }

    pub fn memoise_mode_alternate(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_alternate(self.n)
    }

    pub fn memoise_mode_analog(&'a self) -> MemoisedMode {
        GPIO::memoise_mode_analog(self.n)
    }

    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        self.port.apply_memoised_mode(mode);
        self
    }

    pub fn set_otype_opendrain(&'a self) -> &Self {
        self.port.set_otype_opendrain(self.n);
        self
    }

    pub fn set_otype_pushpull(&'a self) -> &Self {
        self.port.set_otype_pushpull(self.n);
        self
    }

    pub fn set_ospeed_low(&'a self) -> &Self {
        self.port.set_ospeed_low(self.n);
        self
    }

    pub fn set_ospeed_medium(&'a self) -> &Self {
        self.port.set_ospeed_medium(self.n);
        self
    }

    pub fn set_ospeed_high(&'a self) -> &Self {
        self.port.set_ospeed_high(self.n);
        self
    }

    pub fn set_ospeed_veryhigh(&'a self) -> &Self {
        self.port.set_ospeed_veryhigh(self.n);
        self
    }

    pub fn set_af(&'a self, af: u32) -> &Self {
        self.port.set_af(self.n, af);
        self
    }

    pub fn set_pull_floating(&'a self) -> &Self {
        self.port.set_pull_floating(self.n);
        self
    }

    pub fn set_pull_up(&'a self) -> &Self {
        self.port.set_pull_up(self.n);
        self
    }

    pub fn set_pull_down(&'a self) -> &Self {
        self.port.set_pull_down(self.n);
        self
    }
}

pub struct Pins<'a> {
    pub led_red: Pin<'a>,
    pub led_green: Pin<'a>,
    pub led_blue: Pin<'a>,

    pub tvcc_en: Pin<'a>,
    pub reset: Pin<'a>,
    pub gnd_detect: Pin<'a>,

    // Used for SWO in SWD mode
    pub usart1_rx: Pin<'a>,

    // Used for external serial interface
    pub usart2_rx: Pin<'a>,
    pub usart2_tx: Pin<'a>,

    // SPI pins for SWD, SPI1_MOSI is used as TMS in JTAG mode
    pub spi1_clk: Pin<'a>, // Physically connected to SPI2_CLK
    pub spi1_miso: Pin<'a>,
    pub spi1_mosi: Pin<'a>,

    // SPI pins for JTAG, disabled in SWD mode
    pub spi2_clk: Pin<'a>, // Physically connected to SPI1_CLK
    pub spi2_miso: Pin<'a>,
    pub spi2_mosi: Pin<'a>,

    // USB HS
    pub usb_dm: Pin<'a>,
    pub usb_dp: Pin<'a>,
    pub usb_sel: Pin<'a>,
}

impl<'a> Pins<'a> {
    /// Configure I/O pins
    pub fn setup(&self) {
        // Open-drain output to LED (active low).
        self.led_red
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_low()
            .set_mode_output();

        self.led_green
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_low()
            .set_mode_output();

        self.led_blue
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_low()
            .set_mode_output();

        // Push-pull output drives target supply LDO (active high).
        self.tvcc_en
            .set_low()
            .set_otype_pushpull()
            .set_ospeed_low()
            .set_mode_output();

        // Open-drain output to RESET reset line (active low).
        self.reset
            .set_high()
            .set_otype_opendrain()
            .set_ospeed_high()
            .set_mode_output();

        // Input for GNDDetect
        self.gnd_detect
            .set_pull_up()
            .set_mode_input();

        // Used for SWO in SWD mode. Starts high-impedance.
        self.usart1_rx
            .set_af(7)
            .set_mode_input();

        // VCP pins
        self.usart2_rx
            .set_af(7)
            .set_pull_up()
            .set_mode_alternate();
        self.usart2_tx
            .set_high()
            .set_ospeed_high()
            .set_af(7)
            .set_mode_alternate();

        // Push-pull output to SPI1_CLK. Starts high-impedance.
        self.spi1_clk
            .set_af(5)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Input to SPI1_MISO
        self.spi1_miso
            .set_af(5)
            .set_mode_input();

        // Push-pull output to SPI1_MOSI. Starts high-impedance.
        self.spi1_mosi
            .set_af(5)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Push-pull output to SPI2_CLK. Starts high-impedance.
        self.spi2_clk
            .set_af(5)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // Input to SPI2_MISO
        self.spi2_miso
            .set_af(5)
            .set_mode_input();

        // Push-pull output to SPI2_MOSI. Starts high-impedance.
        self.spi2_mosi
            .set_af(5)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_input();

        // USB HighSpeed pins
        self.usb_dm
            .set_af(12)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_alternate();
        self.usb_dp
            .set_af(12)
            .set_otype_pushpull()
            .set_ospeed_veryhigh()
            .set_mode_alternate();
        self.usb_sel
            .set_high()
            .set_otype_pushpull()
            .set_ospeed_low()
            .set_mode_output();
    }

    /// Place SPI pins into high-impedance mode
    pub fn high_impedance_mode(&self) {
        self.reset.set_high().set_mode_output();
        self.usart1_rx.set_mode_input();
        self.spi1_clk.set_mode_input();
        self.spi1_miso.set_mode_input();
        self.spi1_mosi.set_mode_input();
        self.spi2_clk.set_mode_input();
        self.spi2_miso.set_mode_input();
        self.spi2_mosi.set_mode_input();
    }

    /// Place SPI pins into JTAG mode
    pub fn jtag_mode(&self) {
        self.reset.set_mode_output();
        self.usart1_rx.set_mode_input();
        self.spi1_clk.set_mode_input();
        self.spi1_miso.set_mode_input();
        self.spi1_mosi.set_mode_alternate();
        self.spi2_clk.set_mode_alternate();
        self.spi2_miso.set_mode_alternate();
        self.spi2_mosi.set_mode_alternate();
    }

    /// Place SPI pins into SWD mode
    pub fn swd_mode(&self) {
        self.reset.set_mode_output();
        self.usart1_rx.set_mode_alternate();
        self.spi2_clk.set_mode_input();
        self.spi2_miso.set_mode_input();
        self.spi2_mosi.set_mode_input();
        self.spi1_clk.set_mode_alternate();
        self.spi1_miso.set_mode_alternate();
        self.spi1_mosi.set_mode_alternate();
    }

    /// Disconnect SPI1_MOSI from SWDIO, target drives the bus
    pub fn swd_rx(&self) {
        self.spi1_mosi.set_mode_input();
    }

    /// Connect SPI1_MOSI to SWDIO, we drive the bus
    pub fn swd_tx(&self) {
        self.spi1_mosi.set_mode_alternate();
    }

    /// Swap SPI1_CLK pin to direct output mode for manual driving
    pub fn swd_clk_direct(&self) {
        self.spi1_clk.set_mode_output();
    }

    /// Swap SPI1_CLK pin back to alternate mode for SPI use
    pub fn swd_clk_spi(&self) {
        self.spi1_clk.set_mode_alternate();
    }
}
