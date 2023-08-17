use std::os::fd::RawFd;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::{Arc, Condvar, Mutex};
use log::{trace};
use libusb_src::*;
use super::super::{CtxManager, ResultFuture};
use super::device::CtxDeviceImpl;
use super::interface::CtxInterfaceImpl;
use super::ptr::Context;
use crate::error::*;
use crate::platform::ptr::DeviceHandle;
use crate::platform::Request;

pub(crate) struct Manager{
    ctx: Context,
    event: Arc<Mutex<EventControllerCtx>>,
    cond:  Arc<Condvar>,
}

impl Manager{
    pub(crate) fn new(mut ctx: Context) ->Result<Self>{
        ctx.init()?;

        let s =
        Self{
            ctx,
            event:Arc::new(Mutex::new(EventControllerCtx{
                device_count: 0,
                is_exit: false,
            })),
            cond: Arc::new(Condvar::new()),
        };

        s.work_event();

        Ok(s)
    }
    fn work_event(&self){
        let cond = self.cond.clone();
        let event = self.event.clone();
        let ptr = self.ctx.clone();

        std::thread::spawn(move || {
            let p = ptr;
            let mut ctx = {
                event.lock().unwrap().clone()
            };

            unsafe {
                while !ctx.is_exit{
                    if ctx.device_count > 0 {
                        libusb_handle_events(p.0);
                        ctx = event.lock().unwrap().clone()
                    }else{
                        let mut g = event.lock().unwrap();
                        g = cond.wait(g).unwrap();
                        ctx = g.clone();
                    }
                }
                libusb_exit(p.0);
                trace!("event_finish");
            }
        });

    }
    pub(crate)  fn open_device(&self){
        let mut ctx = self.event.lock().unwrap();
        (*ctx).device_count+=1;
        self.cond.notify_all();
    }

    pub(crate)  fn close_device(&self){
        let mut ctx = self.event.lock().unwrap();
        (*ctx).device_count-=1;
        self.cond.notify_all();
    }

    fn exit(&self){
        let mut ctx = self.event.lock().unwrap();
        (*ctx).is_exit=true;
        self.cond.notify_all();
    }
}

impl CtxManager<CtxInterfaceImpl, Request, CtxDeviceImpl> for Manager {
    fn device_list(&self) -> ResultFuture<Vec<CtxDeviceImpl>>{
        let ctx = self.ctx;
        Box::pin(async move {
            let ctx = ctx;
            let list = unsafe {
                let mut devs_raw: *const *mut libusb_device = null_mut();
                let cnt = libusb_get_device_list(ctx.0, &mut devs_raw);
                check_err(cnt as _)?;

                let list = &*slice_from_raw_parts(devs_raw, cnt as _);

                list.into_iter().map(|one|{CtxDeviceImpl::new(*one)}).collect()
            };
            Ok(list)
        })
    }

    fn open_device_with_fd(&self, fd: RawFd) -> Result<CtxDeviceImpl> {
        unsafe {
            let mut handle= null_mut();
            check_err(libusb_wrap_sys_device(self.ctx.0, fd as _, &mut handle))?;
            let dev = libusb_get_device(handle);
            let device = CtxDeviceImpl{
                dev,
                handle: Mutex::new(DeviceHandle(handle)),
                manager: None,
            };
            self.open_device();
            Ok(device)
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        self.exit();
        trace!("drop manager");
    }
}

#[derive(Clone, Debug, Copy)]
struct EventControllerCtx{
    device_count: usize,
    is_exit: bool,
}

