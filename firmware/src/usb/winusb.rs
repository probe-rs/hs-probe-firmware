use core::convert::TryFrom;
use num_enum::TryFromPrimitive;
use usb_device::class_prelude::*;
use usb_device::control::RequestType;

const fn u16_low(val: u16) -> u8 {
    val.to_le_bytes()[0]
}

const fn u16_high(val: u16) -> u8 {
    val.to_le_bytes()[1]
}

const GET_OS_FEATURE: u8 = b'A';

#[allow(non_snake_case)]
#[repr(u16)]
#[derive(TryFromPrimitive)]
pub enum OSFeatureDescriptorType {
    CompatibleID = 4,
    Properties = 5,
    Descriptor = 7,
}

const LEN: u16 = 330;

const VENDOR_CODE: u8 = 0x41;

const DAP_V2_INTERFACE: u8 = 3;
const DFU_INTERFACE: u8 = 4;

enum MsDescriptorTypes {
    Header = 0x0,
    HeaderConfiguration = 0x1,
    HeaderFunction = 0x2,
    CompatibleId = 0x3,
    RegistryProperty = 0x4,
}

const MS_OS_DESCRIPTOR: [u8; LEN as usize] = [
    0xa,
    0x00, // Length 10 bytes
    MsDescriptorTypes::Header as u8,
    0x00, // HEADER_DESCRIPTOR
    0x00,
    0x00,
    0x03,
    0x06, // Windows version
    u16_low(LEN),
    u16_high(LEN), // Total descriptor length
    // Function header,
    0x8,
    0x0, // Length 8
    MsDescriptorTypes::HeaderFunction as u8,
    0x00,
    DAP_V2_INTERFACE, // First interface (dap v2 -> 1)
    0x0,              // reserved
    8 + 20 + 132,
    0x00, // Subset length, including header
    // compatible ID descriptor
    20,
    0x00, // length 20
    MsDescriptorTypes::CompatibleId as u8,
    0x00,
    b'W',
    b'I',
    b'N',
    b'U',
    b'S',
    b'B',
    0x00,
    0x00, // Compatible ID: 8 bytes ASCII
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00, // Sub-Compatible ID: 8 bytes ASCII
    // Registry property
    80 + 2 + 42 + 2 + 2 + 2 + 2,
    0x00, // length
    MsDescriptorTypes::RegistryProperty as u8,
    0x00,
    7,
    0, // Data type: multi sz
    42,
    0x00, // property name length,
    b'D',
    0,
    b'e',
    0,
    b'v',
    0,
    b'i',
    0,
    b'c',
    0,
    b'e',
    0,
    b'I',
    0,
    b'n',
    0,
    b't',
    0,
    b'e',
    0,
    b'r',
    0,
    b'f',
    0,
    b'a',
    0,
    b'c',
    0,
    b'e',
    0,
    b'G',
    0,
    b'U',
    0,
    b'I',
    0,
    b'D',
    0,
    b's',
    0,
    0,
    0,
    80,
    0x00, // data length
    b'{',
    0,
    b'C',
    0,
    b'D',
    0,
    b'B',
    0,
    b'3',
    0,
    b'B',
    0,
    b'5',
    0,
    b'A',
    0,
    b'D',
    0,
    b'-',
    0,
    b'2',
    0,
    b'9',
    0,
    b'3',
    0,
    b'B',
    0,
    b'-',
    0,
    b'4',
    0,
    b'6',
    0,
    b'6',
    0,
    b'3',
    0,
    b'-',
    0,
    b'A',
    0,
    b'A',
    0,
    b'3',
    0,
    b'6',
    0,
    b'-',
    0,
    b'1',
    0,
    b'A',
    0,
    b'A',
    0,
    b'E',
    0,
    b'4',
    0,
    b'6',
    0,
    b'4',
    0,
    b'6',
    0,
    b'3',
    0,
    b'7',
    0,
    b'7',
    0,
    b'6',
    0,
    b'}',
    0,
    0,
    0,
    0,
    0,
    // Function header,
    0x8,
    0x0, // Length 8
    MsDescriptorTypes::HeaderFunction as u8,
    0x00,
    DFU_INTERFACE, // First interface (dap v2 -> 1)
    0x0,           // reserved
    8 + 20 + 132,  // Header + compatible ID
    0x00,          // Subset length, including header
    // compatible ID descriptor
    20,
    0x00, // length 20
    MsDescriptorTypes::CompatibleId as u8,
    0x00,
    b'W',
    b'I',
    b'N',
    b'U',
    b'S',
    b'B',
    0x00,
    0x00, // Compatible ID: 8 bytes ASCII
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00, // Sub-Compatible ID: 8 bytes ASCII
    // Registry property
    80 + 2 + 42 + 2 + 2 + 2 + 2,
    0x00, // length
    MsDescriptorTypes::RegistryProperty as u8,
    0x00,
    7,
    0, // Data type: multi sz
    42,
    0x00, // property name length,
    b'D',
    0,
    b'e',
    0,
    b'v',
    0,
    b'i',
    0,
    b'c',
    0,
    b'e',
    0,
    b'I',
    0,
    b'n',
    0,
    b't',
    0,
    b'e',
    0,
    b'r',
    0,
    b'f',
    0,
    b'a',
    0,
    b'c',
    0,
    b'e',
    0,
    b'G',
    0,
    b'U',
    0,
    b'I',
    0,
    b'D',
    0,
    b's',
    0,
    0,
    0,
    80,
    0x00, // data length
    b'{',
    0,
    b'A',
    0,
    b'5',
    0,
    b'D',
    0,
    b'C',
    0,
    b'B',
    0,
    b'F',
    0,
    b'1',
    0,
    b'0',
    0,
    b'-',
    0,
    b'6',
    0,
    b'5',
    0,
    b'3',
    0,
    b'0',
    0,
    b'-',
    0,
    b'1',
    0,
    b'1',
    0,
    b'D',
    0,
    b'2',
    0,
    b'-',
    0,
    b'9',
    0,
    b'0',
    0,
    b'1',
    0,
    b'F',
    0,
    b'-',
    0,
    b'0',
    0,
    b'0',
    0,
    b'C',
    0,
    b'0',
    0,
    b'4',
    0,
    b'F',
    0,
    b'B',
    0,
    b'9',
    0,
    b'5',
    0,
    b'1',
    0,
    b'E',
    0,
    b'D',
    0,
    b'}',
    0,
    0,
    0,
    0,
    0,
];

