use std::sync::OnceLock;
use crate::device::UsbDevice;
use crate::platform::*;
use crate::error::*;

static  MANAGER: OnceLock<Manager> = OnceLock::new();

pub(crate) struct Manager{
    platform: ManagerCtxImpl
}
impl Manager{
    pub fn get()->&'static Self{
        MANAGER.get_or_init(||{
            let platform = ManagerCtxImpl::new();
            Self{platform}
        })
    }


    pub  fn device_list(&self)->Result<Vec<UsbDevice>>{
        self.platform.device_list()
    }
}












