use std::ffi::c_void;
use libusb_src::*;
use crate::error::*;

#[derive(Copy, Clone)]
pub struct Transfer(pub *mut libusb_transfer);
unsafe impl Sync for Transfer {}
unsafe impl Send for Transfer {}

impl Transfer{
    pub fn from_ptr(ptr: *mut libusb_transfer)->Self{
        Self(ptr)
    }

    pub unsafe fn new(iso_packets: u32)->Result<Self>{
        let t = libusb_alloc_transfer(iso_packets as _);
        if t.is_null(){
            return Err(Error::Other("Alloc transfer fail".to_string()));
        }
        Ok(Self(t))
    }
    pub unsafe fn submit(&self)->Result<()>{
        check_err(libusb_submit_transfer(self.0))?;
        Ok(())
    }

    pub(crate) fn set_callback(&mut self, callback: libusb_transfer_cb_fn){
        unsafe {
            (*self.0).callback = callback;
        }
    }
    pub(crate) fn set_user_data(&mut self, user_data: *mut c_void){
        unsafe {
            (*self.0).user_data = user_data;
        }
    }

    pub unsafe fn cancel(&self)->Result<()>{
        check_err(libusb_cancel_transfer(self.0))?;
        Ok(())
    }

    pub unsafe fn free(&self){
        libusb_free_transfer(self.0)
    }


}

