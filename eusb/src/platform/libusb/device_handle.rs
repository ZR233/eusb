use std::ffi::CStr;
use libusb_src::*;
use crate::manager::Manager;
use super::errors::*;

pub(crate) struct DeviceHandle(*mut libusb_device_handle);

unsafe impl Send for DeviceHandle{}
unsafe impl Sync for DeviceHandle{}

impl From<*mut libusb_device_handle> for DeviceHandle{
    fn from(value: *mut libusb_device_handle) -> Self {
        Manager::get().platform.open_device();
        Self(value)
    }
}
impl Drop for DeviceHandle{
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                Manager::get().platform.close_device();
                libusb_close(self.0);
            }
        }
    }
}

impl DeviceHandle{

    pub fn get_string_descriptor_ascii(&self, index: u8)->Result<String>{
        unsafe {
            let mut buff = [0u8;1024];
            let _ = check_err( libusb_get_string_descriptor_ascii(self.0, index, buff.as_mut_ptr(), 1024))?;
            let c = CStr::from_ptr(buff.as_ptr() as _);
            let out =c.to_string_lossy().to_string();
            Ok(out)
        }
    }
}

