#![no_std]
#![no_main]

use cortex_m_rt::{entry, pre_init};
use git_version::git_version;
pub use hs_probe_bsp as bsp;
use panic_rtt_target as _;
use rtt_target::{rtt_init_print, rprintln};
use stm32_device_signature::device_id_hex;

const GIT_VERSION: &str = git_version!();

mod app;
mod dap;
mod swd;
mod usb;

#[pre_init]
unsafe fn pre_init() {
    // Check if we should jump to system bootloader.
    //
    // When we receive the BOOTLOAD command over USB,
    // we write a flag to a static and reset the chip,
    // and `bootload::check()` will jump to the system
    // memory bootloader if the flag is present.
    //
    // It must be called from pre_init as otherwise the
    // flag is overwritten when statics are initialised.
    bsp::bootload::check();
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let rcc = bsp::rcc::RCC::new(stm32ral::rcc::RCC::take().unwrap());

    let usb_phy = stm32ral::usbphyc::USBPHYC::take().unwrap();
    let usb_global = stm32ral::otg_hs_global::OTG_HS_GLOBAL::take().unwrap();
    let usb_device = stm32ral::otg_hs_device::OTG_HS_DEVICE::take().unwrap();
    let usb_pwrclk = stm32ral::otg_hs_pwrclk::OTG_HS_PWRCLK::take().unwrap();
    let mut usb = crate::usb::USB::new(usb_phy, usb_global, usb_device, usb_pwrclk);

    let dma = bsp::dma::DMA::new(
        stm32ral::dma::DMA1::take().unwrap(),
        stm32ral::dma::DMA2::take().unwrap()
    );
    let spi1 = bsp::spi::SPI::new(stm32ral::spi::SPI1::take().unwrap());
    let mut uart1 = bsp::uart::UART::new(stm32ral::usart::USART1::take().unwrap(), &dma);

    let gpioa = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    let gpiob = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());
    let gpioc = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOC::take().unwrap());
    let gpiod = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOD::take().unwrap());
    let gpioe = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOE::take().unwrap());
    let gpiog = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOG::take().unwrap());
    let gpioi = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOI::take().unwrap());

    let pins = bsp::gpio::Pins {
        led: gpioc.pin(10),
        tvcc_en: gpioe.pin(2),
        reset: gpiog.pin(13),
        gnd_detect: gpiog.pin(14),
        usart1_rx: gpiob.pin(7),
        usart2_rx: gpiod.pin(6),
        usart2_tx: gpiod.pin(5),
        spi1_clk: gpiob.pin(3),
        spi1_miso: gpiob.pin(4),
        spi1_mosi: gpiob.pin(5),
        spi2_clk: gpioi.pin(1),
        spi2_miso: gpioi.pin(2),
        spi2_mosi: gpioi.pin(3),
        usb_dm: gpiob.pin(14),
        usb_dp: gpiob.pin(15),
    };

    let swd = swd::SWD::new(&spi1, &pins);
    let mut dap = dap::DAP::new(swd, &mut uart1, &pins);

    // Create App instance with the HAL instances
    let mut app = app::App::new(&rcc, &dma, &pins, &spi1, &mut usb, &mut dap);

    rprintln!("Starting...");

    // Initialise application, including system peripherals
    unsafe { app.setup(device_id_hex()) };

    loop {
        // Process events
        app.poll();
    }
}
