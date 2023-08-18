use std::ffi::{CStr, OsStr};
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use libusb_src::libusb_config_descriptor;
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

    fn interfaces(&self) -> Vec<Interface> {
        todo!()
    }
}