use std::ffi::{c_char, c_uchar, CStr, CString, FromVecWithNulError, OsStr};
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use libusb_src::{libusb_config_descriptor, libusb_get_string_descriptor_ascii};
use crate::adaptor::IConfig;
use crate::prelude::CtxDeviceImpl;
use super::interface::Interface;

pub struct Config{
    configuration_value: u8,
    ptr: *const libusb_config_descriptor,
    pub(crate) device: Option<Arc<CtxDeviceImpl>>
}
impl From<*const libusb_config_descriptor> for Config{
    fn from(value: *const libusb_config_descriptor) -> Self {
        unsafe {
            let configuration_value = (*value).bConfigurationValue;

            Self{
                configuration_value,
                ptr: value,
                device: None,
            }
        }

    }
}


impl IConfig<Interface> for Config {
    fn with_value(value: usize) -> Self {
        Self{
            configuration_value:value as _,
            ptr:null_mut(),
            device: None
        }
    }

    fn configuration_value(&self) -> u8 {
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

    fn configuration(&self) -> String {
        unsafe {
            let handle = match self.device.clone().unwrap().get_handle() {
                Ok(h) => {h}
                Err(_) => {return String::new()}
            } ;
            let index = (*self.ptr).iConfiguration;
            let mut data = vec![0 as c_uchar; 1024];
            libusb_get_string_descriptor_ascii(handle.0, index, data.as_mut_ptr(), data.len() as _);
            let str = CStr::from_bytes_until_nul(&data).unwrap();
            str.to_str().unwrap().to_string()
        }
    }

    fn interfaces(&self) -> Vec<Interface> {

        unsafe {
            let out = Vec::with_capacity((*self.ptr).bNumInterfaces as _);



            out
        }
    }
}