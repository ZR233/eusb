use std::ffi::{c_uchar, CStr};
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use libusb_src::*;
use crate::adaptor::*;
use super::device::CtxDeviceImpl;
use super::interface::Interface;

pub struct Config{
    configuration_value: i32,
    pub(crate) ptr: *const libusb_config_descriptor,
    pub(crate) device: Option<Arc<CtxDeviceImpl>>
}
impl From<*const libusb_config_descriptor> for Config{
    fn from(value: *const libusb_config_descriptor) -> Self {
        unsafe {
            let configuration_value = (*value).bConfigurationValue as _;

            Self{
                configuration_value,
                ptr: value,
                device: None,
            }
        }

    }
}

impl Drop for Config {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                libusb_free_config_descriptor(self.ptr);
            }
        }
    }
}

impl IConfig<Interface> for Config {
    fn with_value(value: i32) -> Self {
        Self{
            configuration_value: value,
            ptr:null_mut(),
            device: None
        }
    }

    fn configuration_value(&self) -> i32 {
        self.configuration_value
    }

    fn extra(&self) -> Vec<u8> {
        unsafe {
            let ptr = (*self.ptr).extra as *const u8;
            let len = (*self.ptr).extra_length as usize;
            let src = &*slice_from_raw_parts(ptr, len);
            src.to_vec()
        }
    }

    fn max_power(&self) -> u8 {
        unsafe { (*self.ptr).bMaxPower }
    }

    fn configuration(&self) -> Result<String> {
        unsafe {
            let handle = self.device.clone().unwrap().get_handle()?;
            let index = (*self.ptr).iConfiguration;
            let mut data = vec![0 as c_uchar; 1024];
            libusb_get_string_descriptor_ascii(handle.0, index, data.as_mut_ptr(), data.len() as _);
            let str = CStr::from_bytes_until_nul(&data)
                .map_err(|e|{
                    Error::Other(e.to_string())
            })?;
            let str = str.to_str().map_err(|e|{
                Error::Other(e.to_string())
            })?;
            Ok(str.to_string())
        }
    }


}