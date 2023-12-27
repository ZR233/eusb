use std::ptr::{null_mut, slice_from_raw_parts};
use log::debug;
use crate::error::*;
use libusb_src::*;
use crate::platform::libusb::device::Device;
use crate::platform::libusb::errors::*;
use crate::platform::libusb::device_handle::DeviceHandle;


pub(crate) struct Context(*mut libusb_context);

unsafe impl Send for Context {}

unsafe impl Sync for Context {}

impl Context {
    pub(crate) fn new() -> Self {
        unsafe {
            let mut ptr = null_mut();
            check_err(libusb_init(&mut ptr)).unwrap();
            debug!("libusb_init");
            Self(ptr)
        }
    }

    pub(crate) fn device_list(&self) -> Result<Vec<Device>> {
        unsafe {
            let mut devs_raw: *const *mut libusb_device = null_mut();
            let count = check_err(libusb_get_device_list(self.0, &mut devs_raw) as _)? as usize;
            let list = &*slice_from_raw_parts(devs_raw, count);
            let out: Vec<Device> = list.iter().copied().map(|o| o.into()).collect();
            libusb_free_device_list(devs_raw, 0);
            Ok(out)
        }
    }
    pub(crate) fn handle_events(&self, )->Result{
        unsafe {
            check_err(libusb_handle_events(self.0))?;
        }
        Ok(())
    }
    pub(crate) fn open_device_with_vid_pid(&self, vid: u16, pid: u16 )->Result<DeviceHandle>{
        unsafe {
            let h = libusb_open_device_with_vid_pid(self.0, vid, pid);
            if h.is_null() {
                return Err(Error::NotFound);
            }
            Ok(DeviceHandle::from(h))
        }
    }


    pub(crate) fn exit(&self){
        unsafe {
            if !self.0.is_null() {
                debug!("libusb_exit");
                libusb_exit(self.0);
            }
        }
    }
}




