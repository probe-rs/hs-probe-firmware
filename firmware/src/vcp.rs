// Copyright 2019-2022 Alexis Marquet
// Dual licensed under the Apache 2.0 and MIT licenses.

use core::cmp::Ordering;

use crate::{
    bsp::{dma::DMA, gpio::Pins, rcc::Clocks},
    VCP_PACKET_SIZE,
};

use stm32ral::usart;
use stm32ral::{modify_reg, write_reg};
use usbd_serial::{ParityType, StopBits};

/// UART configuration struct
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct VcpConfig {
    pub stop_bits: StopBits,
    pub data_bits: u8,
    pub parity_type: ParityType,
    pub data_rate: u32,
}

impl Default for VcpConfig {
    fn default() -> Self {
        VcpConfig {
            stop_bits: StopBits::One,
            data_bits: 8,
            parity_type: ParityType::None,
            data_rate: 8_000,
        }
    }
}

pub struct VCP<'a> {
    uart: usart::Instance,
    pins: &'a Pins<'a>,
    dma: &'a DMA,
    rx_buffer: [u8; VCP_PACKET_SIZE as usize],
    tx_buffer: [u8; VCP_PACKET_SIZE as usize],
    last_idx_rx: usize,
    last_idx_tx: usize,
    fck: u32,
}

impl<'a> VCP<'a> {
    pub fn new(uart: usart::Instance, pins: &'a Pins, dma: &'a DMA) -> Self {
        VCP {
            uart,
            pins,
            dma,
            rx_buffer: [0; VCP_PACKET_SIZE as usize],
            tx_buffer: [0; VCP_PACKET_SIZE as usize],
            last_idx_rx: 0,
            last_idx_tx: 0,
            fck: 72_000_000,
        }
    }

    /// Call with the system clock speeds to configure peripherals that require timing information.
    ///
    /// Currently this only configures the pins & DMA RX
    pub fn setup(&mut self, clocks: &Clocks) {
        self.fck = clocks.pclk1();

        self.pins.usart2_tx.set_ospeed_veryhigh();
        self.pins.usart2_tx.set_otype_pushpull();
        self.pins.usart2_tx.set_pull_up();
        self.pins.usart2_tx.set_mode_alternate();
        self.pins.usart2_tx.set_af(7);

        self.pins.usart2_rx.set_ospeed_veryhigh();
        self.pins.usart2_rx.set_otype_pushpull();
        self.pins.usart2_rx.set_pull_up();
        self.pins.usart2_rx.set_mode_alternate();
        self.pins.usart2_rx.set_af(7);

        self.dma.usart2_start_rx(&mut self.rx_buffer);
    }

    /// Start the VCP function.
    ///
    /// This enables both TX & RX.
    pub fn start(&mut self) {
        self.last_idx_rx = 0;
        self.last_idx_tx = 0;
        write_reg!(usart, self.uart, CR3, DMAR: Enabled, DMAT: Enabled);

        write_reg!(
            usart,
            self.uart,
            CR1,
            OVER8: Oversampling8,
            RE: Enabled,
            TE: Enabled,
            UE: Enabled
        );
    }

    /// Disable UART.
    pub fn stop(&self) {
        modify_reg!(
            usart,
            self.uart,
            CR1,
            RE: Disabled,
            TE: Disabled,
            UE: Disabled
        );
    }

