use stm32ral::gpio;
use stm32ral::{modify_reg, read_reg, write_reg};

pub struct GPIO {
    p: gpio::Instance,
}

impl<'a> GPIO {
    pub fn new(p: gpio::Instance) -> Self {
        GPIO { p }
    }

    pub fn pin(&'a self, n: u8) -> Pin<'a> {
        assert!(n < 16);
        let n = unsafe { core::mem::transmute(n) };
        Pin { n, port: self }
    }

    #[inline(always)]
    pub fn set_high(&'a self, n: PinIndex) -> &Self {
        write_reg!(gpio, self.p, BSRR, 1 << (n as u8));
        self
    }

    #[inline(always)]
    pub fn set_low(&'a self, n: PinIndex) -> &Self {
        write_reg!(gpio, self.p, BSRR, 1 << ((n as u8) + 16));
        self
    }

    #[inline]
    pub fn toggle(&'a self, n: PinIndex) -> &Self {
        let pin = (read_reg!(gpio, self.p, IDR) >> (n as u8)) & 1;
        if pin == 1 {
            self.set_low(n)
        } else {
            self.set_high(n)
        }
    }

    #[inline]
    pub fn set_mode(&'a self, n: PinIndex, mode: u32) -> &Self {
        let offset = (n as u8) * 2;
        let mask = 0b11 << offset;
        let val = (mode << offset) & mask;
        modify_reg!(gpio, self.p, MODER, |r| (r & !mask) | val);
        self
    }

    pub const fn memoise_mode(n: PinIndex, mode: u32) -> MemoisedMode {
        let n = (n as u8) & 0xF;
        let offset = n * 2;
        let mask = 0b11 << offset;
        let value = (mode << offset) & mask;
        MemoisedMode { mask: !mask, value }
    }

    #[inline]
    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        modify_reg!(gpio, self.p, MODER, |r| (r & mode.mask) | mode.value);
        self
    }

    #[inline]
    pub fn set_mode_input(&'a self, n: PinIndex) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    pub const fn memoise_mode_input(n: PinIndex) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Input)
    }

    #[inline]
    pub fn set_mode_output(&'a self, n: PinIndex) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    pub const fn memoise_mode_output(n: PinIndex) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Output)
    }

    #[inline]
    pub fn set_mode_alternate(&'a self, n: PinIndex) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    pub const fn memoise_mode_alternate(n: PinIndex) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Alternate)
    }

    #[inline]
    pub fn set_mode_analog(&'a self, n: PinIndex) -> &Self {
        self.set_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    pub const fn memoise_mode_analog(n: PinIndex) -> MemoisedMode {
        Self::memoise_mode(n, gpio::MODER::MODER0::RW::Analog)
    }

    #[inline]
    pub fn set_otype(&'a self, n: PinIndex, otype: u32) -> &Self {
        let offset = n as u8;
        let mask = 0b1 << offset;
        let val = (otype << offset) & mask;
        modify_reg!(gpio, self.p, OTYPER, |r| (r & !mask) | val);
        self
    }

    #[inline]
    pub fn set_otype_opendrain(&'a self, n: PinIndex) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::OpenDrain)
    }

    #[inline]
    pub fn set_otype_pushpull(&'a self, n: PinIndex) -> &Self {
        self.set_otype(n, gpio::OTYPER::OT0::RW::PushPull)
    }

    #[inline]
    pub fn set_ospeed(&'a self, n: PinIndex, ospeed: u32) -> &Self {
        let offset = (n as u8) * 2;
        let mask = 0b11 << offset;
        let val = (ospeed << offset) & mask;
        modify_reg!(gpio, self.p, OSPEEDR, |r| (r & !mask) | val);
        self
    }

    #[inline]
    pub fn set_ospeed_low(&'a self, n: PinIndex) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::LowSpeed)
    }

    #[inline]
    pub fn set_ospeed_medium(&'a self, n: PinIndex) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::MediumSpeed)
    }

    #[inline]
    pub fn set_ospeed_high(&'a self, n: PinIndex) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::HighSpeed)
    }

    #[inline]
    pub fn set_ospeed_veryhigh(&'a self, n: PinIndex) -> &Self {
        self.set_ospeed(n, gpio::OSPEEDR::OSPEEDR0::RW::VeryHighSpeed)
    }

    #[inline]
    pub fn set_af(&'a self, n: PinIndex, af: u32) -> &Self {
        let n = n as u8;
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

    #[inline]
    pub fn set_pull(&'a self, n: PinIndex, pull: u32) -> &Self {
        let offset = (n as u8) * 2;
        let mask = 0b11 << offset;
        let val = (pull << offset) & mask;
        modify_reg!(gpio, self.p, PUPDR, |r| (r & !mask) | val);
        self
    }

    #[inline]
    pub fn set_pull_floating(&'a self, n: PinIndex) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::Floating)
    }

    #[inline]
    pub fn set_pull_up(&'a self, n: PinIndex) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullUp)
    }

    #[inline]
    pub fn set_pull_down(&'a self, n: PinIndex) -> &Self {
        self.set_pull(n, gpio::PUPDR::PUPDR0::RW::PullDown)
    }

    #[inline]
    pub fn get_idr(&'a self) -> u32 {
        read_reg!(gpio, self.p, IDR)
    }

    #[inline]
    pub fn get_pin_idr(&'a self, n: PinIndex) -> u32 {
        let n = n as u8;
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

#[derive(Copy, Clone)]
#[repr(u8)]
pub enum PinIndex {
    Pin0 = 0,
    Pin1 = 1,
    Pin2 = 2,
    Pin3 = 3,
    Pin4 = 4,
    Pin5 = 5,
    Pin6 = 6,
    Pin7 = 7,
    Pin8 = 8,
    Pin9 = 9,
    Pin10 = 10,
    Pin11 = 11,
    Pin12 = 12,
    Pin13 = 13,
    Pin14 = 14,
    Pin15 = 15,
}

pub struct Pin<'a> {
    n: PinIndex,
    port: &'a GPIO,
}

impl<'a> Pin<'a> {
    #[inline(always)]
    pub fn set_high(&self) -> &Self {
        self.port.set_high(self.n);
        self
    }

    #[inline(always)]
    pub fn set_low(&self) -> &Self {
        self.port.set_low(self.n);
        self
    }

    #[inline(always)]
    pub fn set_bool(&self, state: bool) {
        match state {
            false => self.set_low(),
            true => self.set_high(),
        };
    }

    #[inline(always)]
    pub fn set_state(&self, state: PinState) {
        match state {
            PinState::Low => self.set_low(),
            PinState::High => self.set_high(),
        };
    }

    #[inline(always)]
    pub fn get_state(&self) -> PinState {
        match self.port.get_pin_idr(self.n) {
            0 => PinState::Low,
            1 => PinState::High,
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    pub fn is_high(&self) -> bool {
        match self.get_state() {
            PinState::High => true,
            PinState::Low => false,
        }
    }

    #[inline(always)]
    pub fn is_low(&self) -> bool {
        match self.get_state() {
            PinState::Low => true,
            PinState::High => false,
        }
    }

    #[inline(always)]
    pub fn toggle(&'a self) -> &Self {
        self.port.toggle(self.n);
        self
    }

    #[inline]
    pub fn set_mode_input(&'a self) -> &Self {
        self.port.set_mode_input(self.n);
        self
    }

    #[inline]
    pub fn set_mode_output(&'a self) -> &Self {
        self.port.set_mode_output(self.n);
        self
    }

    #[inline]
    pub fn set_mode_alternate(&'a self) -> &Self {
        self.port.set_mode_alternate(self.n);
        self
    }

    #[inline]
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

    #[inline]
    pub fn apply_memoised_mode(&'a self, mode: MemoisedMode) -> &Self {
        self.port.apply_memoised_mode(mode);
        self
    }

    #[inline]
    pub fn set_otype_opendrain(&'a self) -> &Self {
        self.port.set_otype_opendrain(self.n);
        self
    }

    #[inline]
    pub fn set_otype_pushpull(&'a self) -> &Self {
        self.port.set_otype_pushpull(self.n);
        self
    }

    #[inline]
    pub fn set_ospeed_low(&'a self) -> &Self {
        self.port.set_ospeed_low(self.n);
        self
    }

    #[inline]
    pub fn set_ospeed_medium(&'a self) -> &Self {
        self.port.set_ospeed_medium(self.n);
        self
    }

    #[inline]
    pub fn set_ospeed_high(&'a self) -> &Self {
        self.port.set_ospeed_high(self.n);
        self
    }

    #[inline]
    pub fn set_ospeed_veryhigh(&'a self) -> &Self {
        self.port.set_ospeed_veryhigh(self.n);
        self
    }

    #[inline]
    pub fn set_af(&'a self, af: u32) -> &Self {
        self.port.set_af(self.n, af);
        self
    }

    #[inline]
    pub fn set_pull_floating(&'a self) -> &Self {
        self.port.set_pull_floating(self.n);
        self
    }

    #[inline]
    pub fn set_pull_up(&'a self) -> &Self {
        self.port.set_pull_up(self.n);
        self
    }

    #[inline]
    pub fn set_pull_down(&'a self) -> &Self {
        self.port.set_pull_down(self.n);
        self
    }
}

pub struct Pins<'a> {
    pub led_red: Pin<'a>,
    pub led_green: Pin<'a>,
    pub led_blue: Pin<'a>,

    pub t5v_en: Pin<'a>,
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

        // Push-pull output drives target 5V supply enable.
        self.t5v_en
            .set_low()
            .set_otype_pushpull()
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
        self.gnd_detect.set_pull_up().set_mode_input();

        // Used for SWO in SWD mode. Starts high-impedance.
        self.usart1_rx.set_af(7).set_mode_input();

        // VCP pins
        self.usart2_rx.set_af(7).set_pull_up().set_mode_alternate();
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
        self.spi1_miso.set_af(5).set_mode_input();

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
        self.spi2_miso.set_af(5).set_mode_input();

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
    #[inline]
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
    #[inline]
    pub fn jtag_mode(&self) {
        self.reset.set_mode_output();
        self.usart1_rx.set_mode_input();
        self.spi1_clk.set_mode_input();
        self.spi1_miso.set_mode_input();
        self.spi1_mosi.set_mode_output();
        self.spi2_clk.set_mode_output();
        self.spi2_miso.set_mode_input();
        self.spi2_mosi.set_mode_output();
    }

    /// Place SPI pins into SWD mode
    #[inline]
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
    #[inline]
    pub fn swd_rx(&self) {
        self.spi1_mosi.set_mode_input();
    }

    /// Connect SPI1_MOSI to SWDIO, SPI drives the bus
    #[inline]
    pub fn swd_tx(&self) {
        self.spi1_mosi.set_mode_alternate();
    }

    /// Connect SPI1_MOSI to SWDIO, manual bitbanging
    #[inline]
    pub fn swd_tx_direct(&self) {
        self.spi1_mosi.set_mode_output();
    }

    /// Swap SPI1_CLK pin to direct output mode for manual driving
    #[inline]
    pub fn swd_clk_direct(&self) {
        self.spi1_clk.set_mode_output();
    }

    /// Swap SPI1_CLK pin back to alternate mode for SPI use
    #[inline]
    pub fn swd_clk_spi(&self) {
        self.spi1_clk.set_mode_alternate();
    }
}
