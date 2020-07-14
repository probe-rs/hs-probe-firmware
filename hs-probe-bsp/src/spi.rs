// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::spi;
use stm32ral::{modify_reg, read_reg, write_reg};

use super::dma::DMA;
use super::gpio::Pins;

pub struct SPI {
    spi: spi::Instance,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum SPIClock {
    Clk24M = 0,
    Clk12M = 1,
    Clk6M = 2,
    Clk3M = 3,
    Clk1M5 = 4,
    Clk750k = 5,
    Clk375k = 6,
    Clk187k5 = 7,
}

impl SPIClock {
    /// Returns the highest value for clock that is not higher than `max`,
    /// or None if max is below the slowest clock option.
    pub fn from_max(max: u32) -> Option<Self> {
        match max {
            f if f >= 24_000_000 => Some(SPIClock::Clk24M),
            f if f >= 12_000_000 => Some(SPIClock::Clk12M),
            f if f >= 6_000_000 => Some(SPIClock::Clk6M),
            f if f >= 3_000_000 => Some(SPIClock::Clk3M),
            f if f >= 1_500_000 => Some(SPIClock::Clk1M5),
            f if f >= 750_000 => Some(SPIClock::Clk750k),
            f if f >= 375_000 => Some(SPIClock::Clk375k),
            f if f >= 187_500 => Some(SPIClock::Clk187k5),
            _ => None,
        }
    }
}

impl SPI {
    pub fn new(spi: spi::Instance) -> Self {
        SPI { spi }
    }

    /// Set up SPI peripheral for normal SPI mode, either flash or FPGA
    pub fn setup_spi(&self) {
        // 12MHz, SPI Mode 3 (CPOL=1 CPHA=1)
        write_reg!(
            spi,
            self.spi,
            CR1,
            BIDIMODE: Unidirectional,
            CRCEN: Disabled,
            RXONLY: FullDuplex,
            SSM: Enabled,
            SSI: SlaveNotSelected,
            LSBFIRST: MSBFirst,
            BR: Div4,
            MSTR: Master,
            CPOL: IdleHigh,
            CPHA: SecondEdge,
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

    /// Change SPI clock rate to one of the SPIClock variants
    pub fn set_clock(&self, clock: SPIClock) {
        modify_reg!(spi, self.spi, CR1, BR: clock as u32);
    }

    /// Wait for any pending operation then disable SPI
    pub fn disable(&self) {
        self.wait_busy();
        write_reg!(spi, self.spi, CR1, SPE: Disabled);
    }

    /// Transmit `data` and write the same number of bytes into `rxdata`.
    pub fn exchange(&self, dma: &DMA, txdata: &[u8], rxdata: &mut [u8]) {
        debug_assert!(rxdata.len() >= 64);

        // Set up DMA transfer (configures NDTR and MAR and enables streams)
        dma.spi1_enable(txdata, &mut rxdata[..txdata.len()]);

        // Start SPI transfer
        modify_reg!(spi, self.spi, CR1, SPE: Enabled);

        // Busy wait for RX DMA completion (at most 43µs)
        while dma.spi1_busy() {}

        // Disable SPI and DMA
        dma.spi1_disable();
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
