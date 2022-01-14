// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use crate::{
    bsp::{gpio::Pins, uart::UART, rcc::Clocks},
    jtag, swd, DAP1_PACKET_SIZE, DAP2_PACKET_SIZE,
};
use core::convert::{TryFrom, TryInto};
use num_enum::{IntoPrimitive, TryFromPrimitive};

#[derive(Copy, Clone)]
pub enum DAPVersion {
    V1,
    V2,
}

#[derive(Copy, Clone, TryFromPrimitive, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum Command {
    // General Commands
    DAP_Info = 0x00,
    DAP_HostStatus = 0x01,
    DAP_Connect = 0x02,
    DAP_Disconnect = 0x03,
    DAP_WriteABORT = 0x08,
    DAP_Delay = 0x09,
    DAP_ResetTarget = 0x0A,

    // Common SWD/JTAG Commands
    DAP_SWJ_Pins = 0x10,
    DAP_SWJ_Clock = 0x11,
    DAP_SWJ_Sequence = 0x12,

    // SWD Commands
    DAP_SWD_Configure = 0x13,
    // DAP_SWD_Sequence = 0x1D,

    // SWO Commands
    DAP_SWO_Transport = 0x17,
    DAP_SWO_Mode = 0x18,
    DAP_SWO_Baudrate = 0x19,
    DAP_SWO_Control = 0x1A,
    DAP_SWO_Status = 0x1B,
    DAP_SWO_ExtendedStatus = 0x1E,
    DAP_SWO_Data = 0x1C,

    // JTAG Commands
    DAP_JTAG_Sequence = 0x14,
    // DAP_JTAG_Configure = 0x15,
    // DAP_JTAG_IDCODE = 0x16,

    // Transfer Commands
    DAP_TransferConfigure = 0x04,
    DAP_Transfer = 0x05,
    DAP_TransferBlock = 0x06,
    DAP_TransferAbort = 0x07,

    // Atomic Commands
    // DAP_ExecuteCommands = 0x7F,
    // DAP_QueueCommands = 0x7E,

    // Unimplemented Command Response
    Unimplemented = 0xFF,
}

#[derive(Copy, Clone, IntoPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum ResponseStatus {
    DAP_OK = 0x00,
    DAP_ERROR = 0xFF,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[allow(non_camel_case_types)]
