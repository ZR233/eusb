use std::ffi::c_int;
use std::sync::Arc;
use log::trace;
use libusb_src::*;
use super::device::CtxDeviceImpl;
pub(crate) use super::super::CtxDevice;
use super::super::CtxInterface;
use crate::error::*;
use crate::platform::ptr::DeviceHandle;

pub struct CtxInterfaceImpl{
    num: usize,
    device: Arc<CtxDeviceImpl>,
    handle: DeviceHandle,
}

unsafe impl Send for CtxInterfaceImpl{}
unsafe impl Sync for CtxInterfaceImpl{}

impl CtxInterfaceImpl{
    pub(crate) fn new(device: &Arc<CtxDeviceImpl>, num: usize)->Result<Self>{
        let device = device.clone();
        let handle = device.get_handle()?;

        let number = num as c_int;
        unsafe {
            let r = libusb_claim_interface(handle.0, number);
            check_err(r)?;
        }
        Ok(Self{
            num,
            device,
            handle,
        })
    }
}


impl CtxInterface for CtxInterfaceImpl{
    fn claim(&self) -> Result<()> {
        unsafe {
            let handle = self.device.get_handle()?;
            let r = libusb_claim_interface(handle.0, self.num as _);
            if r==LIBUSB_ERROR_NOT_SUPPORTED {
                return  Ok(());
            }
            check_err(r)?;
            Ok(())
        }
    }
}

impl Drop for CtxInterfaceImpl {
    fn drop(&mut self) {
        unsafe {
            libusb_release_interface(self.handle.0, self.num as _);
            trace!("Interface {} release.", self.num);
        }
    }
}