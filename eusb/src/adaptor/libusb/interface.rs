use std::sync::Arc;
use log::trace;
use libusb_src::*;
use super::device::CtxDeviceImpl;
use super::super::IInterface;
use crate::error::*;

pub struct Interface {
    num: usize,
    device: Arc<CtxDeviceImpl>,
}

unsafe impl Send for Interface {}
unsafe impl Sync for Interface {}

impl Interface {
    pub(crate) fn new(device: &Arc<CtxDeviceImpl>, num: usize)->Result<Self>{
        let device = device.clone();
        let s = Self{
            num,
            device: device.clone(),
        };

        unsafe {
            let handle = device.get_handle()?;
            let r = libusb_claim_interface(handle.0, s.num as _);
            if r==LIBUSB_ERROR_NOT_SUPPORTED {
                return  Ok(s);
            }
            check_err(r)?;
            Ok(s)
        }
    }
}


impl IInterface for Interface {}

impl Drop for Interface {
    fn drop(&mut self) {
        unsafe {
            match self.device.get_handle() {
                Ok(h) => {
                    libusb_release_interface(h.0, self.num as _);
                    trace!("Interface {} release.", self.num);
                }
                Err(_) => {}
            };
        }
    }
}