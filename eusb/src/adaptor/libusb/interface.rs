use std::sync::Arc;
use std::time::Duration;
use log::trace;
use libusb_src::*;
use crate::define::{EndpointDescriptor};
use super::device::CtxDeviceImpl;
use super::super::IInterface;
use crate::error::*;
use crate::platform::Request;

pub struct Interface {
    num: usize,
    device: Arc<CtxDeviceImpl>,
}

unsafe impl Send for Interface {}
unsafe impl Sync for Interface {}

impl Interface {
    pub(crate) fn new(device: &Arc<CtxDeviceImpl>, num: usize)->Result<Self>{
        let device = device.clone();
        let s = Self{
            num,
            device: device.clone(),
        };

        unsafe {
            let handle = device.get_handle()?;
            let r = libusb_claim_interface(handle.0, s.num as _);
            if r==LIBUSB_ERROR_NOT_SUPPORTED {
                return  Ok(s);
            }
            check_err(r)?;
            Ok(s)
        }
    }
}


impl IInterface<Request> for Interface {
    fn bulk_request(&self, endpoint: EndpointDescriptor, data:Vec<u8>, timeout: Duration) -> Result<Request> {
        Request::bulk(&self.device, endpoint, data, timeout)
    }

    fn interrupt_request(&self, endpoint: EndpointDescriptor, data: Vec<u8>, timeout: Duration) -> Result<Request> {
        Request::interrupt(&self.device, endpoint, data, timeout)
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        unsafe {
            match self.device.get_handle() {
                Ok(h) => {
                    libusb_release_interface(h.0, self.num as _);
                    trace!("Interface {} release.", self.num);
                }
                Err(_) => {}
            };
        }
    }
}