use std::ffi::c_void;
use std::ptr::{null_mut, slice_from_raw_parts_mut};
use std::time::Duration;

use log::trace;

use libusb_src::*;

use crate::platform::libusb::status_to_result;

use super::errors::*;

pub(crate) struct Transfer {
    pub ptr: *mut libusb_transfer,
    pub data: Vec<u8>,
}

unsafe impl Sync for Transfer {}

unsafe impl Send for Transfer {}

pub(crate) enum TransferDirection {
    Out { data: Vec<u8> },
    In { len: usize },
}

#[allow(unused)]
impl Transfer {
    pub fn new(iso_packets: i32, size: usize) -> Self {
        unsafe {
            let ptr = libusb_alloc_transfer(iso_packets);
            Self {
                ptr,
                data: vec![0; size],
            }
        }
    }
    pub fn new_with_direction(iso_packets: i32, direction: TransferDirection) -> Self {
        match direction {
            TransferDirection::Out { data } => {
                let mut t = Self::new(iso_packets, data.len());
                t.data.copy_from_slice(&data);
                t
            }
            TransferDirection::In { len } => {
                Self::new(iso_packets, len)
            }
        }
    }

    pub unsafe fn control_transfer(
        callback: libusb_transfer_cb_fn,
        direction: TransferDirection,
        request_type: u8, request: u8, value: u16, index: u16, timeout: Duration,
    ) -> Self {
        let length;
        let mut t = match direction {
            TransferDirection::Out { data } => {
                length = data.len();
                let mut t = Self::new(0, LIBUSB_CONTROL_SETUP_SIZE + length);
                t.data[LIBUSB_CONTROL_SETUP_SIZE..].copy_from_slice(&data);
                t
            }
            TransferDirection::In { len } => {
                length = len;
                Self::new(0, LIBUSB_CONTROL_SETUP_SIZE + len)
            }
        };

        let buffer_ptr = t.data.as_mut_ptr();
        libusb_fill_control_setup(buffer_ptr, request_type, request, value, index, length as _);
        libusb_fill_control_transfer(t.ptr, null_mut(), buffer_ptr, callback, null_mut(), timeout.as_millis() as _);
        t
    }

    pub unsafe fn bulk_transfer(
        mut endpoint: u8,
        callback: libusb_transfer_cb_fn,
        direction: TransferDirection,
        timeout: Duration,
    ) -> Self {
        endpoint = match &direction {
            TransferDirection::Out { .. } => { LIBUSB_ENDPOINT_OUT | endpoint }
            TransferDirection::In { .. } => { LIBUSB_ENDPOINT_IN | endpoint }
        };
        let mut t = Self::new_with_direction(0, direction);


        unsafe {
            let buffer_ptr = t.data.as_mut_ptr();
            libusb_fill_bulk_transfer(t.ptr, null_mut(), endpoint, buffer_ptr, t.data.len() as _, callback, null_mut(), timeout.as_millis() as _);
        }
        t
    }

    pub unsafe fn iso_transfer(
        mut endpoint: u8,
        num_iso_packets: i32,
        callback: libusb_transfer_cb_fn,
        direction: TransferDirection,
        timeout: Duration,
    ) -> Self {
        endpoint = match &direction {
            TransferDirection::Out { .. } => { LIBUSB_ENDPOINT_OUT | endpoint }
            TransferDirection::In { .. } => { LIBUSB_ENDPOINT_IN | endpoint }
        };
        let mut t = Self::new_with_direction(num_iso_packets, direction);
        let length = t.data.len() as i32;

        unsafe {
            let buffer_ptr = t.data.as_mut_ptr();
            libusb_fill_iso_transfer(
                t.ptr,
                null_mut(),
                endpoint,
                buffer_ptr,
                length,
                num_iso_packets,
                callback, null_mut(), timeout.as_millis() as _);

            libusb_set_iso_packet_lengths(t.ptr, (length / num_iso_packets) as _)
        }
        t
    }

    pub fn control_transfer_get_data(&self) -> &[u8] {
        unsafe {
            let p = libusb_control_transfer_get_data(self.ptr);
            let l = (*self.ptr).actual_length as usize;
            &*slice_from_raw_parts_mut(p, l)
        }
    }
    pub unsafe fn set_handle(&mut self, handle: *mut libusb_device_handle) {
        (*self.ptr).dev_handle = handle;
    }
    pub unsafe fn set_callback(&mut self, callback: libusb_transfer_cb_fn) {
        (*self.ptr).callback = callback;
    }
    pub unsafe fn set_user_data(&mut self, user_data: *mut c_void) {
        (*self.ptr).user_data = user_data;
    }

    pub unsafe fn get_user_data(&mut self) -> *mut c_void {
        (*self.ptr).user_data
    }
    pub fn submit(&self) -> Result {
        unsafe {
            check_err(libusb_submit_transfer(self.ptr))?;
            Ok(())
        }
    }
    pub fn actual_length(&self) -> usize {
        unsafe {
            (*self.ptr).actual_length as _
        }
    }
    pub fn cancel(&self) -> Result {
        unsafe {
            check_err(libusb_cancel_transfer(self.ptr))?;
            Ok(())
        }
    }

    pub fn result(&self) -> Result {
        unsafe {
            (*self.ptr).to_result()
        }
    }
}

pub(crate) trait ToResult {
    fn to_result(&self) -> Result;
}


impl ToResult for libusb_transfer {
    fn to_result(&self) -> Result {
        status_to_result(self.status)
    }
}


impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.ptr);
            trace!("transfer free");
        }
    }
}