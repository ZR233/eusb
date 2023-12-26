use std::fmt::{Display, Formatter};
use crate::error::*;
use crate::manager::Manager;
use crate::platform::DeviceCtxImpl;


pub struct UsbDevice{
    ctx: DeviceCtxImpl
}


impl Display for UsbDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB device: ", )
    }
}

impl From<DeviceCtxImpl> for UsbDevice{
    fn from(value: DeviceCtxImpl) -> Self {
        Self{
            ctx: value
        }
    }
}

impl UsbDevice{
    pub async fn list()->Result<Vec<UsbDevice>>{
        let manager = Manager::get();
        manager.device_list().await
    }


}