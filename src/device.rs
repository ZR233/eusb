use std::fmt::{Display, Formatter, Pointer};
use std::ptr::null_mut;
use libusb_src::*;
use libusb_src::constants::*;
use crate::error::*;


pub struct Device{
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: *mut libusb_device_handle,
}
#[derive(Debug)]
pub enum UsbSpeed{
    Unknown,
    Low,
    Full,
    High,
    Super,
    SuperPlus,
}

unsafe impl Send for Device{}
unsafe impl Sync for Device{}
pub type Descriptor = libusb_device_descriptor;

impl Device{
    pub(crate) fn new(
        dev: *mut libusb_device,
    )->Self{
        unsafe {
            libusb_ref_device(dev);
        }

        Self{
            dev,
            handle: null_mut(),
        }
    }
    pub fn descriptor(&self)->Descriptor{
        let mut desc = Descriptor::default();
        unsafe {
            let desc_ptr = (&mut desc) as *mut libusb_device_descriptor;
            let r =  libusb_get_device_descriptor(self.dev, desc_ptr);
            if r < 0{
                return desc;
            }
        }
        desc
    }

    pub fn speed(&self)-> UsbSpeed{
        let r = unsafe {
           libusb_get_device_speed(self.dev)
        };
        if r < 0 {
            return UsbSpeed::Unknown;
        }

       match r {
            LIBUSB_SPEED_LOW => UsbSpeed::Low,
            LIBUSB_SPEED_FULL => UsbSpeed::Full,
            LIBUSB_SPEED_HIGH => UsbSpeed::High,
            LIBUSB_SPEED_SUPER=> UsbSpeed::Super,
            LIBUSB_SPEED_SUPER_PLUS => UsbSpeed::SuperPlus,
            LIBUSB_SPEED_UNKNOWN | _ =>UsbSpeed::Unknown
        }
    }

    fn get_handle(&mut self)-> Result< *mut libusb_device_handle>{
        unsafe {
            if self.handle.is_null() {
               let r = libusb_open(self.dev, &mut self.handle);
               check_err(r)?;
            }
        }
        Ok(self.handle)
    }






}


impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let des = self.descriptor();
        write!(f, "pid: {} vid: {}", des.idProduct, des.idVendor)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.dev);
            if !self.handle.is_null() {
                libusb_close(self.handle);
            }
        }
    }
}




