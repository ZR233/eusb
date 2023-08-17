use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use log::{error, trace};
use libusb_src::*;
pub(crate) use super::super::ResultFuture;
pub(crate) use super::super::CtxDevice;
use super::interface::CtxInterfaceImpl;
use super::Manager;
use super::ptr::*;
use crate::error::*;

pub(crate) struct CtxDeviceImpl {
    ctx: Context,
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<DeviceHandle>,
    pub(crate) manager: Option<Arc<Manager>>
}

unsafe impl Send for CtxDeviceImpl {}
unsafe impl Sync for CtxDeviceImpl {}

impl CtxDeviceImpl {
    pub(crate) fn new(ctx: Context, dev: *mut libusb_device)->Self{
        return Self{
            ctx,
            dev,
            handle: Mutex::new(DeviceHandle(null_mut())),
            manager: None,
        }
    }

    fn descriptor(&self) -> libusb_device_descriptor {
        let mut desc =libusb_device_descriptor::default();
        unsafe {
            let _ = libusb_get_device_descriptor(self.dev, &mut desc);
        }
        desc
    }

    fn get_handle(&self) -> Result<DeviceHandle> {
        let mut g = self.handle.lock().unwrap();

        unsafe {
            if g.is_null() {
                let r = libusb_open(self.dev, &mut g.0);
                check_err(r)?;
                let manager = self.manager.clone().unwrap();
                manager.open_device();

                libusb_set_auto_detach_kernel_driver(g.0, 1);
            }
        }
        Ok(g.clone())
    }
}


impl CtxDevice<CtxInterfaceImpl> for CtxDeviceImpl {
    fn pid(&self) -> u16 {
        let desc = self.descriptor();
        unsafe {
            desc.idProduct
        }
    }

    fn vid(&self) -> u16 {
        let desc = self.descriptor();
        unsafe {
            desc.idVendor
        }
    }

    fn interface_list(&self) -> ResultFuture<Vec<Arc<CtxInterfaceImpl>>> {
        Box::pin(async{
            let o = vec![];
            Ok(o)
        })
    }

    fn serial_number(&self) -> ResultFuture<String> {
        let desc = self.descriptor();
        let handle = self.get_handle();
        Box::pin(async move{
            let dev = handle?;
            let index = desc.iSerialNumber;
            let mut buff = vec![0u8; 256];
            let buff_len = buff.len();
            if index > 0 {
                unsafe {
                    let r = libusb_get_string_descriptor_ascii(
                        dev.0,
                        index,
                        buff.as_mut_ptr(),
                        buff_len as _
                    );
                    if r > 0{
                        buff.resize(r as _, 0);
                        match String::from_utf8(buff){
                            Ok(s) => {return Ok(s);}
                            Err(_) => {}
                        }
                    }

                }
            }
            Ok(String::new())
        })
    }
}


impl Drop for CtxDeviceImpl {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.dev);
            let handle = self.handle.lock().unwrap();
            if !handle.is_null() {
                self.manager.clone().unwrap().close_device();
                libusb_close(handle.0);
                trace!("Device closed");
            }
        }
    }
}

