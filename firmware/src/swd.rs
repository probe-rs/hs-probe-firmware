// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use crate::bsp::{gpio::Pins, spi::SPI};
use num_enum::IntoPrimitive;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    BadParity,
    AckWait,
    AckFault,
    AckProtocol,
    AckUnknown(u8),
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(u8)]
#[derive(Copy, Clone, Debug, IntoPrimitive)]
pub enum DPRegister {
    DPIDR = 0,
    CTRLSTAT = 1,
    SELECT = 2,
    RDBUFF = 3,
}

pub struct SWD<'a> {
    spi: &'a SPI,
    pins: &'a Pins<'a>,

    wait_retries: usize,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum APnDP {
    DP = 0,
    AP = 1,
}

impl From<bool> for APnDP {
    fn from(x: bool) -> APnDP {
        if x {
            APnDP::AP
        } else {
            APnDP::DP
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum RnW {
    W = 0,
    R = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum ACK {
    OK = 0b001,
    WAIT = 0b010,
    FAULT = 0b100,
    PROTOCOL = 0b111,
}

impl ACK {
    pub fn try_ok(ack: u8) -> Result<()> {
        match ack {
            v if v == (ACK::OK as u8) => Ok(()),
            v if v == (ACK::WAIT as u8) => Err(Error::AckWait),
            v if v == (ACK::FAULT as u8) => Err(Error::AckFault),
            v if v == (ACK::PROTOCOL as u8) => Err(Error::AckProtocol),
            _ => Err(Error::AckUnknown(ack)),
        }
    }
}

impl<'a> SWD<'a> {
    pub fn new(spi: &'a SPI, pins: &'a Pins) -> Self {
        SWD {
            spi,
            pins,
            wait_retries: 8,
        }
    }

    pub fn set_clock(&self, max_frequency: u32) -> bool {
        if let Some(prescaler) = self.spi.calculate_prescaler(max_frequency) {
            self.spi.set_prescaler(prescaler);
            true
        } else {
            false
        }
    }

    pub fn spi_enable(&self) {
        self.spi.setup_swd();
    }

    pub fn spi_disable(&self) {
        self.spi.disable();
    }

    pub fn set_wait_retries(&mut self, wait_retries: usize) {
        self.wait_retries = wait_retries;
    }

    pub fn idle_low(&self) {
        self.spi.tx4(0x0);
    }

    pub fn read_dp(&self, a: u8) -> Result<u32> {
        self.read(APnDP::DP, a)
    }

    pub fn write_dp(&self, a: u8, data: u32) -> Result<()> {
        self.write(APnDP::DP, a, data)
    }

    pub fn read_ap(&self, a: u8) -> Result<u32> {
        self.read(APnDP::AP, a)
    }

    pub fn read(&self, apndp: APnDP, a: u8) -> Result<u32> {
        for _ in 0..self.wait_retries {
            match self.read_inner(apndp, a) {
                Err(Error::AckWait) => continue,
                x => return x,
            }
        }
        Err(Error::AckWait)
    }

    pub fn write(&self, apndp: APnDP, a: u8, data: u32) -> Result<()> {
        for _ in 0..self.wait_retries {
            match self.write_inner(apndp, a, data) {
                Err(Error::AckWait) => continue,
                x => return x,
            }
        }
        Err(Error::AckWait)
    }

    fn read_inner(&self, apndp: APnDP, a: u8) -> Result<u32> {
        let req = Self::make_request(apndp, RnW::R, a);
        self.spi.tx8(req);
        self.spi.wait_busy();
        self.spi.drain();
        self.pins.swd_rx();

        // 1 clock for turnaround and 3 for ACK
        let ack = self.spi.rx4() >> 1;
        match ACK::try_ok(ack as u8) {
            Ok(_) => (),
            Err(e) => {
                // On non-OK ACK, target has released the bus but
                // is still expecting a turnaround clock before
                // the next request, and we need to take over the bus.
                self.pins.swd_tx();
                self.idle_low();
                return Err(e);
            }
        }

        // Read 8x4=32 bits of data and 8x1=8 bits for parity+turnaround+trailing.
        // Doing a batch of 5 8-bit reads is the quickest option as we keep the FIFO hot.
        let (data, parity) = self.spi.swd_rdata_phase(self.pins);
        let parity = (parity & 1) as u32;

        // Back to driving SWDIO to ensure it doesn't float high
        self.pins.swd_tx();

        if parity == (data.count_ones() & 1) {
            Ok(data)
        } else {
            Err(Error::BadParity)
        }
    }

    fn write_inner(&self, apndp: APnDP, a: u8, data: u32) -> Result<()> {
        let req = Self::make_request(apndp, RnW::W, a);
        let parity = data.count_ones() & 1;

        self.spi.tx8(req);
        self.spi.wait_busy();
        self.spi.drain();
        self.pins.swd_rx();

        // 1 clock for turnaround and 3 for ACK and 1 for turnaround
        let ack = (self.spi.rx5() >> 1) & 0b111;
        self.pins.swd_tx();
        match ACK::try_ok(ack as u8) {
            Ok(_) => (),
            Err(e) => return Err(e),
        }

        // Write 8x4=32 bits of data and 8x1=8 bits for parity+trailing idle.
        // This way we keep the FIFO full and eliminate delays between words,
        // even at the cost of more trailing bits. We can't change DS to 4 bits
        // until the FIFO is empty, and waiting for that costs more time overall.
        // Additionally, many debug ports require a couple of clock cycles after
        // the parity bit of a write transaction to make the write effective.
        self.spi.swd_wdata_phase(data, parity as u8);
        self.spi.wait_busy();

        Ok(())
    }

    fn make_request(apndp: APnDP, rnw: RnW, a: u8) -> u8 {
        let req = 1 | ((apndp as u8) << 1) | ((rnw as u8) << 2) | (a << 3) | (1 << 7);
        let parity = (req.count_ones() & 1) as u8;
        req | (parity << 5)
    }
}
