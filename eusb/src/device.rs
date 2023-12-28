use std::fmt::{Display, Formatter};
use std::time::Duration;
use crate::define::*;
use crate::endpoint::{EndpointPipIn};
use crate::error::*;
use crate::manager::Manager;
use crate::platform::{DeviceCtx, DeviceCtxImpl};
use crate::utils::bcd_to_version;


pub struct UsbDevice {
    ctx: DeviceCtxImpl,
}


impl Display for UsbDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let des = self.device_descriptor().unwrap();
        write!(f, "USB device [0x{:04X}:0x{:04X}], bus: {}, address: {}",
               des.idVendor, des.idProduct,
               self.ctx.bus_number(), self.ctx.device_address())
    }
}

impl From<DeviceCtxImpl> for UsbDevice {
    fn from(value: DeviceCtxImpl) -> Self {
        Self {
            ctx: value
        }
    }
}

#[allow(unused)]
impl UsbDevice {
    pub fn list() -> Result<Vec<UsbDevice>> {
        let manager = Manager::get();
        manager.device_list()
    }
    pub fn open_with_vid_pid(vid: u16, pid: u16) -> Result<UsbDevice> {
        let manager = Manager::get();
        manager.open_device_with_vid_pid(vid, pid)
    }

    pub fn serial_number(&self) -> Result<String> {
        self.ctx.serial_number()
    }

    pub fn product(&self) -> Result<String> {
        let des = self.device_descriptor()?;
        self.ctx.get_string_ascii(des.iProduct)
    }
    pub fn manufacturer(&self) -> Result<String> {
        let des = self.device_descriptor()?;
        self.ctx.get_string_ascii(des.iManufacturer)
    }
    pub fn bcd_usb_version(&self) -> Result<Vec<u16>> {
        let des = self.device_descriptor()?;
        Ok(bcd_to_version(des.bcdUSB))
    }

    pub fn device_class(&self) -> Result<DeviceClass> {
        self.ctx.device_class()
    }
    pub fn config_list(&self) -> Result<Vec<ConfigDescriptor>> {
        self.ctx.config_list()
    }
    pub fn set_config_by_value(&self, config_value: u8)->Result<()>{ self.ctx.set_config_by_value(config_value)}
    pub fn device_subclass(&self) -> Result<DeviceClass> {
        self.ctx.device_subclass()
    }

    pub fn device_protocol(&self) -> Result<DeviceClass> {
        self.ctx.device_protocol()
    }
    pub fn bcd_device_version(&self) -> Result<Vec<u16>> {
        let des = self.device_descriptor()?;
        Ok(bcd_to_version(des.bcdDevice))
    }
    pub fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        self.ctx.device_descriptor()
    }

    pub fn get_active_configuration(&self) -> Result<ConfigDescriptor> {
        self.ctx.get_active_configuration()
    }

    pub fn bulk_transfer_pip_in(&self, endpoint: u8, pip_config: PipConfig) -> Result<EndpointPipIn> {
        let inner = self.ctx.bulk_transfer_pip_in(endpoint, pip_config)?;
        Ok(inner.into())
    }

    pub async fn control_transfer_in(
        &self,
        control_transfer_request: ControlTransferRequest,
        capacity: usize,
    ) -> Result<Vec<u8>> {
        self.ctx.control_transfer_in(control_transfer_request, capacity).await
    }


    pub async fn control_transfer_out(
        &self,
        control_transfer_request: ControlTransferRequest,
        data: &[u8],
    ) -> Result<usize> {
        self.ctx.control_transfer_out(control_transfer_request, data).await
    }

    pub async fn bulk_transfer_in(
        &self, endpoint: u8, capacity: usize, timeout: Duration,
    ) -> Result<Vec<u8>> {
        self.ctx.bulk_transfer_in(endpoint, capacity, timeout).await
    }

    pub async fn bulk_transfer_out(
        &self, endpoint: u8, data: &[u8], timeout: Duration,
    ) -> Result<usize> {
        self.ctx.bulk_transfer_out(endpoint, data, timeout).await
    }
    pub async fn interrupt_transfer_in(
        &self, endpoint: u8, capacity: usize, timeout: Duration,
    ) -> Result<Vec<u8>> {
        self.ctx.interrupt_transfer_in(endpoint, capacity, timeout).await
    }

    pub async fn interrupt_transfer_out(
        &self, endpoint: u8, data: &[u8], timeout: Duration,
    ) -> Result<usize> {
        self.ctx.interrupt_transfer_out(endpoint, data, timeout).await
    }

    pub async fn iso_transfer_in(&self, endpoint: u8, num_iso_packages: usize, package_capacity: usize, timeout: Duration) -> Result<Vec<Vec<u8>>>{
        self.ctx.iso_transfer_in(endpoint, num_iso_packages, package_capacity, timeout).await
    }
}