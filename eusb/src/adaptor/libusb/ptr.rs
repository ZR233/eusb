use std::ptr::null_mut;
use libusb_src::*;
use crate::define::DeviceClass;
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



#[derive(Copy, Clone)]
pub(crate) struct DeviceHandle(pub(crate) *mut libusb_device_handle);
unsafe impl Send for DeviceHandle{}
unsafe impl Sync for DeviceHandle{}

impl DeviceHandle {
    pub(crate) fn is_null(&self)->bool{ self.0.is_null() }
}



pub(crate) struct ConfigDescriptorPtr{
    pub(crate) config: *const libusb_config_descriptor
}



impl ConfigDescriptorPtr{
    pub(crate) fn new(dev: *const libusb_device, index: u8)->Result<Self>{
        unsafe {
            let mut config_ptr: *const  libusb_config_descriptor = null_mut();
            let r = libusb_get_config_descriptor(dev, index, &mut config_ptr);
            check_err(r)?;
            Ok( Self{config: config_ptr})
        }
    }

}

impl Drop for ConfigDescriptorPtr{
    fn drop(&mut self) {
        unsafe {
            libusb_free_config_descriptor(self.config);
        }
    }
}

pub(crate) fn class_from_lib(class: u8)->DeviceClass{
    match class {
        LIBUSB_CLASS_PER_INTERFACE=> DeviceClass::PerInterface,
        LIBUSB_CLASS_AUDIO => DeviceClass::Audio,
        LIBUSB_CLASS_COMM => DeviceClass::Comm,
        LIBUSB_CLASS_HID => DeviceClass::Hid,
        LIBUSB_CLASS_PHYSICAL => DeviceClass::Physical,
        LIBUSB_CLASS_PRINTER => DeviceClass::Printer,
        LIBUSB_CLASS_IMAGE => DeviceClass::Image,
        LIBUSB_CLASS_MASS_STORAGE => DeviceClass::MassStorage,
        LIBUSB_CLASS_HUB => DeviceClass::Hub,
        LIBUSB_CLASS_DATA => DeviceClass::Data,
        LIBUSB_CLASS_SMART_CARD => DeviceClass::SmartCard,
        LIBUSB_CLASS_CONTENT_SECURITY => DeviceClass::ContentSecurity,
        LIBUSB_CLASS_VIDEO => DeviceClass::Video,
        LIBUSB_CLASS_PERSONAL_HEALTHCARE => DeviceClass::PersonalHealthcare,
        LIBUSB_CLASS_DIAGNOSTIC_DEVICE => DeviceClass::DiagnosticDevice,
        LIBUSB_CLASS_WIRELESS => DeviceClass::Wireless,
        LIBUSB_CLASS_APPLICATION => DeviceClass::Application,
        LIBUSB_CLASS_VENDOR_SPEC => DeviceClass::VendorSpec,
        _ => panic!("Unknown class: {}", class)
    }
}