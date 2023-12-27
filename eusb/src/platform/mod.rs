use crate::error::*;
use crate::device::UsbDevice;

#[cfg(libusb)]
pub(crate) mod libusb;
#[cfg(libusb)]
pub(crate)  use libusb::{ device::DeviceCtxImpl, manager::ManagerCtxImpl, endpoint::EndpointInImpl};
use crate::define::{ConfigDescriptor, DeviceDescriptor};

pub(crate) trait EndpointInInner{
    
}
pub(crate) trait EndpointOutInner{
    
}

pub(crate) trait DeviceCtx{
    fn device_descriptor(&self)->Result< DeviceDescriptor>;
    fn serial_number(&self)->Result<String>;
    fn bus_number(&self)->u8;
    fn device_address(&self)->u8;
    fn get_active_configuration(&self)->Result<ConfigDescriptor>;
    fn open_endpoint_in(&self, endpoint: u8)->Result<EndpointInImpl>;
}

pub(crate) trait ManagerCtx {
    fn new()->Self;
    fn device_list(&self)->Result<Vec<UsbDevice>>;
    fn open_device_with_vid_pid(&self, vid: u16, pid: u16)->Result<UsbDevice>;
    fn close(&self);
}
