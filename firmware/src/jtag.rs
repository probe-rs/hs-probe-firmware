// Copyright 2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use crate::bsp::delay::Delay;
use crate::bsp::dma::DMA;
use crate::bsp::gpio::{Pin, Pins};
use crate::bsp::spi::SPI;
use crate::DAP2_PACKET_SIZE;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

struct JTAGPins<'a> {
    tms: &'a Pin<'a>,
    tck: &'a Pin<'a>,
    tdo: &'a Pin<'a>,
    tdi: &'a Pin<'a>,
}

#[allow(clippy::upper_case_acronyms)]
pub struct JTAG<'a> {
    spi: &'a SPI,
    dma: &'a DMA,
    pins: JTAGPins<'a>,
    delay: &'a Delay,
    half_period_ticks: AtomicU32,
    use_bitbang: AtomicBool,
}

impl<'a> JTAG<'a> {
    /// Create a new JTAG object from the provided Pins struct.
    pub fn new(spi: &'a SPI, dma: &'a DMA, pins: &'a Pins, delay: &'a Delay) -> Self {
        let jtag_pins = JTAGPins {
            tms: &pins.spi1_mosi,
            tck: &pins.spi2_clk,
            tdo: &pins.spi2_miso,
            tdi: &pins.spi2_mosi,
        };

        JTAG {
            spi,
            dma,
            pins: jtag_pins,
            delay,
            half_period_ticks: AtomicU32::new(10000),
            use_bitbang: AtomicBool::new(true),
        }
    }

    pub fn set_clock(&self, max_frequency: u32) {
        let period = self.delay.calc_period_ticks(max_frequency);
        self.half_period_ticks.store(period / 2, Ordering::SeqCst);

        if let Some(prescaler) = self.spi.calculate_prescaler(max_frequency) {
            self.spi.set_prescaler(prescaler);
            self.use_bitbang.store(false, Ordering::SeqCst);
        } else {
            self.use_bitbang.store(true, Ordering::SeqCst);
        }
    }

    pub fn spi_enable(&self) {
        self.spi.setup_jtag();
    }

    pub fn spi_disable(&self) {
        self.spi.disable();
    }

    #[inline(never)]
    pub fn tms_sequence(&self, data: &[u8], mut bits: usize) {
        self.bitbang_mode();

        let half_period_ticks = self.half_period_ticks.load(Ordering::SeqCst);
        let mut last = self.delay.get_current();
        last = self.delay.delay_ticks_from_last(half_period_ticks, last);

        for byte in data {
            let mut byte = *byte;
            let frame_bits = core::cmp::min(bits, 8);
            for _ in 0..frame_bits {
                let bit = byte & 1;
                byte >>= 1;

                self.pins.tms.set_bool(bit != 0);
                self.pins.tck.set_low();
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
                self.pins.tck.set_high();
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
            }
            bits -= frame_bits;
        }
    }

