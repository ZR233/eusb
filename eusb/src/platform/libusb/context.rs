use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use log::debug;
use crate::device::UsbDevice;
use crate::error::*;
use libusb_src::*;
use crate::platform::libusb::device::Device;
use crate::platform::libusb::errors::*;
use crate::platform::{DeviceCtxImpl, ManagerCtx};


pub(crate) struct Context(*mut libusb_context);

unsafe impl Send for Context {}

unsafe impl Sync for Context {}

impl Context {
    fn new() -> Self {
        unsafe {
            let mut ptr = null_mut();
            check_err(libusb_init(&mut ptr)).unwrap();
            debug!("libusb_init");
            Self(ptr)
        }
    }

    fn device_list(&self) -> Result<Vec<Device>> {
        unsafe {
            let mut devs_raw: *const *mut libusb_device = null_mut();
            let count = check_err(libusb_get_device_list(self.0, &mut devs_raw) as _)? as usize;
            let list = &*slice_from_raw_parts(devs_raw, count);
            let out: Vec<Device> = list.iter().copied().map(|o| o.into()).collect();
            libusb_free_device_list(devs_raw, 0);
            Ok(out)
        }
    }

    fn exit(&self){
        unsafe {
            if !self.0.is_null() {
                debug!("libusb_exit");
                libusb_exit(self.0);
            }
        }
    }
}


pub(crate) struct ManagerCtxImpl {
    ctx: Arc<Context>,
}


impl ManagerCtx for ManagerCtxImpl {
    fn new() -> Self {
        Self {
            ctx: Arc::new(Context::new())
        }
    }

    fn device_list(&self) -> Result<Vec<UsbDevice>> {
        let ctx = self.ctx.clone();
        let mut d = ctx.device_list()?;
        let mut out = Vec::with_capacity(d.len());
        while let Some(one) = d.pop() {
            let device: DeviceCtxImpl = one.into();
            out.push(device.into());
        }
        Ok(out)
    }

    fn close(&self) {
       self.ctx.exit();
    }
}

