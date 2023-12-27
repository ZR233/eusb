use std::future::Future;
use std::pin::Pin;
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
    fn control_transfer_in(&self, control_transfer_request: ControlTransferRequest, capacity: usize,
    ) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>>>>;
    fn control_transfer_out(&self, control_transfer_request: ControlTransferRequest, data: &[u8],
    ) -> Pin<Box<dyn Future<Output=Result<usize>>>>;

    fn bulk_transfer_pip_in(&self, endpoint: u8, pip_config: PipConfig)->Result<EndpointInImpl>;
}

pub(crate) trait ManagerCtx {
    fn new() -> Self;
    fn device_list(&self) -> Result<Vec<UsbDevice>>;
    fn open_device_with_vid_pid(&self, vid: u16, pid: u16) -> Result<UsbDevice>;
    fn close(&self);
}
