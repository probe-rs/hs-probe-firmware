// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::dma;
use stm32ral::{modify_reg, read_reg, write_reg};

/*
SPI1_RX: DMA2, stream 2, channel 3
SPI1_TX: DMA2, stream 3, channel 3
SPI2_RX: DMA1, stream 3, channel 0
SPI2_TX: DMA1, stream 4, channel 0
USART1_RX: DMA2, stream 5, channel 4
USART2_RX: DMA1, stream 5, channel 4
USART2_TX: DMA1, stream 6, channel 4
*/

const SPI_DR_OFFSET: u32 = 0x0C;
const UART_RDR_OFFSET: u32 = 0x24;
const UART_TDR_OFFSET: u32 = 0x28;

pub struct DMA {
    dma1: dma::Instance,
    dma2: dma::Instance,
}

impl DMA {
    pub fn new(dma1: dma::Instance, dma2: dma::Instance) -> Self {
        DMA { dma1, dma2 }
    }

    pub fn setup(&self) {
        // Set up DMA2 stream 2, channel 3 for SPI1_RX
        write_reg!(
            dma,
            self.dma2,
            CR2,
            CHSEL: 3,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR2,
            stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET
        );

        // Set up DMA2 stream 3, channel 3 for SPI1_TX
        write_reg!(
            dma,
            self.dma2,
            CR3,
            CHSEL: 3,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: MemoryToPeripheral,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR3,
            stm32ral::spi::SPI1 as u32 + SPI_DR_OFFSET
        );

        // Set up DMA1 stream 3, channel 0 for SPI2_RX
        write_reg!(
            dma,
            self.dma1,
            CR3,
            CHSEL: 0,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma1,
            PAR3,
            stm32ral::spi::SPI2 as u32 + SPI_DR_OFFSET
        );

        // Set up DMA1 stream 4, channel 0 for SPI2_TX
        write_reg!(
            dma,
            self.dma1,
            CR4,
            CHSEL: 0,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: MemoryToPeripheral,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma1,
            PAR4,
            stm32ral::spi::SPI2 as u32 + SPI_DR_OFFSET
        );

        // Set up DMA2 stream 5, channel 4 for USART1_RX
        write_reg!(
            dma,
            self.dma2,
            CR5,
            CHSEL: 4,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Enabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR5,
            stm32ral::usart::USART1 as u32 + UART_RDR_OFFSET
        );

        // Set up DMA1 stream 5, channel 4 for USART2_RX
        write_reg!(
            dma,
            self.dma1,
            CR5,
            CHSEL: 4,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Enabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma1,
            PAR5,
            stm32ral::usart::USART2 as u32 + UART_RDR_OFFSET
        );

        // Set up DMA1 stream 6, channel 4 for USART2_TX
        write_reg!(
            dma,
            self.dma1,
            CR6,
            CHSEL: 4,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: MemoryToPeripheral,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma1,
            PAR6,
            stm32ral::usart::USART2 as u32 + UART_TDR_OFFSET
        );
    }

