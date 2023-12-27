use std::ffi::c_void;
use std::ptr::{null_mut, slice_from_raw_parts_mut};
use std::time::Duration;
use libusb_src::*;
use super::errors::*;

pub(crate) struct Transfer(pub(crate) *mut libusb_transfer);

unsafe impl Sync for Transfer {}

unsafe impl Send for Transfer {}


impl Transfer {
    pub fn new(iso_packets: i32) -> Self {
        unsafe {
            let p = libusb_alloc_transfer(iso_packets);
            Self(p)
        }
    }

    pub unsafe fn control_transfer(
        callback: libusb_transfer_cb_fn,
        data: &[u8],
        request_type: u8, request: u8, value: u16, index: u16, timeout: Duration,
    ) -> Self {
        let t = Self::new(0);
        let length = data.len();
        let mut buffer = vec![0; LIBUSB_CONTROL_SETUP_SIZE + length];
        if (request & LIBUSB_ENDPOINT_DIR_MASK) == LIBUSB_ENDPOINT_OUT {
            buffer[LIBUSB_CONTROL_SETUP_SIZE..].copy_from_slice(data);
        }

        let buffer_ptr = buffer.as_mut_ptr();
        std::mem::forget(buffer);
        libusb_fill_control_setup(buffer_ptr, request_type, request, value, index, length as _);
        libusb_fill_control_transfer(t.0, null_mut(), buffer_ptr, callback, null_mut(), timeout.as_millis() as _);
        (*t.0).flags = LIBUSB_TRANSFER_FREE_BUFFER;
        t
    }

    pub unsafe fn bulk_transfer(
        endpoint: u8,
        callback: libusb_transfer_cb_fn,
        length: usize, timeout: Duration
    )->Self{
        let t = Self::new(0);
        unsafe {
            let mut buffer = vec![0; length];
            let buffer_ptr = buffer.as_mut_ptr();
            std::mem::forget(buffer);
            libusb_fill_bulk_transfer(t.0, null_mut(), endpoint, buffer_ptr, length as _, callback, null_mut(), timeout.as_millis() as _);
            (*t.0).flags = LIBUSB_TRANSFER_FREE_BUFFER;
        }
        t
    }


    pub fn control_transfer_get_data(&self) -> &[u8] {
        unsafe {
            let p = libusb_control_transfer_get_data(self.0);
            let l = (*self.0).actual_length as usize;
            &*slice_from_raw_parts_mut(p, l)
        }
    }
    pub unsafe fn set_handle(&mut self, handle: *mut libusb_device_handle) {
        (*self.0).dev_handle = handle;
    }
    pub unsafe fn set_callback(&mut self, callback: libusb_transfer_cb_fn) {
        (*self.0).callback = callback;
    }
    pub unsafe fn set_user_data(&mut self, user_data: *mut c_void) {
        (*self.0).user_data = user_data;
    }

    pub unsafe fn get_user_data(&mut self) -> *mut c_void {
        (*self.0).user_data
    }
    pub fn submit(&self) -> Result {
        unsafe {
            check_err(libusb_submit_transfer(self.0))?;
            Ok(())
        }
    }
    pub fn actual_length(&self) -> usize {
        unsafe {
            (*self.0).actual_length as _
        }
    }
    pub fn cancel(&self)->Result{
        unsafe {
            check_err(libusb_cancel_transfer(self.0))?;
            Ok(())
        }
    }

    pub fn result(&self) -> Result {
        unsafe {
            match (*self.0).status {
                LIBUSB_TRANSFER_COMPLETED => Ok(()),
                LIBUSB_TRANSFER_OVERFLOW => Err(Error::Overflow),
                LIBUSB_TRANSFER_TIMED_OUT => Err(Error::Timeout),
                LIBUSB_TRANSFER_CANCELLED => Err(Error::Cancelled),
                LIBUSB_TRANSFER_STALL => Err(Error::NotSupported),
                LIBUSB_TRANSFER_NO_DEVICE => Err(Error::NoDevice),
                _ => Err(Error::Other("Unknown".to_string())),
            }
        }
    }
}


impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.0);
        }
    }
}