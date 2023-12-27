use crate::error::*;
use crate::device::UsbDevice;

#[cfg(libusb)]
pub(crate) mod libusb;
#[cfg(libusb)]
pub(crate)  use libusb::{ device::DeviceCtxImpl, manager::ManagerCtxImpl};
use crate::define::DeviceDescriptor;


pub(crate) trait DeviceCtx{
    fn device_descriptor(&self)->Result< DeviceDescriptor>;
    fn serial_number(&self)->Result<String>;
    fn bus_number(&self)->u8;
    fn device_address(&self)->u8;
}

pub(crate) trait ManagerCtx {
    fn new()->Self;
    fn device_list(&self)->Result<Vec<UsbDevice>>;
    fn close(&self);
}
