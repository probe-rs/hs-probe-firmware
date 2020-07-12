use usb_device::class_prelude::*;
use usb_device::Result;

pub struct CmsisDapV2<'a, B: UsbBus> {
    interface: InterfaceNumber,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
    trace_ep: EndpointIn<'a, B>,
}

impl<B: UsbBus> CmsisDapV2<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> CmsisDapV2<B> {
        CmsisDapV2 {
            interface: alloc.interface(),
            read_ep: alloc.alloc(Some(EndpointAddress::from(0x01)), EndpointType::Bulk, 64, 0).expect("alloc_ep failed"),
            write_ep: alloc.alloc(Some(EndpointAddress::from(0x81)), EndpointType::Bulk, 64, 0).expect("alloc_ep failed"),
            trace_ep: alloc.alloc(Some(EndpointAddress::from(0x82)), EndpointType::Bulk, 64, 0).expect("alloc_ep failed"),
        }
    }
}

impl<B: UsbBus> UsbClass<B> for CmsisDapV2<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface(self.interface, 0xff, 0, 0)?;

        writer.endpoint(&self.read_ep)?;
        writer.endpoint(&self.write_ep)?;
        writer.endpoint(&self.trace_ep)?;

        Ok(())
    }

    fn reset(&mut self) {
    }
}
