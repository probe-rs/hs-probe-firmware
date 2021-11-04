// Copyright 2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::cmp::Ordering;
use stm32ral::usart;
use stm32ral::{modify_reg, read_reg, write_reg};

use super::dma::DMA;
use super::rcc::Clocks;

pub struct UART<'a> {
    uart: usart::Instance,
    dma: &'a DMA,
    buffer: [u8; 256],
    last_idx: usize,
    fck: u32,
}

impl<'a> UART<'a> {
    pub fn new(uart: usart::Instance, dma: &'a DMA) -> Self {
        UART {
            uart,
            dma,
            buffer: [0; 256],
            last_idx: 0,
            fck: 72_000_000,
        }
    }

    /// Set the UART peripheral clock speed, used for baud rate calculation.
    pub fn setup(&mut self, clocks: &Clocks) {
        self.fck = clocks.pclk2();
    }

    /// Begin UART reception into buffer.
    ///
    /// UART::poll must be called regularly after starting.
    pub fn start(&mut self) {
        self.last_idx = 0;
        write_reg!(usart, self.uart, CR3, DMAR: Enabled);
        write_reg!(
            usart,
            self.uart,
            CR1,
            OVER8: Oversampling8,
            RE: Enabled,
            UE: Enabled
        );
        self.dma.usart1_start(&mut self.buffer);
    }

    /// End UART reception.
    pub fn stop(&self) {
        self.dma.usart1_stop();
        modify_reg!(usart, self.uart, CR1, RE: Disabled);
    }

    /// Returns true if UART currently enabled
    pub fn is_active(&self) -> bool {
        read_reg!(usart, self.uart, CR1, RE == Enabled)
    }

    /// Return length of internal buffer
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Request a target baud rate. Returns actual baud rate set.
    pub fn set_baud(&self, baud: u32) -> u32 {
        // Find closest divider which is also an even integer >= 16.
        // The baud rate is (2*fck)/BRR.
        let mut div = (2*self.fck) / baud;
        div &= 0xffff_fffe;
        if div < 16 {
            div = 16;
        }

        // Write BRR value based on div.
        // Since we are OVERSAMPLE8, shift bottom 4 bits down by 1.
        let brr = (div & 0xffff_fff0) | ((div & 0xf) >> 1);
        write_reg!(usart, self.uart, BRR, brr);

        // Return actual baud rate
        (2*self.fck) / div
    }

    /// Fetch current number of bytes available.
    ///
    /// Subsequent calls to read() may return a different amount of data.
    pub fn bytes_available(&self) -> usize {
        let dma_idx = self.buffer.len() - self.dma.usart1_ndtr();
        if dma_idx >= self.last_idx {
            dma_idx - self.last_idx
        } else {
            (self.buffer.len() - self.last_idx) + dma_idx
        }
    }

    /// Read new UART data.
    ///
    /// Returns number of bytes written to buffer.
    ///
    /// Reads at most rx.len() new bytes, which may be less than what was received.
    /// Remaining data will be read on the next call, so long as the internal buffer
    /// doesn't overflow, which is not detected.
    pub fn read(&mut self, rx: &mut [u8]) -> usize {
        // See what index the DMA is going to write next, and copy out
        // all prior data. Even if the DMA writes new data while we're
        // processing we won't get out of sync and will handle the new
        // data next time read() is called.
        let dma_idx = self.buffer.len() - self.dma.usart1_ndtr();

        match dma_idx.cmp(&self.last_idx) {
            Ordering::Equal => {
                // No action required if no data has been received.
                0
            }
            Ordering::Less => {
                // Wraparound occurred:
                // Copy from last_idx to end, and from start to new dma_idx.
                let mut n1 = self.buffer.len() - self.last_idx;
                let mut n2 = dma_idx;
                let mut new_last_idx = dma_idx;

                // Ensure we don't overflow rx buffer
                if n1 > rx.len() {
                    n1 = rx.len();
                    n2 = 0;
                    new_last_idx = self.last_idx + n1;
                } else if (n1 + n2) > rx.len() {
                    n2 = rx.len() - n1;
                    new_last_idx = n2;
                }

                rx[..n1].copy_from_slice(&self.buffer[self.last_idx..self.last_idx + n1]);
                rx[n1..(n1 + n2)].copy_from_slice(&self.buffer[..n2]);

                self.last_idx = new_last_idx;
                n1 + n2
            }
            Ordering::Greater => {
                // New data, no wraparound:
                // Copy from last_idx to new dma_idx.
                let mut n = dma_idx - self.last_idx;

                // Ensure we don't overflow rx buffer
                if n > rx.len() {
                    n = rx.len();
                }

                rx[..n].copy_from_slice(&self.buffer[self.last_idx..self.last_idx + n]);

                self.last_idx += n;
                n
            }
        }
    }
}
