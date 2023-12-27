use std::ffi::CStr;
use libusb_src::*;
use crate::manager::Manager;
use crate::platform::libusb::device::Device;
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

    pub fn claim_interface(&self, interface_number: u8)->Result{
        unsafe {
            check_err( libusb_claim_interface(self.0, interface_number as _))?;
            Ok(())
        }
    }
    pub fn release_interface(&self, interface_number: u8)->Result{
        unsafe {
            check_err( libusb_release_interface(self.0, interface_number as _))?;
            Ok(())
        }
    }
    pub fn get_configuration(&self)->Result<u8>{
        unsafe {
            let mut c = 0;
            check_err(libusb_get_configuration(self.0, &mut c))?;
            Ok(c as _)
        }
    }

    pub fn set_configuration(&self, config_value: u8) ->Result{
        unsafe {
            check_err(libusb_set_configuration(self.0, config_value as _))?;
            Ok(())
        }
    }
    pub fn clear_halt(&self, endpoint: usize)->Result{
        unsafe {
            check_err(libusb_clear_halt( self .0,endpoint as _))?;
            Ok(())
        }
    }

    pub fn get_string_descriptor_ascii(&self, index: u8)->Result<String>{
        unsafe {
            let mut buff = [0u8;1024];
            let _ = check_err( libusb_get_string_descriptor_ascii(self.0, index, buff.as_mut_ptr(), 1024))?;
            let c = CStr::from_ptr(buff.as_ptr() as _);
            let out =c.to_string_lossy().to_string();
            Ok(out)
        }
    }

    pub fn get_device(&self)->Device{
        unsafe {
            let mut dev = libusb_get_device(self.0);
            dev = libusb_ref_device(dev);
            dev
        }.into()
    }
}

