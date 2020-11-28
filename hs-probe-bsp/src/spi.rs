// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::sync::atomic::{AtomicU32, Ordering};
use stm32ral::spi;
use stm32ral::{modify_reg, read_reg, write_reg};

use super::dma::DMA;
use super::gpio::Pins;
use crate::rcc::Clocks;
use core::ops::Deref;

pub struct SPI {
    spi: spi::Instance,
    base_clock: AtomicU32,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum SPIPrescaler {
    Div2 = 0b000,
    Div4 = 0b001,
    Div8 = 0b010,
    Div16 = 0b011,
    Div32 = 0b100,
    Div64 = 0b101,
    Div128 = 0b110,
    Div256 = 0b111,
}

impl SPI {
    pub fn new(spi: spi::Instance) -> Self {
        SPI { spi, base_clock: AtomicU32::new(0) }
    }

    pub fn set_base_clock(&self, clocks: &Clocks) {
        if self.spi.deref() as *const _ == spi::SPI1 {
            self.base_clock.store(clocks.pclk2(), Ordering::SeqCst);
        }
        if self.spi.deref() as *const _ == spi::SPI2 {
            self.base_clock.store(clocks.pclk1(), Ordering::SeqCst);
        }
    }

    /// Set up SPI peripheral for SWD mode.
    ///
    /// Defaults to 1.5MHz clock which should be slow enough to work on most targets.
    pub fn setup_swd(&self) {
        write_reg!(
            spi,
            self.spi,
            CR1,
            BIDIMODE: Unidirectional,
            CRCEN: Disabled,
            RXONLY: FullDuplex,
            SSM: Enabled,
            SSI: SlaveNotSelected,
            LSBFIRST: LSBFirst,
            BR: Div32,
            MSTR: Master,
            CPOL: IdleHigh,
            CPHA: SecondEdge,
            SPE: Enabled
        );
    }

    /// Set up SPI peripheral for JTAG mode
    pub fn setup_jtag(&self) {
        // SPI Mode 3 (CPOL=1 CPHA=1)
        write_reg!(
            spi,
            self.spi,
            CR1,
            BIDIMODE: Unidirectional,
            CRCEN: Disabled,
            RXONLY: FullDuplex,
            SSM: Enabled,
            SSI: SlaveNotSelected,
            LSBFIRST: LSBFirst,
            BR: Div256,
            MSTR: Master,
            CPOL: IdleLow,
            CPHA: FirstEdge,
            SPE: Disabled
        );
        write_reg!(
            spi,
            self.spi,
            CR2,
            FRXTH: Quarter,
            DS: EightBit,
            TXDMAEN: Enabled,
            RXDMAEN: Enabled
        );
    }

    pub fn calculate_prescaler(&self, max_frequency: u32) -> Option<SPIPrescaler> {
        let base_clock = self.base_clock.load(Ordering::SeqCst);
        if base_clock == 0 {
            return None
        }

        if (base_clock / 2) <= max_frequency {
            return Some(SPIPrescaler::Div2);
        }
        if (base_clock / 4) <= max_frequency {
            return Some(SPIPrescaler::Div4);
        }
        if (base_clock / 8) <= max_frequency {
            return Some(SPIPrescaler::Div8);
        }
        if (base_clock / 16) <= max_frequency {
            return Some(SPIPrescaler::Div16);
        }
        if (base_clock / 32) <= max_frequency {
            return Some(SPIPrescaler::Div32);
        }
        if (base_clock / 64) <= max_frequency {
            return Some(SPIPrescaler::Div64);
        }
        if (base_clock / 128) <= max_frequency {
            return Some(SPIPrescaler::Div128);
        }
        if (base_clock / 256) <= max_frequency {
            return Some(SPIPrescaler::Div256);
        }
        None
    }

    /// Change SPI clock rate to one of the SPIClock variants
    pub fn set_prescaler(&self, prescaler: SPIPrescaler) {
        modify_reg!(spi, self.spi, CR1, BR: prescaler as u32);
    }

    /// Wait for any pending operation then disable SPI
    pub fn disable(&self) {
        self.wait_busy();
        write_reg!(spi, self.spi, CR1, SPE: Disabled);
    }

    /// Transmit `txdata` and write the same number of bytes into `rxdata`.
    pub fn jtag_exchange(&self, dma: &DMA, txdata: &[u8], rxdata: &mut [u8]) {
        debug_assert!(rxdata.len() >= 64);

        // Set up DMA transfer (configures NDTR and MAR and enables streams)
        dma.spi2_enable(txdata, &mut rxdata[..txdata.len()]);

        // Start SPI transfer
        modify_reg!(spi, self.spi, CR1, SPE: Enabled);

        // Busy wait for RX DMA completion (at most 43µs)
        while dma.spi2_busy() {}

        // Disable SPI and DMA
        dma.spi2_disable();
        modify_reg!(spi, self.spi, CR1, SPE: Disabled);
    }

