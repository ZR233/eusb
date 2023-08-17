use std::ptr::null_mut;
use libusb_src::*;
use crate::error::*;

#[derive(Copy, Clone)]
pub(crate) struct Context(pub(crate) *mut libusb_context);
unsafe impl Send for Context{}
unsafe impl Sync for Context{}


impl Context {
    pub(crate) fn new() ->Self{
        Self(null_mut())
    }

    pub(crate) fn init(&mut self) ->Result<()>{
        unsafe {
            let r = libusb_init(&mut self.0);
            check_err(r)?;
        }
        Ok(())
    }
}



#[derive(Copy, Clone)]
pub(crate) struct DeviceHandle(pub(crate) *mut libusb_device_handle);
unsafe impl Send for DeviceHandle{}
unsafe impl Sync for DeviceHandle{}

impl DeviceHandle {
    pub(crate) fn is_null(&self)->bool{ self.0.is_null() }
}