#[repr(u8)]
enum DAPInfoID {
    VendorID = 0x01,
    ProductID = 0x02,
    SerialNumber = 0x03,
    FirmwareVersion = 0x04,
    TargetVendor = 0x05,
    TargetName = 0x06,
    Capabilities = 0xF0,
    TestDomainTimer = 0xF1,
    SWOTraceBufferSize = 0xFD,
    MaxPacketCount = 0xFE,
    MaxPacketSize = 0xFF,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum HostStatusType {
    Connect = 0,
    Running = 1,
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum ConnectPort {
    Default = 0,
    SWD = 1,
    JTAG = 2,
}

#[repr(u8)]
enum ConnectPortResponse {
    Failed = 0,
    SWD = 1,
    JTAG = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOTransport {
    None = 0,
    DAPCommand = 1,
    USBEndpoint = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOMode {
    Off = 0,
    UART = 1,
    Manchester = 2,
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum SWOControl {
    Stop = 0,
    Start = 1,
}

struct Request<'a> {
    command: Command,
    data: &'a [u8],
}

impl<'a> Request<'a> {
    /// Returns None if the report is empty
    pub fn from_report(report: &'a [u8]) -> Option<Self> {
        let (command, data) = report.split_first()?;

        let command = (*command).try_into().unwrap_or(Command::Unimplemented);

        Some(Request { command, data })
    }

    pub fn next_u8(&mut self) -> u8 {
        let value = self.data[0];
        self.data = &self.data[1..];
        value
    }

    pub fn next_u16(&mut self) -> u16 {
        let value = u16::from_le_bytes(self.data[0..2].try_into().unwrap());
        self.data = &self.data[2..];
        value
    }

    pub fn next_u32(&mut self) -> u32 {
        let value = u32::from_le_bytes(self.data[0..4].try_into().unwrap());
        self.data = &self.data[4..];
        value
    }

    pub fn rest(self) -> &'a [u8] {
        &self.data
    }
}

struct ResponseWriter<'a> {
    buf: &'a mut [u8],
    idx: usize,
}

impl<'a> ResponseWriter<'a> {
    pub fn new(command: Command, buf: &'a mut [u8]) -> Self {
        buf[0] = command as u8;
        ResponseWriter { buf, idx: 1 }
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buf[self.idx] = value;
        self.idx += 1;
    }

    pub fn write_u16(&mut self, value: u16) {
        let value = value.to_le_bytes();
        self.buf[self.idx..self.idx + 2].copy_from_slice(&value);
        self.idx += 2;
    }

    pub fn write_u32(&mut self, value: u32) {
        let value = value.to_le_bytes();
        self.buf[self.idx..self.idx + 4].copy_from_slice(&value);
        self.idx += 4;
    }

    pub fn write_slice(&mut self, data: &[u8]) {
        self.buf[self.idx..self.idx + data.len()].copy_from_slice(&data);
        self.idx += data.len();
    }

    pub fn write_ok(&mut self) {
        self.write_u8(ResponseStatus::DAP_OK.into());
    }

    pub fn write_err(&mut self) {
        self.write_u8(ResponseStatus::DAP_ERROR.into());
    }

    pub fn write_u8_at(&mut self, idx: usize, value: u8) {
        self.buf[idx] = value;
    }

    pub fn write_u16_at(&mut self, idx: usize, value: u16) {
        let value = value.to_le_bytes();
        self.buf[idx..idx + 2].copy_from_slice(&value);
    }

    pub fn mut_at(&mut self, idx: usize) -> &mut u8 {
        &mut self.buf[idx]
    }

    pub fn read_u8_at(&self, idx: usize) -> u8 {
        self.buf[idx]
    }

    pub fn remaining(&mut self) -> &mut [u8] {
        &mut self.buf[self.idx..]
    }

    pub fn skip(&mut self, n: usize) {
        self.idx += n;
    }
}

enum DAPMode {
    SWD,
    JTAG,
}

pub struct DAP<'a> {
    swd: swd::SWD<'a>,
    jtag: jtag::JTAG<'a>,
    uart: &'a mut UART<'a>,
    pins: &'a Pins<'a>,
    mode: Option<DAPMode>,
    swo_streaming: bool,
    match_retries: usize,
}

impl<'a> DAP<'a> {
    pub fn new(
        swd: swd::SWD<'a>,
        jtag: jtag::JTAG<'a>,
        uart: &'a mut UART<'a>,
        pins: &'a Pins,
    ) -> Self {
        DAP {
            swd,
            jtag,
            uart,
            pins,
            mode: None,
            swo_streaming: false,
            match_retries: 5,
        }
    }

    /// Call with the system clock speeds to configure peripherals that require timing information.
    ///
    /// Currently this only configures the SWO USART baud rate calculation.
    pub fn setup(&mut self, clocks: &Clocks) {
        self.uart.setup(clocks);
    }

