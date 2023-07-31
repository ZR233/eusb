use std::ffi::c_int;
use std::fmt::{Display, Formatter, Pointer};
use std::ptr::null_mut;
use std::sync::Mutex;
use libusb_src::*;
use libusb_src::constants::*;
use crate::error::*;
use crate::interface::Interface;


pub struct Device{
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<*mut libusb_device_handle> ,
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
            handle: Mutex::new(null_mut()),
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

    pub(crate) fn get_handle(&self)-> Result<*mut libusb_device_handle>{
        let mut g = self.handle.lock().unwrap();

        unsafe {
            if g.is_null() {
               let r = libusb_open(self.dev, &mut *g);
               check_err(r)?;
               libusb_set_auto_detach_kernel_driver(*g, 1);
            }
        }
        Ok(*g)
    }

    pub fn get_configuration(&self)->Result<i32>{
        unsafe {
            let mut config:c_int  = 0;
            let ptr = (&mut config) as *mut c_int;

            let r = libusb_get_configuration(self.get_handle()?, ptr);
            check_err(r)?;
            Ok(config as _)
        }
    }
    pub fn set_configuration(&self, config: i32)->Result<()>{
        unsafe {
            let config: c_int  = config as _;
            let r = libusb_set_configuration(self.get_handle()?, config);
            check_err(r)?;
        }
        Ok(())
    }

    pub fn get_interface(&self, index: usize) ->Result<Interface>{
        let dev_handle = self.get_handle()?;
        Interface::new(dev_handle, index)
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
            let mut handle = self.handle.lock().unwrap();
            if !handle.is_null() {
                libusb_close(*handle);
            }
        }
    }
}




