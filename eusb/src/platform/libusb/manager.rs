use std::ptr::{null_mut, slice_from_raw_parts};
use libusb_src::*;
use crate::define::{CtxManager, ResultFuture};
use crate::platform::libusb::device::Device;
use crate::platform::libusb::interface::Interface;
use crate::platform::libusb::ptr::Context;
use crate::error::*;

pub(crate) struct Manager{
    ctx: Context
}

impl Manager{
    pub(crate) fn new()->Result<Self>{
        let mut ctx = Context::new();
        ctx.init()?;
        Ok(Self{
            ctx
        })
    }
}

impl CtxManager<Interface, Device> for Manager {
    fn device_list(&self) -> ResultFuture<Vec<Device>>{
        let ctx = self.ctx;
        Box::pin(async move {
            let list = unsafe {
                let mut devs_raw: *const *mut libusb_device = null_mut();
                let cnt = libusb_get_device_list(ctx.0, &mut devs_raw);
                check_err(cnt as _)?;

                let list = &*slice_from_raw_parts(devs_raw, cnt as _);

                list.into_iter().map(|one|{Device::new(ctx, *one)}).collect()
            };
            Ok(list)
        })
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        unsafe {
            libusb_exit(self.ctx.0);
        }
        println!("drop manager");
    }
}

