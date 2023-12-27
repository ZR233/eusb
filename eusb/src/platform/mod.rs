use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use crate::error::*;
use crate::device::UsbDevice;
use crate::define::*;


#[cfg(libusb)]
pub(crate) mod libusb;

#[cfg(libusb)]
pub(crate) use libusb::{device::DeviceCtxImpl, manager::ManagerCtxImpl, endpoint::EndpointInImpl};


pub(crate) trait EndpointInInner {}

pub(crate) trait EndpointOutInner {}

pub(crate) trait DeviceCtx {
    fn device_descriptor(&self) -> Result<DeviceDescriptor>;
    fn serial_number(&self) -> Result<String>;
    fn bus_number(&self) -> u8;
    fn device_address(&self) -> u8;
    fn get_active_configuration(&self) -> Result<ConfigDescriptor>;
    fn open_endpoint_in(&self, endpoint: u8) -> Result<EndpointInImpl>;
    fn control_transfer_in(&self,
                           recipient: UsbControlRecipient,
                           transfer_type: UsbControlTransferType,
                           request: u8,
                           value: u16,
                           index: u16,
                           timeout: Duration,
                           capacity: usize,
    ) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>>>>;
    fn control_transfer_out(&self,
                           recipient: UsbControlRecipient,
                           transfer_type: UsbControlTransferType,
                           request: u8,
                           value: u16,
                           index: u16,
                           timeout: Duration,
                           data: &[u8],
    ) -> Pin<Box<dyn Future<Output=Result<usize>>>>;
}

pub(crate) trait ManagerCtx {
    fn new() -> Self;
    fn device_list(&self) -> Result<Vec<UsbDevice>>;
    fn open_device_with_vid_pid(&self, vid: u16, pid: u16) -> Result<UsbDevice>;
    fn close(&self);
}
