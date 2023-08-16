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