    /// Process a new CMSIS-DAP command from `report`.
    ///
    /// Returns number of bytes written to response buffer.
    pub fn process_command(
        &mut self,
        report: &[u8],
        rbuf: &mut [u8],
        version: DAPVersion,
    ) -> usize {
        let req = match Request::from_report(report) {
            Some(req) => req,
            None => return 0,
        };

        let resp = &mut ResponseWriter::new(req.command, rbuf);

        match req.command {
            Command::DAP_Info => self.process_info(req, resp, version),
            Command::DAP_HostStatus => self.process_host_status(req, resp),
            Command::DAP_Connect => self.process_connect(req, resp),
            Command::DAP_Disconnect => self.process_disconnect(req, resp),
            Command::DAP_WriteABORT => self.process_write_abort(req, resp),
            Command::DAP_Delay => self.process_delay(req, resp),
            Command::DAP_ResetTarget => self.process_reset_target(req, resp),
            Command::DAP_SWJ_Pins => self.process_swj_pins(req, resp),
            Command::DAP_SWJ_Clock => self.process_swj_clock(req, resp),
            Command::DAP_SWJ_Sequence => self.process_swj_sequence(req, resp),
            Command::DAP_SWD_Configure => self.process_swd_configure(req, resp),
            Command::DAP_SWO_Transport => self.process_swo_transport(req, resp),
            Command::DAP_SWO_Mode => self.process_swo_mode(req, resp),
            Command::DAP_SWO_Baudrate => self.process_swo_baudrate(req, resp),
            Command::DAP_SWO_Control => self.process_swo_control(req, resp),
            Command::DAP_SWO_Status => self.process_swo_status(req, resp),
            Command::DAP_SWO_ExtendedStatus => self.process_swo_extended_status(req, resp),
            Command::DAP_SWO_Data => self.process_swo_data(req, resp),
            Command::DAP_JTAG_Sequence => self.process_jtag_sequence(req, resp),
            Command::DAP_TransferConfigure => self.process_transfer_configure(req, resp),
            Command::DAP_Transfer => self.process_transfer(req, resp),
            Command::DAP_TransferBlock => self.process_transfer_block(req, resp),
            Command::DAP_TransferAbort => {
                self.process_transfer_abort();
                // Do not send a response for transfer abort commands
                return 0;
            }
            Command::Unimplemented => {}
        }

        resp.idx
    }

    /// Returns true if SWO streaming is currently active.
    pub fn is_swo_streaming(&self) -> bool {
        self.uart.is_active() && self.swo_streaming
    }

    /// Polls the UART buffer for new SWO data, returning
    /// number of bytes written to buffer.
    pub fn read_swo(&mut self, buf: &mut [u8]) -> usize {
        self.uart.read(buf)
    }

    fn process_info(&mut self, mut req: Request, resp: &mut ResponseWriter, version: DAPVersion) {
        match DAPInfoID::try_from(req.next_u8()) {
            // Return 0-length string for VendorID, ProductID, SerialNumber
            // to indicate they should be read from USB descriptor instead
            Ok(DAPInfoID::VendorID) => resp.write_u8(0),
            Ok(DAPInfoID::ProductID) => resp.write_u8(0),
            Ok(DAPInfoID::SerialNumber) => resp.write_u8(0),
            // Return git version as firmware version
            Ok(DAPInfoID::FirmwareVersion) => {
                resp.write_u8(crate::GIT_VERSION.len() as u8);
                resp.write_slice(crate::GIT_VERSION.as_bytes());
            }
            // Return 0-length string for TargetVendor and TargetName to indicate
            // unknown target device.
            Ok(DAPInfoID::TargetVendor) => resp.write_u8(0),
            Ok(DAPInfoID::TargetName) => resp.write_u8(0),
            Ok(DAPInfoID::Capabilities) => {
                resp.write_u8(1);
                // Bit 0: SWD supported
                // Bit 1: JTAG supported
                // Bit 2: SWO UART supported
                // Bit 3: SWO Manchester not supported
                // Bit 4: Atomic commands not supported
                // Bit 5: Test Domain Timer not supported
                // Bit 6: SWO Streaming Trace supported
                resp.write_u8(0b0100_0111);
            }
            Ok(DAPInfoID::SWOTraceBufferSize) => {
                resp.write_u8(4);
                resp.write_u32(self.uart.buffer_len() as u32);
            }
            Ok(DAPInfoID::MaxPacketCount) => {
                resp.write_u8(1);
                // Maximum of one packet at a time
                resp.write_u8(1);
            }
            Ok(DAPInfoID::MaxPacketSize) => {
                resp.write_u8(2);
                match version {
                    DAPVersion::V1 => {
                        // Maximum of 64 bytes per packet
                        resp.write_u16(DAP1_PACKET_SIZE);
                    }
                    DAPVersion::V2 => {
                        // Maximum of 512 bytes per packet
                        resp.write_u16(DAP2_PACKET_SIZE);
                    }
                }
            }
            _ => resp.write_u8(0),
        }
    }

