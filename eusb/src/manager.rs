use std::sync::OnceLock;
use ctor::{ctor, dtor};
use crate::device::UsbDevice;
use crate::platform::*;
use crate::error::*;

static  MANAGER: OnceLock<Manager> = OnceLock::new();

pub(crate) struct Manager{
    pub(crate) platform: ManagerCtxImpl
}

impl Manager{
    pub fn get()->&'static Self{
        MANAGER.get().unwrap()
    }
    pub  fn device_list(&self)->Result<Vec<UsbDevice>>{
        self.platform.device_list()
    }
    pub fn open_device_with_vid_pid(&self, vid: u16, pid: u16)->Result<UsbDevice>{
        self.platform.open_device_with_vid_pid(vid, pid)
    }
}

#[ctor]
fn init_all() {
    MANAGER.get_or_init(||{
        let platform = ManagerCtxImpl::new();
        Manager{platform}
    });
}

#[dtor]
fn shutdown() {
    Manager::get().platform.close();
}










