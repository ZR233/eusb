use std::fmt::{Display, Formatter};
use crate::define::{ConfigDescriptor, DeviceDescriptor};
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
    pub fn open_with_vid_pid(vid: u16, pid: u16)->Result<UsbDevice>{
        let manager = Manager::get();
        manager.open_device_with_vid_pid(vid, pid)
    }

    pub fn serial_number(&self) -> Result<String> {
        self.ctx.serial_number()
    }

    pub fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        self.ctx.device_descriptor()
    }

    pub fn get_active_configuration(&self)->Result<ConfigDescriptor>{
        self.ctx.get_active_configuration()
    }
}