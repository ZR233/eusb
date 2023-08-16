use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use libusb_src::*;
pub(crate) use crate::define::ResultFuture;
pub(crate) use crate::define::CtxDevice;
use crate::platform::libusb::interface::Interface;
use crate::platform::libusb::Manager;
use crate::platform::libusb::ptr::Context;

pub(crate) struct Device{
    ctx: Context,
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<*mut libusb_device_handle>,
    pub(crate) manager: Option<Arc<Manager>>
}

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device{
    pub(crate) fn new(ctx: Context, dev: *mut libusb_device)->Self{
        return Self{
            ctx,
            dev,
            handle: Mutex::new(null_mut()),
            manager: None,
        }
    }
}


impl CtxDevice<Interface> for Device{
    fn interface_list(&self) -> ResultFuture<Vec<Arc<Interface>>> {
        Box::pin(async{
            let o = vec![];
            Ok(o)
        })
    }

    fn serial_number(&self) -> ResultFuture<String> {
        Box::pin(async{
            Ok("test".to_string())
        })
    }
}