    fn process_host_status(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let status_type = req.next_u8();
        let status_status = req.next_u8();
        // Use HostStatus to set our LED when host is connected to target
        if let Ok(HostStatusType::Connect) = HostStatusType::try_from(status_type) {
            match status_status {
                0 => {
                    self.pins.led_red.set_low();
                    self.pins.led_green.set_high();
                }
                1 => {
                    self.pins.led_red.set_high();
                    self.pins.led_green.set_low();
                }
                _ => (),
            }
        }
        resp.write_u8(0);
    }

    fn process_connect(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let port = req.next_u8();
        match ConnectPort::try_from(port) {
            Ok(ConnectPort::Default) | Ok(ConnectPort::SWD) => {
                self.pins.swd_mode();
                self.swd.spi_enable();
                self.mode = Some(DAPMode::SWD);
                resp.write_u8(ConnectPortResponse::SWD as u8);
            }
            Ok(ConnectPort::JTAG) => {
                self.pins.jtag_mode();
                self.jtag.spi_enable();
                self.mode = Some(DAPMode::JTAG);
                resp.write_u8(ConnectPortResponse::JTAG as u8);
            }
            _ => {
                resp.write_u8(ConnectPortResponse::Failed as u8);
            }
        }
    }

    fn process_disconnect(&mut self, _req: Request, resp: &mut ResponseWriter) {
        self.pins.high_impedance_mode();
        self.mode = None;
        self.swd.spi_disable();
        self.jtag.spi_disable();
        resp.write_ok();
    }

    fn process_write_abort(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        if self.mode.is_none() {
            resp.write_err();
            return;
        }
        let _idx = req.next_u8();
        let word = req.next_u32();
        match self.swd.write_dp(0x00, word) {
            Ok(_) => resp.write_ok(),
            Err(_) => resp.write_err(),
        }
    }

    fn process_delay(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let delay = req.next_u16() as u32;
        cortex_m::asm::delay(48 * delay);
        resp.write_ok();
    }

    fn process_reset_target(&mut self, _req: Request, resp: &mut ResponseWriter) {
        resp.write_ok();
        // "No device specific reset sequence is implemented"
        resp.write_u8(0);
    }

    fn process_swj_pins(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let output = req.next_u8();
        let mask = req.next_u8();
        let wait = req.next_u32();

        const SWCLK_POS: u8 = 0;
        const SWDIO_POS: u8 = 1;
        const TDI_POS: u8 = 2;
        const TDO_POS: u8 = 3;
        const NTRST_POS: u8 = 5;
        const NRESET_POS: u8 = 7;

        match self.mode {
            Some(DAPMode::SWD) => {
                // In SWD mode, use SPI1 MOSI and CLK for SWDIO/TMS and SWCLK/TCK.
                // Between transfers those pins are in SPI alternate mode, so swap them
                // to output to manually set them. They'll be reset to SPI mode by the
                // next transfer command.
                if mask & (1 << SWDIO_POS) != 0 {
                    self.pins.spi1_mosi.set_mode_output();
                    self.pins.spi1_mosi.set_bool(output & (1 << SWDIO_POS) != 0);
                }
                if mask & (1 << SWCLK_POS) != 0 {
                    self.pins.spi1_clk.set_mode_output();
                    self.pins.spi1_clk.set_bool(output & (1 << SWCLK_POS) != 0);
                }
            }
            Some(DAPMode::JTAG) => {
                // In JTAG mode, use SPI1 MOSI and SPI2 SLK for SWDIO/TMS and SWCLK/TCK,
                // and SPI2 MOSI for TDI. Between transfers these pins are already in GPIO
                // mode, so we don't need to change them.
                //
                // TDO is an input pin for JTAG and is ignored to match the DAPLink implementation.
                if mask & (1 << SWDIO_POS) != 0 {
                    self.pins.spi1_mosi.set_bool(output & (1 << SWDIO_POS) != 0);
                }
                if mask & (1 << SWCLK_POS) != 0 {
                    self.pins.spi2_clk.set_bool(output & (1 << SWCLK_POS) != 0);
                }
                if mask & (1 << TDI_POS) != 0 {
                    self.pins.spi2_mosi.set_bool(output & (1 << TDI_POS) != 0);
                }
            }

            // When not in any mode, ignore JTAG/SWD pins entirely.
            None => ()
        };

        // Always allow setting the nRESET pin, which is always in output open-drain mode.
        if mask & (1 << NRESET_POS) != 0 {
            self.pins.reset.set_bool(output & (1 << NRESET_POS) != 0);
        }

        // Delay required time in µs (approximate delay).
        cortex_m::asm::delay(42 * wait);

        // Read and return pin state
        let state = ((self.pins.spi1_clk.get_state() as u8) << SWCLK_POS)
            | ((self.pins.spi1_miso.get_state() as u8) << SWDIO_POS)
            | ((self.pins.spi2_mosi.get_state() as u8) << TDI_POS)
            | ((self.pins.spi2_miso.get_state() as u8) << TDO_POS)
            | (1 << NTRST_POS)
            | ((self.pins.reset.get_state() as u8) << NRESET_POS);
        resp.write_u8(state);
    }