    /// Sets up and enables a DMA transmit/receive for SPI1 (streams 2 and 3, channel 3)
    pub fn spi1_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma2,
            LIFCR,
            CTCIF2: Clear,
            CHTIF2: Clear,
            CTEIF2: Clear,
            CDMEIF2: Clear,
            CFEIF2: Clear,
            CTCIF3: Clear,
            CHTIF3: Clear,
            CTEIF3: Clear,
            CDMEIF3: Clear,
            CFEIF3: Clear
        );
        write_reg!(dma, self.dma2, NDTR2, rx.len() as u32);
        write_reg!(dma, self.dma2, NDTR3, tx.len() as u32);
        write_reg!(dma, self.dma2, M0AR2, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma2, M0AR3, tx.as_ptr() as u32);
        modify_reg!(dma, self.dma2, CR2, EN: Enabled);
        modify_reg!(dma, self.dma2, CR3, EN: Enabled);
    }

    /// Check if SPI1 transaction is still ongoing
    pub fn spi1_busy(&self) -> bool {
        read_reg!(dma, self.dma2, LISR, TCIF2 == NotComplete)
    }

    /// Stop SPI1 DMA
    pub fn spi1_disable(&self) {
        modify_reg!(dma, self.dma2, CR2, EN: Disabled);
        modify_reg!(dma, self.dma2, CR3, EN: Disabled);
    }

    /// Sets up and enables a DMA transmit/receive for SPI2 (streams 3 and 4, channel 0)
    pub fn spi2_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma1,
            LIFCR,
            CTCIF3: Clear,
            CHTIF3: Clear,
            CTEIF3: Clear,
            CDMEIF3: Clear,
            CFEIF3: Clear
        );
        write_reg!(
            dma,
            self.dma1,
            HIFCR,
            CTCIF4: Clear,
            CHTIF4: Clear,
            CTEIF4: Clear,
            CDMEIF4: Clear,
            CFEIF4: Clear
        );
        write_reg!(dma, self.dma1, NDTR3, rx.len() as u32);
        write_reg!(dma, self.dma1, NDTR4, tx.len() as u32);
        write_reg!(dma, self.dma1, M0AR3, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma1, M0AR4, tx.as_ptr() as u32);
        modify_reg!(dma, self.dma1, CR3, EN: Enabled);
        modify_reg!(dma, self.dma1, CR4, EN: Enabled);
    }

    /// Check if SPI2 transaction is still ongoing
    pub fn spi2_busy(&self) -> bool {
        read_reg!(dma, self.dma1, LISR, TCIF3 == NotComplete)
    }

    /// Stop SPI2 DMA
    pub fn spi2_disable(&self) {
        modify_reg!(dma, self.dma1, CR3, EN: Disabled);
        modify_reg!(dma, self.dma1, CR4, EN: Disabled);
    }

    /// Start USART1 reception into provided buffer
    pub fn usart1_start(&self, rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma2,
            HIFCR,
            CTCIF5: Clear,
            CHTIF5: Clear,
            CTEIF5: Clear,
            CDMEIF5: Clear,
            CFEIF5: Clear
        );
        write_reg!(dma, self.dma2, NDTR5, rx.len() as u32);
        write_reg!(dma, self.dma2, M0AR5, rx.as_mut_ptr() as u32);
        modify_reg!(dma, self.dma2, CR5, EN: Enabled);
    }

    /// Return how many bytes are left to transfer for USART1
    pub fn usart1_ndtr(&self) -> usize {
        read_reg!(dma, self.dma2, NDTR5) as usize
    }

    /// Stop USART1 DMA
    pub fn usart1_stop(&self) {
        modify_reg!(dma, self.dma2, CR5, EN: Disabled);
    }

    /// Start USART2 reception into provided buffer
    pub fn usart2_start_rx(&self, rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma1,
            HIFCR,
            CTCIF5: Clear,
            CHTIF5: Clear,
            CTEIF5: Clear,
            CDMEIF5: Clear,
            CFEIF5: Clear
        );
        write_reg!(dma, self.dma1, NDTR5, rx.len() as u32);
        write_reg!(dma, self.dma1, M0AR5, rx.as_mut_ptr() as u32);
        modify_reg!(dma, self.dma1, CR5, EN: Enabled);
    }

    /// Return how many bytes are left to transfer for USART2 RX
    pub fn usart2_rx_ndtr(&self) -> usize {
        read_reg!(dma, self.dma1, NDTR5) as usize
    }
    /// Return how many bytes are left to transfer for USART2 TX
    pub fn usart2_tx_ndtr(&self) -> usize {
        read_reg!(dma, self.dma1, NDTR6) as usize
    }

    /// Start a DMA transfer for USART2 TX
    pub fn usart2_start_tx_transfer(&self, tx: &[u8], len: usize) {
        write_reg!(
            dma,
            self.dma1,
            HIFCR,
            CTCIF6: Clear,
            CHTIF6: Clear,
            CTEIF6: Clear,
            CDMEIF6: Clear,
            CFEIF6: Clear
        );

        modify_reg!(dma, self.dma1, CR6, EN: Disabled);
        write_reg!(dma, self.dma1, NDTR6, len as u32);
        write_reg!(dma, self.dma1, M0AR6, tx.as_ptr() as u32);
        // This barrier guarantees that when the transfer starts,
        // any store done to RAM will be drained from the store
        // buffer of the M7.
        cortex_m::asm::dsb();
        modify_reg!(dma, self.dma1, CR6, EN: Enabled);
    }

    /// Stop USART2 DMA
    pub fn usart2_stop(&self) {
        modify_reg!(dma, self.dma1, CR5, EN: Disabled);
        modify_reg!(dma, self.dma1, CR6, EN: Disabled);
    }
}