    /// Fetch current number of bytes available.
    ///
    /// Subsequent calls to read() may return a different amount of data.
    pub fn rx_bytes_available(&self) -> usize {
        // length of the buffer minus the remainder of the dma transfer
        let dma_idx = self.rx_buffer.len() - self.dma.usart2_rx_ndtr();
        if dma_idx >= self.last_idx_rx {
            dma_idx - self.last_idx_rx
        } else {
            (self.rx_buffer.len() - self.last_idx_rx) + dma_idx
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
        let dma_idx = self.rx_buffer.len() - self.dma.usart2_rx_ndtr();
        match dma_idx.cmp(&self.last_idx_rx) {
            Ordering::Equal => {
                // No action required if no data has been received.
                0
            }
            Ordering::Less => {
                // Wraparound occurred:
                // Copy from last_idx to end, and from start to new dma_idx.
                let mut n1 = self.rx_buffer.len() - self.last_idx_rx;
                let mut n2 = dma_idx;
                let mut new_last_idx = dma_idx;

                // Ensure we don't overflow rx buffer
                if n1 > rx.len() {
                    n1 = rx.len();
                    n2 = 0;
                    new_last_idx = self.last_idx_rx + n1;
                } else if (n1 + n2) > rx.len() {
                    n2 = rx.len() - n1;
                    new_last_idx = n2;
                }

                rx[..n1].copy_from_slice(&self.rx_buffer[self.last_idx_rx..self.last_idx_rx + n1]);
                rx[n1..(n1 + n2)].copy_from_slice(&self.rx_buffer[..n2]);

                self.last_idx_rx = new_last_idx;
                n1 + n2
            }
            Ordering::Greater => {
                // New data, no wraparound:
                // Copy from last_idx to new dma_idx.
                let mut n = dma_idx - self.last_idx_rx;

                // Ensure we don't overflow rx buffer
                if n > rx.len() {
                    n = rx.len();
                }

                rx[..n].copy_from_slice(&self.rx_buffer[self.last_idx_rx..self.last_idx_rx + n]);

                self.last_idx_rx += n;
                n
            }
        }
    }

    /// Setup the USART line config.
    ///
    /// This should be done between a `stop()` and a `start` call since
    /// configuring this requires the UE bit to be `0b0`.
    pub fn set_config(&mut self, coding: VcpConfig) {
        // Find closest divider which is also an even integer >= 16.
        // The baud rate is (2*fck)/BRR.
        let mut div = (2 * self.fck) / coding.data_rate;
        div &= 0xffff_fffe;
        if div < 16 {
            div = 16;
        }

        // Write BRR value based on div.
        // Since we are OVERSAMPLE8, shift bottom 4 bits down by 1.
        let brr = (div & 0xffff_fff0) | ((div & 0xf) >> 1);
        write_reg!(usart, self.uart, BRR, brr);

        // configure data bits
        match coding.data_bits {
            7 => modify_reg!(usart, self.uart, CR1, M1: 1, M0: 0),
            8 => modify_reg!(usart, self.uart, CR1, M1: 0, M0: 0),
            9 => modify_reg!(usart, self.uart, CR1, M1: 0, M0: 1),
            _ => panic!(),
        }

        // configure stop bits
        match coding.stop_bits {
            StopBits::One => modify_reg!(usart, self.uart, CR2, STOP: 0b00),
            StopBits::OnePointFive => modify_reg!(usart, self.uart, CR2, STOP: 0b11),
            StopBits::Two => modify_reg!(usart, self.uart, CR2, STOP: 0b10),
        }

        // configure parity type
        match coding.parity_type {
            ParityType::None => modify_reg!(usart, self.uart, CR1, PCE: 0),
            ParityType::Odd => modify_reg!(usart, self.uart, CR1, PCE:1, PS: 1),
            ParityType::Event => modify_reg!(usart, self.uart, CR1, PCE:1, PS: 0),
            ParityType::Mark => (),  // unsupported?
            ParityType::Space => (), // unsupported?
        }
    }

    /// Check state of TX Dma transfer
    pub fn is_tx_idle(&self) -> bool {
        self.dma.usart2_tx_ndtr() == 0
    }
    /// Start DMA transfer from buffer to TX Shift register.
    pub fn write(&mut self, tx: &[u8], len: usize) {
        self.tx_buffer[0..len].copy_from_slice(&tx);
        self.dma.usart2_start_tx_transfer(&self.tx_buffer, len);
    }
}