    fn process_swj_clock(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let clock = req.next_u32();

        self.jtag.set_clock(clock);
        let valid = self.swd.set_clock(clock);
        if valid {
            resp.write_ok();
        } else {
            resp.write_err();
        }
    }

    fn process_swj_sequence(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let nbits: usize = match req.next_u8() {
            // CMSIS-DAP says 0 means 256 bits
            0 => 256,
            // Other integers are normal.
            n => n as usize,
        };

        let payload = req.rest();
        let nbytes = (nbits + 7) / 8;
        let seq = if nbytes <= payload.len() {
            &payload[..nbytes]
        } else {
            resp.write_err();
            return;
        };

        match self.mode {
            Some(DAPMode::SWD) => {
                self.swd.tx_sequence(seq, nbits);
            }
            Some(DAPMode::JTAG) => {
                self.jtag.tms_sequence(seq, nbits);
            }
            None => {
                resp.write_err();
                return;
            }
        }

        resp.write_ok();
    }

    fn process_swd_configure(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let config = req.next_u8();
        let clk_period = config & 0b011;
        let always_data = (config & 0b100) != 0;
        if clk_period == 0 && !always_data {
            resp.write_ok();
        } else {
            resp.write_err();
        }
    }

    fn process_swo_transport(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let transport = req.next_u8();
        match SWOTransport::try_from(transport) {
            Ok(SWOTransport::None) => {
                self.swo_streaming = false;
                resp.write_ok();
            }
            Ok(SWOTransport::DAPCommand) => {
                self.swo_streaming = false;
                resp.write_ok();
            }
            Ok(SWOTransport::USBEndpoint) => {
                self.swo_streaming = true;
                resp.write_ok();
            }
            _ => resp.write_err(),
        }
    }

    fn process_swo_mode(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let mode = req.next_u8();
        match SWOMode::try_from(mode) {
            Ok(SWOMode::Off) => {
                resp.write_ok();
            }
            Ok(SWOMode::UART) => {
                resp.write_ok();
            }
            _ => resp.write_err(),
        }
    }

    fn process_swo_baudrate(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let target = req.next_u32();
        let actual = self.uart.set_baud(target);
        resp.write_u32(actual);
    }

    fn process_swo_control(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        match SWOControl::try_from(req.next_u8()) {
            Ok(SWOControl::Stop) => {
                self.uart.stop();
                resp.write_ok();
            }
            Ok(SWOControl::Start) => {
                self.uart.start();
                resp.write_ok();
            }
            _ => resp.write_err(),
        }
    }

    fn process_swo_status(&mut self, _req: Request, resp: &mut ResponseWriter) {
        // Trace status:
        // Bit 0: trace capture active
        // Bit 6: trace stream error (always written as 0)
        // Bit 7: trace buffer overflow (always written as 0)
        resp.write_u8(self.uart.is_active() as u8);
        // Trace count: remaining bytes in buffer
        resp.write_u32(self.uart.bytes_available() as u32);
    }

