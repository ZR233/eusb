use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use futures::future::{BoxFuture, LocalBoxFuture};
use crate::error::*;
use crate::device::UsbDevice;
use crate::define::*;
#[cfg(unix)]
pub use std::os::unix::io::RawFd;


#[cfg(libusb)]
pub(crate) mod libusb;

#[cfg(libusb)]
pub(crate) use libusb::{device::DeviceCtxImpl, manager::ManagerCtxImpl, endpoint::EndpointPipInImpl};

type AsyncResult<T=()> =  Pin<Box<dyn Future<Output=Result<T>>>>;

pub(crate) trait EndpointPipInInner {
    fn next(&mut self)-> BoxFuture<Option<Vec<u8>>>;
}

pub(crate) trait DeviceCtx {
    fn device_descriptor(&self) -> Result<DeviceDescriptor>;
    fn get_string_ascii(&self, index: u8)-> Result<String>;
    fn device_class(&self) -> Result<DeviceClass>;
    fn device_subclass(&self) -> Result<DeviceClass>;
    fn device_protocol(&self) -> Result<DeviceClass>;
    fn config_list(&self) -> Result<Vec<ConfigDescriptor>>;
    fn set_config_by_value(&self, config_value: u8)->Result;
    fn serial_number(&self) -> Result<String>;
    fn bus_number(&self) -> u8;
    fn device_address(&self) -> u8;
    fn get_active_configuration(&self) -> Result<ConfigDescriptor>;
    fn control_transfer_in(&self, control_transfer_request: ControlTransferRequest, capacity: usize) -> AsyncResult<Vec<u8>>;
    fn control_transfer_out(&self, control_transfer_request: ControlTransferRequest, data: &[u8], ) -> AsyncResult<usize>;
    fn bulk_transfer_in(&self, endpoint: u8, capacity: usize, timeout: Duration) ->AsyncResult<Vec<u8>>;
    fn bulk_transfer_out(&self, endpoint: u8, data: &[u8], timeout: Duration)->AsyncResult<usize>;
    fn interrupt_transfer_in(&self, endpoint: u8, capacity: usize, timeout: Duration) ->AsyncResult<Vec<u8>>;
    fn interrupt_transfer_out(&self, endpoint: u8, data: &[u8], timeout: Duration)->AsyncResult<usize>;
    fn iso_transfer_in(&self, endpoint: u8, num_iso_packages: usize, package_capacity: usize, timeout: Duration) ->AsyncResult<Vec<Vec<u8>>>;
    fn iso_transfer_out(&self, endpoint: u8, packs: Vec<Vec<u8>>, timeout: Duration)->AsyncResult<Vec<usize>>;
    fn bulk_transfer_pip_in(&self, endpoint: u8, pip_config: PipConfig)->Result<EndpointPipInImpl>;
}

pub(crate) trait ManagerCtx {
    fn new() -> Self;
    fn device_list(&self) -> Result<Vec<UsbDevice>>;
    fn open_device_with_vid_pid(&self, vid: u16, pid: u16) -> Result<UsbDevice>;
    #[cfg(unix)]
    fn open_device_with_fd(&self, fd: RawFd)->Result<UsbDevice>;
    fn close(&self);
}