    /// Handle a sequence request. The request data follows the CMSIS-DAP
    /// DAP_JTAG_Sequence command:
    /// * First byte contains the number of sequences, then
    /// * First byte of each sequence contains:
    ///     * Bits 5..0: Number of clock cycles, where 0 means 64 cycles
    ///     * Bit 6: TMS value
    ///     * Bit 7: TDO capture enable
    /// * Subsequent bytes of each sequence contain TDI data, one bit per
    ///   clock cycle, with the final byte padded. Data is transmitted from
    ///   successive bytes, least significant bit first.
    ///
    /// Captured TDO data is written least significant bit first to successive
    /// bytes of `rxbuf`, which must be long enough for the requested capture,
    /// or conservatively as long as `data`.
    /// The final byte of TDO data for each sequence is padded, in other words,
    /// as many TDO bytes will be returned as there were TDI bytes in sequences
    /// with capture enabled.
    ///
    /// Returns the number of bytes of rxbuf which were written to.
    pub fn sequences(&self, data: &[u8], rxbuf: &mut [u8]) -> usize {
        // Read request header containing number of sequences.
        if data.is_empty() {
            return 0;
        };
        let mut nseqs = data[0];
        let mut data = &data[1..];
        let mut rxidx = 0;

        // Sanity check
        if nseqs == 0 || data.is_empty() {
            return 0;
        }

        let half_period_ticks = self.half_period_ticks.load(Ordering::SeqCst);
        self.delay.delay_ticks(half_period_ticks);

        // Process alike sequences in one shot
        // This
        if !self.use_bitbang.load(Ordering::SeqCst) {
            let mut buffer = [0u8; DAP2_PACKET_SIZE as usize];
            let mut buffer_idx = 0;
            let transfer_type = data[0] & 0b1100_0000;
            while nseqs > 0 {
                // Read header byte for this sequence.
                if data.is_empty() {
                    break;
                };
                let header = data[0];
                if (header & 0b1100_0000) != transfer_type {
                    // This sequence can't be processed in the same way
                    break;
                }
                let nbits = header & 0b0011_1111;
                if nbits & 7 != 0 {
                    // We can handle only 8*N bit sequences here
                    break;
                }
                let nbits = if nbits == 0 { 64 } else { nbits as usize };
                let nbytes = Self::bytes_for_bits(nbits);

                if data.len() < (nbytes + 1) {
                    break;
                };
                data = &data[1..];

                buffer[buffer_idx..buffer_idx + nbytes].copy_from_slice(&data[..nbytes]);
                buffer_idx += nbytes;
                nseqs -= 1;
                data = &data[nbytes..];
            }
            if buffer_idx > 0 {
                let capture = transfer_type & 0b1000_0000;
                let tms = transfer_type & 0b0100_0000;

                // Set TMS for this transfer.
                self.pins.tms.set_bool(tms != 0);

                self.spi_mode();
                self.spi
                    .jtag_exchange(self.dma, &buffer[..buffer_idx], &mut rxbuf[rxidx..]);
                if capture != 0 {
                    rxidx += buffer_idx;
                }
                // Set TDI GPIO to the last bit the SPI peripheral transmitted,
                // to prevent it changing state when we set it to an output.
                self.pins.tdi.set_bool((buffer[buffer_idx - 1] >> 7) != 0);
                self.bitbang_mode();
                self.spi.disable();
            }
        }

        // Process each sequence.
        for _ in 0..nseqs {
            // Read header byte for this sequence.
            if data.is_empty() {
                break;
            };
            let header = data[0];
            data = &data[1..];
            let capture = header & 0b1000_0000;
            let tms = header & 0b0100_0000;
            let nbits = header & 0b0011_1111;
            let nbits = if nbits == 0 { 64 } else { nbits as usize };
            let nbytes = Self::bytes_for_bits(nbits);
            if data.len() < nbytes {
                break;
            };

            // Split data into TDI data for this sequence and data for remaining sequences.
            let tdi = &data[..nbytes];
            data = &data[nbytes..];

            // Set TMS for this transfer.
            self.pins.tms.set_bool(tms != 0);

            // Run one transfer, either read-write or write-only.
            if capture != 0 {
                self.transfer_rw(nbits, tdi, &mut rxbuf[rxidx..]);
                rxidx += nbytes;
            } else {
                self.transfer_wo(nbits, tdi);
            }
        }

        rxidx
    }

    /// Write-only JTAG transfer without capturing TDO.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    #[inline(never)]
    fn transfer_wo(&self, n: usize, tdi: &[u8]) {
        let half_period_ticks = self.half_period_ticks.load(Ordering::SeqCst);
        let mut last = self.delay.get_current();

        for (byte_idx, byte) in tdi.iter().enumerate() {
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx * 8 + bit_idx == n {
                    return;
                }

                // Set TDI and toggle TCK.
                self.pins.tdi.set_bool(byte & (1 << bit_idx) != 0);
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
                self.pins.tck.set_high();
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
                self.pins.tck.set_low();
            }
        }
    }

    /// Read-write JTAG transfer, with TDO capture.
    ///
    /// Writes `n` bits from successive bytes of `tdi`, LSbit first.
    /// Captures `n` bits from TDO and writes into successive bytes of `tdo`, LSbit first.
    #[inline(never)]
    fn transfer_rw(&self, n: usize, tdi: &[u8], tdo: &mut [u8]) {
        let half_period_ticks = self.half_period_ticks.load(Ordering::SeqCst);
        let mut last = self.delay.get_current();

        for (byte_idx, (tdi, tdo)) in tdi.iter().zip(tdo.iter_mut()).enumerate() {
            *tdo = 0;
            for bit_idx in 0..8 {
                // Stop after transmitting `n` bits.
                if byte_idx * 8 + bit_idx == n {
                    return;
                }

                // We set TDI half a period before the clock rising edge where it is sampled
                // by the target, and we sample TDO immediately before the clock falling edge
                // where it is updated by the target.
                self.pins.tdi.set_bool(tdi & (1 << bit_idx) != 0);
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
                self.pins.tck.set_high();
                last = self.delay.delay_ticks_from_last(half_period_ticks, last);
                if self.pins.tdo.is_high() {
                    *tdo |= 1 << bit_idx;
                }
                self.pins.tck.set_low();
            }
        }
    }

    /// Compute required number of bytes to store a number of bits.
    fn bytes_for_bits(bits: usize) -> usize {
        (bits + 7) / 8
    }

    fn bitbang_mode(&self) {
        self.pins.tdo.set_mode_input();
        self.pins.tdi.set_mode_output();
        self.pins.tck.set_low().set_mode_output();
    }

    fn spi_mode(&self) {
        self.pins.tdo.set_mode_alternate();
        self.pins.tdi.set_mode_alternate();
        self.pins.tck.set_mode_alternate();
    }
}