    fn process_swo_extended_status(&mut self, _req: Request, resp: &mut ResponseWriter) {
        // Trace status:
        // Bit 0: trace capture active
        // Bit 6: trace stream error (always written as 0)
        // Bit 7: trace buffer overflow (always written as 0)
        resp.write_u8(self.uart.is_active() as u8);
        // Trace count: remaining bytes in buffer.
        resp.write_u32(self.uart.bytes_available() as u32);
        // Index: sequence number of next trace. Always written as 0.
        resp.write_u32(0);
        // TD_TimeStamp: test domain timer value for trace sequence
        resp.write_u32(0);
    }

    fn process_swo_data(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        // Write status byte to response
        resp.write_u8(self.uart.is_active() as u8);

        // Skip length for now
        resp.skip(2);

        let mut buf = resp.remaining();

        // Limit maximum return size to maximum requested bytes
        let n = req.next_u16() as usize;
        if buf.len() > n {
            buf = &mut buf[..n];
        }

        // Read data from UART
        let len = self.uart.read(&mut buf);
        resp.skip(len);

        // Go back and write length
        resp.write_u16_at(2, len as u16);
    }

    fn process_jtag_sequence(&mut self, req: Request, resp: &mut ResponseWriter) {
        match self.mode {
            Some(DAPMode::JTAG) => {}
            _ => {
                resp.write_err();
                return;
            }
        }

        resp.write_ok();

        // Run requested JTAG sequences. Cannot fail.
        let size = self.jtag.sequences(req.rest(), resp.remaining());
        resp.skip(size);
    }

    fn process_transfer_configure(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        // We don't support variable idle cycles
        let _idle_cycles = req.next_u8();

        // Send number of wait retries through to SWD
        self.swd.set_wait_retries(req.next_u16() as usize);

        // Store number of match retries
        self.match_retries = req.next_u16() as usize;

        resp.write_ok();
    }

    fn process_transfer(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let _idx = req.next_u8();
        let ntransfers = req.next_u8();
        let mut match_mask = 0xFFFF_FFFFu32;

        // Ensure SWD pins are in the right mode, in case they've been used as outputs
        // by the SWJ_Pins command.
        self.pins.swd_clk_spi();
        self.pins.swd_tx();

        // Skip two bytes in resp to reserve space for final status,
        // which we update while processing.
        resp.write_u16(0);

        for transfer_idx in 0..ntransfers {
            // Store how many transfers we execute in the response
            resp.write_u8_at(1, transfer_idx + 1);

            // Parse the next transfer request
            let transfer_req = req.next_u8();
            let apndp = (transfer_req & (1 << 0)) != 0;
            let rnw = (transfer_req & (1 << 1)) != 0;
            let a = (transfer_req & (3 << 2)) >> 2;
            let vmatch = (transfer_req & (1 << 4)) != 0;
            let mmask = (transfer_req & (1 << 5)) != 0;
            let _ts = (transfer_req & (1 << 7)) != 0;

            if rnw {
                // Issue register read
                let mut read_value = if apndp {
                    // Reads from AP are posted, so we issue the
                    // read and subsequently read RDBUFF for the data.
                    // This requires an additional transfer so we'd
                    // ideally keep track of posted reads and just
                    // keep issuing new AP reads, but our reads are
                    // sufficiently fast that for now this is simpler.
                    let rdbuff = swd::DPRegister::RDBUFF.into();
                    if self.swd.read_ap(a).check(resp.mut_at(2)).is_none() {
                        break;
                    }
                    match self.swd.read_dp(rdbuff).check(resp.mut_at(2)) {
                        Some(v) => v,
                        None => break,
                    }
                } else {
                    // Reads from DP are not posted, so directly read the register.
                    match self.swd.read_dp(a).check(resp.mut_at(2)) {
                        Some(v) => v,
                        None => break,
                    }
                };

                // Handle value match requests by retrying if needed.
                // Since we're re-reading the same register the posting
                // is less important and we can just use the returned value.
                if vmatch {
                    let target_value = req.next_u32();
                    let mut match_tries = 0;
                    while (read_value & match_mask) != target_value {
                        match_tries += 1;
                        if match_tries > self.match_retries {
                            break;
                        }

                        read_value = match self.swd.read(apndp.into(), a).check(resp.mut_at(2)) {
                            Some(v) => v,
                            None => break,
                        }
                    }

                    // If we didn't read the correct value, set the value mismatch
                    // flag in the response and quit early.
                    if (read_value & match_mask) != target_value {
                        resp.write_u8_at(1, resp.read_u8_at(1) | (1 << 4));
                        break;
                    }
                } else {
                    // Save read register value
                    resp.write_u32(read_value);
                }
            } else {
                // Write transfer processing

                // Writes with match_mask set just update the match mask
                if mmask {
                    match_mask = req.next_u32();
                    continue;
                }

                // Otherwise issue register write
                let write_value = req.next_u32();
                if self
                    .swd
                    .write(apndp.into(), a, write_value)
                    .check(resp.mut_at(2))
                    .is_none()
                {
                    break;
                }
            }
        }
    }