    /// Transmit 4 bits
    pub fn tx4(&self, data: u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FourBit);
        self.write_dr_u8(data);
        self.wait_txe();
    }

    /// Transmit 8 bits
    pub fn tx8(&self, data: u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        self.write_dr_u8(data);
        self.wait_txe();
    }

    /// Transmit 16 bits
    pub fn tx16(&self, data: u16) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        self.write_dr_u16(data);
        self.wait_txe();
    }

    /// Transmit an SWD WDATA phase, with 32 bits of data and 1 bit of parity.
    ///
    /// We transmit an extra 7 trailing idle bits after the parity bit because
    /// it's much quicker to do that than reconfigure SPI to a smaller data size.
    pub fn swd_wdata_phase(&self, data: u32, parity: u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        // Trigger 4 words, filling the FIFO
        self.write_dr_u16((data & 0xFFFF) as u16);
        self.write_dr_u16((data >> 16) as u16);
        self.wait_txe();
        // Trigger fifth and final word
        self.write_dr_u8(parity & 1);
    }

    /// Receive 4 bits
    pub fn rx4(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FourBit);
        self.write_dr_u8(0);
        self.wait_rxne();
        self.read_dr_u8()
    }

    /// Receive 5 bits
    pub fn rx5(&self) -> u8 {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FiveBit);
        self.write_dr_u8(0);
        self.wait_rxne();
        self.read_dr_u8()
    }

    /// Receive an SWD RDATA phase, with 32 bits of data and 1 bit of parity.
    ///
    /// This method requires `Pins` be passed in so it can directly control
    /// the SWD lines at the end of RDATA in order to correctly sample PARITY
    /// and then resume driving SWDIO.
    pub fn swd_rdata_phase(&self, pins: &Pins) -> (u32, u8) {
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: EightBit);
        // Trigger 4 words, filling the FIFO
        self.write_dr_u16(0);
        self.write_dr_u16(0);
        self.wait_rxne();
        let mut data = self.read_dr_u8() as u32;
        self.wait_rxne();
        data |= (self.read_dr_u8() as u32) << 8;
        self.wait_rxne();
        data |= (self.read_dr_u8() as u32) << 16;

        // While we wait for the final word to be available in the RXFIFO,
        // handle the parity bit. First wait for current transaction to complete.
        self.wait_rxne();

        // The parity bit is currently being driven onto the bus by the target.
        // On the next rising edge, the target will release the bus, and we need
        // to then start driving it before sending any more clocks to avoid a false START.
        let parity = pins.spi1_miso.is_high() as u8;
        // Take direct control of SWCLK
        pins.swd_clk_direct();
        // Send one clock pulse. Target releases bus after rising edge.
        pins.spi1_clk.set_low();
        pins.spi1_clk.set_high();
        // Drive bus ourselves with 0 (all our SPI read transactions transmitted 0s)
        pins.swd_tx();
        // Restore SWCLK to SPI control
        pins.swd_clk_spi();

        // Trigger four dummy idle cycles
        write_reg!(spi, self.spi, CR2, FRXTH: Quarter, DS: FourBit);
        self.write_dr_u8(0);

        // Now read the final data word that was waiting in RXFIFO
        data |= (self.read_dr_u8() as u32) << 24;

        (data, parity)
    }

    /// Empty the receive FIFO
    pub fn drain(&self) {
        // FIFO is 32 bits so ideally we'd make two 16-bit reads, but that screws
        // up the SPI's FIFO pointers and wrecks subsequent reads on later operations.
        // It's still faster to just do 4 reads instead of looping on FRLVL.
        self.read_dr_u8();
        self.read_dr_u8();
        self.read_dr_u8();
        self.read_dr_u8();
    }

    /// Wait for current SPI operation to complete
    #[inline(always)]
    pub fn wait_busy(&self) {
        while read_reg!(spi, self.spi, SR, BSY == Busy) {}
    }

    /// Wait for RXNE
    #[inline(always)]
    fn wait_rxne(&self) {
        while read_reg!(spi, self.spi, SR, RXNE == Empty) {}
    }

    /// Wait for TXE
    #[inline(always)]
    fn wait_txe(&self) {
        while read_reg!(spi, self.spi, SR, TXE != Empty) {}
    }

    /// Perform an 8-bit read from DR
    #[inline(always)]
    fn read_dr_u8(&self) -> u8 {
        unsafe { core::ptr::read_volatile(&self.spi.DR as *const _ as *const u8) }
    }

    /// Perform an 8-bit write to DR
    #[inline(always)]
    fn write_dr_u8(&self, data: u8) {
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u8, data) };
    }

    /// Perform a 16-bit write to DR
    ///
    /// Note that in 8-bit or smaller data mode, this enqueues two transmissions.
    #[inline(always)]
    fn write_dr_u16(&self, data: u16) {
        unsafe { core::ptr::write_volatile(&self.spi.DR as *const _ as *mut u16, data) };
    }
}
