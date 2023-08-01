use std::ffi::{c_int, c_uchar};
use libusb_src::*;
use crate::error::*;
use futures::channel::oneshot::*;
use libusb_src::*;




pub struct  Interface{
    number: c_int,
    dev_handle: *mut libusb_device_handle,
}

unsafe impl Send for Interface{}
unsafe impl Sync for Interface{}



impl Interface {
    pub(crate) fn new(dev_handle: *mut libusb_device_handle, index: usize)->Result<Self>{
        let number = index as c_int;
        unsafe {
            let r = libusb_claim_interface(dev_handle, number);
            check_err(r)?
        }

        Ok(Self{
            number,
            dev_handle
        })
    }


}

impl Drop for Interface {
    fn drop(&mut self) {
        unsafe {
            libusb_release_interface(self.dev_handle, self.number);
        }
    }
}