    fn process_transfer_block(&mut self, mut req: Request, resp: &mut ResponseWriter) {
        let _idx = req.next_u8();
        let ntransfers = req.next_u16();
        let transfer_req = req.next_u8();
        let apndp = (transfer_req & (1 << 0)) != 0;
        let rnw = (transfer_req & (1 << 1)) != 0;
        let a = (transfer_req & (3 << 2)) >> 2;

        // Ensure SWD pins are in the right mode, in case they've been used as outputs
        // by the SWJ_Pins command.
        self.pins.swd_clk_spi();
        self.pins.swd_tx();

        // Skip three bytes in resp to reserve space for final status,
        // which we update while processing.
        resp.write_u16(0);
        resp.write_u8(0);

        // Keep track of how many transfers we executed,
        // so if there is an error the host knows where
        // it happened.
        let mut transfers = 0;

        // If reading an AP register, post first read early.
        if rnw && apndp && self.swd.read_ap(a).check(resp.mut_at(3)).is_none() {
            // Quit early on error
            resp.write_u16_at(1, 1);
            return;
        }

        for transfer_idx in 0..ntransfers {
            transfers = transfer_idx;
            if rnw {
                // Handle repeated reads
                let read_value = if apndp {
                    // For AP reads, the first read was posted, so on the final
                    // read we need to read RDBUFF instead of the AP register.
                    if transfer_idx < ntransfers - 1 {
                        match self.swd.read_ap(a).check(resp.mut_at(3)) {
                            Some(v) => v,
                            None => break,
                        }
                    } else {
                        let rdbuff = swd::DPRegister::RDBUFF.into();
                        match self.swd.read_dp(rdbuff).check(resp.mut_at(3)) {
                            Some(v) => v,
                            None => break,
                        }
                    }
                } else {
                    // For DP reads, no special care required
                    match self.swd.read_dp(a).check(resp.mut_at(3)) {
                        Some(v) => v,
                        None => break,
                    }
                };

                // Save read register value to response
                resp.write_u32(read_value);
            } else {
                // Handle repeated register writes
                let write_value = req.next_u32();
                let result = self.swd.write(apndp.into(), a, write_value);
                if result.check(resp.mut_at(3)).is_none() {
                    break;
                }
            }
        }

        // Write number of transfers to response
        resp.write_u16_at(1, transfers + 1);
    }

    fn process_transfer_abort(&mut self) {
        // We'll only ever receive an abort request when we're not already
        // processing anything else, since processing blocks checking for
        // new requests. Therefore there's nothing to do here.
    }
}

trait CheckResult<T> {
    /// Check result of an SWD transfer, updating the response status byte.
    ///
    /// Returns Some(T) on successful transfer, None on error.
    fn check(self, resp: &mut u8) -> Option<T>;
}

impl<T> CheckResult<T> for swd::Result<T> {
    fn check(self, resp: &mut u8) -> Option<T> {
        match self {
            Ok(v) => {
                *resp = 1;
                Some(v)
            }
            Err(swd::Error::AckWait) => {
                *resp = 2;
                None
            }
            Err(swd::Error::AckFault) => {
                *resp = 4;
                None
            }
            Err(_) => {
                *resp = (1 << 3) | 7;
                None
            }
        }
    }
}
