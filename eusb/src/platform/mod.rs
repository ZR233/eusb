use crate::error::*;
use std::future::Future;
use std::pin::Pin;
use crate::device::UsbDevice;

#[cfg(libusb)]
pub(crate) mod libusb;
#[cfg(libusb)]
pub(crate)  use libusb::{context::ManagerCtxImpl, device::DeviceCtxImpl};


pub(crate) trait DeviceCtx{
    
}

pub(crate) trait ManagerCtx {
    fn new()->Self;
    fn device_list(&self)->Pin<Box<dyn Future<Output=Result<Vec<UsbDevice>>>>>;
}
