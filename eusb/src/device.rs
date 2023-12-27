use std::fmt::{Display, Formatter};
use crate::define::{ConfigDescriptor, ControlTransferRequest, DeviceDescriptor};
use crate::endpoint::EndpointIn;
use crate::error::*;
use crate::manager::Manager;
use crate::platform::{DeviceCtx, DeviceCtxImpl};


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

    pub fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        self.ctx.device_descriptor()
    }

    pub fn get_active_configuration(&self) -> Result<ConfigDescriptor> {
        self.ctx.get_active_configuration()
    }

    pub fn open_endpoint_in(&self, endpoint: u8) -> Result<EndpointIn> {
        let inner = self.ctx.open_endpoint_in(endpoint)?;
        Ok(inner.into())
    }

    pub async fn control_transfer_in(&self,
                                     control_transfer_request: ControlTransferRequest,
                                     capacity: usize,
    ) -> Result<Vec<u8>> {
        self.ctx.control_transfer_in(control_transfer_request, capacity).await
    }


    pub async fn control_transfer_out(&self,
                                      control_transfer_request: ControlTransferRequest,
                                      data: &[u8],
    ) -> Result<usize> {
        self.ctx.control_transfer_out(control_transfer_request, data).await
    }
}