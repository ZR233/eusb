use std::mem;
use std::ptr::{slice_from_raw_parts};
use std::sync::Arc;
use log::debug;
use crate::error::*;
use libusb_src::*;
use crate::define::*;
use crate::device::*;

pub struct UsbManager{
    context:  *mut libusb_context,
    event_controller: Arc<EventController>
}


unsafe  impl Send for UsbManager{}
unsafe impl Sync for UsbManager{}

struct Context(*mut libusb_context);
unsafe impl Send for Context{}
unsafe impl Sync for Context{}


impl UsbManager {
    pub fn new()->Result<Self>{
        let mut context = mem::MaybeUninit::<*mut libusb_context>::uninit();

        let context = unsafe {
            let r = libusb_init(context.as_mut_ptr());
            check_err(r)?;
            context.assume_init()
        };

        let event_controller =Arc::new(EventController::new());
        {
            let context = Context(context);
            let event_controller = event_controller.clone();

                std::thread::spawn(move || {
                    let ptr = context;
                    let controller = event_controller;
                    let mut ctx = {
                        controller.ctx.lock().unwrap().clone()
                    };

                    unsafe {
                        while !ctx.is_exit{
                            // debug!("ctx: {:?}", ctx);
                            if ctx.device_count>0 {
                                // debug!("wait even");
                                libusb_handle_events(ptr.0);
                                // debug!("even ok");
                                ctx = {
                                    controller.ctx.lock().unwrap().clone()
                                };
                            }else{
                                // debug!("wait cvar");
                                let mut g = controller.ctx.lock().unwrap();
                                g = controller.cond.wait(g).unwrap();
                                // std::thread::sleep(Duration::from_millis(100));
                                ctx = g.clone();
                                // debug!("cond ok");
                            }
                        }
                        libusb_exit(ptr.0);
                        debug!("event_finish");
                    }
                });

        }

        Ok(Self{
            context,
            event_controller
        })
    }

    pub fn device_list(&self)->Result<DeviceList>{

        let list = unsafe {
            let mut devs_raw = mem::MaybeUninit::<*const *mut libusb_device>::uninit();
            let cnt = libusb_get_device_list(self.context, devs_raw.as_mut_ptr());
            check_err(cnt as _)?;

            let devs_raw = devs_raw.assume_init();
            DeviceList{
                ptr: devs_raw,
                i: 0,
                length: cnt as _,
                event_controller: self.event_controller.clone()
            }
        };

        Ok(list)
    }

    pub fn open_device_with_vid_pid(&self, vendor_id: usize, product_id: usize)->Result<Arc<Device>>{
        let list = self.device_list()?;
        for device in list {
            let desc= device.descriptor();
            if desc.id_vendor() == vendor_id as u16 && desc.id_product() == product_id as u16 {
                return  Ok(device);
            }
        }
        Err(Error::NotFound)
    }

}

impl Drop for UsbManager {
    fn drop(&mut self) {
        self.event_controller.exit();
    }
}

pub struct  DeviceList{
    ptr: *const *mut libusb_device,
    i: usize,
    length: usize,
    event_controller: Arc<EventController>
}

impl Iterator for DeviceList{
    type Item = Arc<Device>;

    fn next(&mut self) -> Option<Self::Item> {

        let device = unsafe {
            let list = slice_from_raw_parts(self.ptr, self.length);
            let dev =*((*list).get(self.i)?);
            if dev.is_null() {
                return None;
            }

            Device::new(dev, self.event_controller.clone())
        };

        self.i+=1;
        Some(Arc::new(device))
    }
}

impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe {
            libusb_free_device_list(self.ptr, 1);
        }
    }
}



#[cfg(test)]
mod test{
    use crate::core::UsbManager;

    #[test]
    fn test_device_list() {
        let manager = UsbManager::new().unwrap();
        let list = manager.device_list().unwrap();
        for x in list {
            let sp = x.speed();
            println!("{} speed: {:?}", x, sp);
        }
    }

    #[test]
    fn test_device_pid_vid() {
        let manager = UsbManager::new().unwrap();
        let device = manager.open_device_with_vid_pid(0x1d50,0x6089).unwrap();

        println!("{} speed: {:?}", device, device.speed());
    }

}