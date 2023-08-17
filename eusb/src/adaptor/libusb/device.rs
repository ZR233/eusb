use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{trace};
use libusb_src::*;
pub(crate) use super::super::ResultFuture;
pub(crate) use super::super::CtxDevice;
use super::interface::CtxInterfaceImpl;
use super::Manager;
use super::ptr::*;
use crate::error::*;
use crate::platform::Request;
use crate::adaptor::{EndpointDirection, RequestParamControlTransfer};
use crate::adaptor::libusb::channel::{request_channel, RequestReceiver, RequestSender};
use crate::adaptor::libusb::transfer::Transfer;
use crate::define::Endpoint;

pub(crate) struct CtxDeviceImpl {
    ctx: Context,
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<DeviceHandle>,
    pub(crate) manager: Option<Arc<Manager>>,

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

    pub(crate) fn get_handle(&self) -> Result<DeviceHandle> {
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

   pub(crate)  fn transfer_channel(self: &Arc<Self>, buffer: usize) -> (RequestSender, RequestReceiver) {
        let (tx, rx) = request_channel(buffer);
        return (tx, rx)
    }
}


impl CtxDevice<CtxInterfaceImpl, Request> for CtxDeviceImpl {
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

    fn serial_number(self: &Arc<Self>) -> ResultFuture<String> {
        let desc = self.descriptor();
        let s = self.clone();
        Box::pin(async move{
            let dev = s.get_handle()?;
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

    fn control_request(self: &Arc<Self>, param: RequestParamControlTransfer, direction: EndpointDirection) -> Result<Request> {
        let request = Request::control(self, param, direction)?;
        Ok(request)
    }

    fn bulk_request(
        self: &Arc<Self>,
        endpoint: Endpoint,
        package_len: usize,
        timeout: Duration)-> Result<Request>{

        Request::bulk(self, endpoint, package_len, timeout)
    }


    fn get_interface(self: &Arc<Self>, num: usize) -> Result<CtxInterfaceImpl> {
        CtxInterfaceImpl::new(self, num)
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