const MS_COMPATIBLE_ID_DESCRIPTOR: [u8; 40] = [
    0x28, 0x00, 0x00, 0x00, // Length 40 bytes
    0x00, 0x01, // Version
    0x04, 0x00, // Compatibility ID Descriptor index
    0x01, // Number of sections
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved: 7 bytes
    0x01, // Interface Number
    0x01, // Reserved
    b'W', b'I', b'N', b'U', b'S', b'B', 0x00, 0x00, // Compatible ID: 8 bytes ASCII
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Sub-Compatible ID: 8 bytes ASCII
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Reserved: 6 bytes
];

const IF2_MS_PROPERTIES_OS_DESCRIPTOR: [u8; 142] = [
    0x8e, 0x00, 0x00, 0x00, // Length, 40 bytes
    0x00, 0x01, // Version
    0x05, 0x00, // wIndex: properties
    0x01, 0x00, // wCount: a single property
    0x84, 0x00, 0x00, 0x00, // Property length
    0x01, 0x00, 0x00, 0x00, // dwPropertyDataType: REG_SZ
    0x28, 0x00, // name length
    b'D', 0, b'e', 0, b'v', 0, b'i', 0, b'c', 0, b'e', 0, b'I', 0, b'n', 0, b't', 0, b'e', 0, b'r',
    0, b'f', 0, b'a', 0, b'c', 0, b'e', 0, b'G', 0, b'U', 0, b'I', 0, b'D', 0, 0, 0, 0x4e, 0x00,
    0x00, 0x00, // data length
    b'{', 0, b'C', 0, b'D', 0, b'B', 0, b'3', 0, b'B', 0, b'5', 0, b'A', 0, b'D', 0, b'-', 0, b'2',
    0, b'9', 0, b'3', 0, b'B', 0, b'-', 0, b'4', 0, b'6', 0, b'6', 0, b'3', 0, b'-', 0, b'A', 0,
    b'A', 0, b'3', 0, b'6', 0, b'-', 0, b'1', 0, b'A', 0, b'A', 0, b'E', 0, b'4', 0, b'6', 0, b'4',
    0, b'6', 0, b'3', 0, b'7', 0, b'7', 0, b'6', 0, b'}', 0, 0, 0,
];

pub struct MicrosoftDescriptors;

impl<B: UsbBus> UsbClass<B> for MicrosoftDescriptors {
    /*
    fn get_string(&self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        // Special string to support Microsoft OS Desriptors
        if u8::from(index) == 0xee {
            Some("MSFT100A") // MSFT100 + vendor code (1 byte)
        } else {
            None
        }
    }
    */

    fn get_bos_descriptors(&self, writer: &mut BosWriter) -> usb_device::Result<()> {
        writer.capability(
            5,
            &[
                0, // reserved
                0xdf,
                0x60,
                0xdd,
                0xd8,
                0x89,
                0x45,
                0xc7,
                0x4c,
                0x9c,
                0xd2,
                0x65,
                0x9d,
                0x9e,
                0x64,
                0x8A,
                0x9f, // platform capability UUID , Microsoft OS 2.0 platform compabitility
                0x00,
                0x00,
                0x03,
                0x06, // Minimum compatible Windows version (8.1)
                u16_low(LEN),
                u16_high(LEN), // desciptor set total len ,
                VENDOR_CODE,
                0x0, // Device does not support alternate enumeration
            ],
        )
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();
        if req.request_type != RequestType::Vendor {
            return;
        }

        // Check if the ve
        if req.request == VENDOR_CODE {
            if req.index == 0x7 {
                xfer.accept_with_static(&MS_OS_DESCRIPTOR).ok();
            } else {
                xfer.reject().ok();
            }

            /*
            match OSFeatureDescriptorType::try_from(req.index) {
                Ok(OSFeatureDescriptorType::CompatibleID) => {
                    // Handle request for an Extended Compatible ID Descriptor.
                    // Interface  is ignored as there is only one device-wide
                    // Compatible ID Descriptor.
                    xfer.accept_with_static(&MS_COMPATIBLE_ID_DESCRIPTOR).ok();
                }
                Ok(OSFeatureDescriptorType::Properties) => {
                    // Handle request for an Extended Properties OS Descriptor.
                    match req.value as u8 {
                        2 => {
                            xfer.accept_with_static(&IF2_MS_PROPERTIES_OS_DESCRIPTOR)
                                .ok();
                        }
                        _ => {
                            xfer.reject().ok();
                        }
                    }
                }
                Ok(OSFeatureDescriptorType::Descriptor) => {
                    xfer.accept_with_static(&MS_OS_DESCRIPTOR).ok();
                }
                _ => {
                    xfer.reject().ok();
                }
            }
            */
        }
    }
}
