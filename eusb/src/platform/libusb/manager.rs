use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use log::{debug};
use thread_priority::{ThreadBuilderExt, ThreadPriority};
use crate::device::UsbDevice;
use crate::platform::libusb::context::Context;
use crate::platform::{DeviceCtxImpl, ManagerCtx};
use super::errors::*;

pub(crate) struct ManagerCtxImpl {
    ctx: Arc<Context>,
    event: Arc<Mutex<EventControllerCtx>>,
    cond: Arc<Condvar>,
    join: Mutex<Option<JoinHandle<()>>>,
}


impl ManagerCtx for ManagerCtxImpl {
    fn new() -> Self {
        let ctx = Arc::new(Context::new());
        let event = Arc::new(Mutex::new(EventControllerCtx {
            device_count: 0,
            is_exit: false,
        }));
        let cond = Arc::new(Condvar::new());
        let join = work_event(ctx.clone(), event.clone(), cond.clone());

        Self {
            ctx,
            event,
            cond,
            join: Mutex::new(Some(join)),
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

    fn open_device_with_vid_pid(&self, vid: u16, pid: u16) -> Result<UsbDevice> {
        let handle = self.ctx.open_device_with_vid_pid(vid, pid)?;
        let dev = DeviceCtxImpl::from(handle);
        Ok(dev.into())
    }


    fn close(&self) {
        {
            let mut ctx = self.event.lock().unwrap();
            ctx.is_exit = true;
            self.cond.notify_all();
        }
        let mut g = self.join.lock().unwrap();
        if let Some(j) = g.take() {
            j.join().unwrap();
        }
    }
}


impl ManagerCtxImpl {

    pub(crate)  fn open_device(&self){
        let mut ctx = self.event.lock().unwrap();
        ctx.device_count+=1;
        debug!("device cnt: {}", ctx.device_count);
        self.cond.notify_all();
    }

    pub(crate)  fn close_device(&self){
        let mut ctx = self.event.lock().unwrap();
        ctx.device_count-=1;
        debug!("device cnt: {}", ctx.device_count);
        self.cond.notify_all();
    }
}

fn work_event(
    ctx: Arc<Context>,
    event: Arc<Mutex<EventControllerCtx>>,
    cond: Arc<Condvar>,
) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("USB main event".into()).spawn_with_priority(ThreadPriority::Max, move |result| {
        if result.is_err() {
            println!("Set priority result fail: {:?}", result);
        }

        let p = ctx;
        let mut ctx = {
            *event.lock().unwrap()
        };

        while !ctx.is_exit {
            if ctx.device_count > 0 {
                let _ = p.handle_events();
                ctx = *event.lock().unwrap()
            } else {
                let mut g = event.lock().unwrap();
                g = cond.wait(g).unwrap();
                ctx = *g;
            }
        }
        p.exit();
    }).unwrap()
}

#[derive(Clone, Debug, Copy)]
struct EventControllerCtx {
    device_count: usize,
    is_exit: bool,
}
