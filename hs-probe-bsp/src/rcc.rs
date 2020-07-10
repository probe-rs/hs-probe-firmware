use stm32ral::{rcc, flash};
use stm32ral::{read_reg, modify_reg, reset_reg};

pub struct RCC {
    rcc: rcc::Instance,
}

impl RCC {
    pub fn new(rcc: rcc::Instance) -> Self {
        RCC { rcc }
    }

    /// Set up the device, enabling all required clocks
    pub fn setup(&self, frequency: CoreFrequency) {
        // Turn on HSI
        modify_reg!(rcc, self.rcc, CR, HSION: On);
        // Wait for HSI to be ready
        while read_reg!(rcc, self.rcc, CR, HSIRDY == NotReady) {}
        // Swap system clock to HSI
        modify_reg!(rcc, self.rcc, CFGR, SW: HSI);
        // Wait for system clock to be HSI
        while read_reg!(rcc, self.rcc, CFGR, SWS != HSI) {}

        // Disable everything
        modify_reg!(rcc, self.rcc, CR,
            HSEON: Off,
            CSSON: Off,
            PLLON: Off,
            PLLI2SON: Off,
            PLLSAION: Off
        );
        reset_reg!(rcc, self.rcc, RCC, AHB1ENR);
        reset_reg!(rcc, self.rcc, RCC, AHB2ENR);
        reset_reg!(rcc, self.rcc, RCC, AHB3ENR);
        reset_reg!(rcc, self.rcc, RCC, APB1ENR);
        reset_reg!(rcc, self.rcc, RCC, APB2ENR);

        // Configure HSE in bypass mode
        modify_reg!(rcc, self.rcc, CR, HSEBYP: Bypassed);
        // Start HSE
        modify_reg!(rcc, self.rcc, CR, HSEON: On);
        // Wait for HSE to be ready
        while read_reg!(rcc, self.rcc, CR, HSERDY == NotReady) {}

        // Calculate PLL parameters and flash latency
        let pllm = 6;
        let plln;
        let pllp;
        let pllq;
        let flash_latency;
        match frequency {
            CoreFrequency::F48MHz => {
                plln = 96;
                pllp = 0b01; // /4
                pllq = 4;
                flash_latency = 0b0001;
            }
            CoreFrequency::F72MHz => {
                plln = 144;
                pllp = 0b01; // /4
                pllq = 6;
                flash_latency = 0b0010;
            }
            CoreFrequency::F216MHz => {
                plln = 216;
                pllp = 0b00; // /2
                pllq = 9;
                flash_latency = 0b0111;
            }
        }

        // Configure PLL from HSE
        modify_reg!(rcc, self.rcc, PLLCFGR,
            PLLSRC: HSE,
            PLLM: pllm,
            PLLN: plln,
            PLLP: pllp,
            PLLQ: pllq
        );

        // Turn on PLL
        modify_reg!(rcc, self.rcc, CR, PLLON: On);
        // Wait for PLL to be ready
        while read_reg!(rcc, self.rcc, CR, PLLRDY == NotReady) {}

        // Adjust flash wait states
        let flash = flash::FLASH::take().unwrap();
        modify_reg!(flash, flash, ACR, LATENCY: flash_latency);
        flash::FLASH::release(flash);

        // Swap system clock to PLL
        modify_reg!(rcc, self.rcc, CFGR, SW: PLL);
        // Wait for system clock to be PLL
        while read_reg!(rcc, self.rcc, CFGR, SWS != PLL) {}

        // Configure HCLK, PCLK1, PCLK2
        modify_reg!(rcc, self.rcc, CFGR, PPRE1: Div1, PPRE2: Div1, HPRE: Div1);

        // Enable peripheral clocks
        modify_reg!(rcc, self.rcc, AHB1ENR, GPIOAEN: Enabled, GPIOCEN: Enabled);
    }
}

#[derive(Eq, PartialEq)]
pub enum CoreFrequency {
    F48MHz,
    F72MHz,
    F216MHz,
}
