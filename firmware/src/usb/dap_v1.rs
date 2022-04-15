use crate::app::Request;
use crate::DAP1_PACKET_SIZE;
use usb_device::control::{Recipient, RequestType};
use usb_device::Result;
use usb_device::{class_prelude::*, device};

const INTERFACE_CLASS_HID: u8 = 0x03;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum DescriptorType {
    Hid = 0x21,
    Report = 0x22,
}

const REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xFF, // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01, // Usage (0x01)
    0xA1, 0x01, // Collection (Application)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0xFF, //   Logical Maximum (255)
    0x75, 0x08, //   Report Size (8)
    0x95, 0x40, //   Report Count (64)
    0x09, 0x01, //   Usage (0x01)
    0x81, 0x02, //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    0x95, 0x40, //   Report Count (64)
    0x09, 0x01, //   Usage (0x01)
    0x91,
    0x02, //   Output (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0x95, 0x01, //   Report Count (1)
    0x09, 0x01, //   Usage (0x01)
    0xB1,
    0x02, //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    0xC0, // End Collection

          // 32 bytes
];

pub struct CmsisDapV1<'a, B: UsbBus> {
    interface: InterfaceNumber,
    name: StringIndex,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
}

impl<B: UsbBus> CmsisDapV1<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> CmsisDapV1<B> {
        CmsisDapV1 {
            interface: alloc.interface(),
            name: alloc.string(),
            read_ep: alloc
                .alloc(
                    Some(EndpointAddress::from(0x01)),
                    EndpointType::Interrupt,
                    DAP1_PACKET_SIZE,
                    1,
                )
                .expect("alloc_ep failed"),
            write_ep: alloc
                .alloc(
                    Some(EndpointAddress::from(0x81)),
                    EndpointType::Interrupt,
                    DAP1_PACKET_SIZE,
                    1,
                )
                .expect("alloc_ep failed"),
        }
    }

    pub fn process(&mut self) -> Option<Request> {
        let mut buf = [0u8; DAP1_PACKET_SIZE as usize];
        match self.read_ep.read(&mut buf) {
            Ok(size) if size > 0 => Some(Request::DAP1Command((buf, size))),
            _ => None,
        }
    }

    pub fn write_packet(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > self.write_ep.max_packet_size() as usize {
            return Err(UsbError::BufferOverflow);
        }
        self.write_ep.write(&data).map(|_| ())
    }
}

impl<B: UsbBus> UsbClass<B> for CmsisDapV1<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface_alt(
            self.interface,
            device::DEFAULT_ALTERNATE_SETTING,
            INTERFACE_CLASS_HID,
            0,
            0,
            Some(self.name),
        )?;

        let descriptor_len = REPORT_DESCRIPTOR.len();
        if descriptor_len > u16::max_value() as usize {
            return Err(UsbError::InvalidState);
        }
        let descriptor_len = (descriptor_len as u16).to_le_bytes();
        writer.write(
            DescriptorType::Hid as u8,
            &[
                0x11,                         // bcdHID.lower
                0x01,                         // bcdHID.upper
                0x00,                         // bCountryCode: 0 = not supported
                0x01,                         // bNumDescriptors
                DescriptorType::Report as u8, // bDescriptorType
                descriptor_len[0],            // bDescriptorLength.lower
                descriptor_len[1],            // bDescriptorLength.upper
            ],
        )?;

        writer.endpoint(&self.read_ep)?;
        writer.endpoint(&self.write_ep)?;

        Ok(())
    }

    fn get_string(&self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        if index == self.name {
            Some("HS-Probe CMSIS-DAP v1 Interface")
        } else {
            None
        }
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();
        if !(req.request_type == RequestType::Standard
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.interface) as u16)
        {
            return;
        }

        if req.request == control::Request::GET_DESCRIPTOR {
            let (dtype, index) = req.descriptor_type_index();
            if dtype == DescriptorType::Report as u8 && index == 0 {
                xfer.accept_with(REPORT_DESCRIPTOR).ok();
            }
        }
    }
}
