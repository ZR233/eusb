use std::mem;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::{Arc};
use crate::error::*;
use libusb_src::*;
use crate::define::*;
use crate::device::*;

#[derive(Clone)]
pub struct UsbManager{
    pub(crate) ctx: Arc<Context>,
}

pub(crate) struct Context{
    pub(crate) context: LibusbContext,
    pub(crate) event_controller: Arc<EventController>
}

pub(crate) struct LibusbContext(pub(crate) *mut libusb_context);
unsafe impl Send for LibusbContext{}
unsafe impl Sync for LibusbContext{}

impl LibusbContext {
    fn new(mut ptr:  *mut libusb_context) ->Result<Self>{
        unsafe {
            let r = libusb_init(&mut ptr);
            check_err(r)?;
            Ok(Self(ptr))
        }
    }
}
impl Drop for LibusbContext {
    fn drop(&mut self) {
        unsafe {
            libusb_exit(self.0)
        }
    }
}

pub struct UsbOption{
    ptr: *mut libusb_context
}

impl UsbOption {

    #[cfg(windows)]
    pub fn use_usbdk(&mut self)-> Result<&mut Self> {
        unsafe {
            let r = libusb_set_option(self.ptr, LIBUSB_OPTION_USE_USBDK);
            check_err(r)?;
        }
        Ok(self)
    }

    #[cfg(all(not(target_os = "android"), unix))]
    pub fn no_device_discovery(&mut self)-> Result<&mut Self>{
        unsafe {
            let r = libusb_set_option(self.ptr, LIBUSB_OPTION_NO_DEVICE_DISCOVERY);
            check_err(r)?;
        }
        Ok(self)
    }

    pub fn init(&self)->Result<UsbManager>{
        UsbManager::from_libusb(self.ptr)
    }
}


impl UsbManager {
    pub fn builder()->UsbOption{
        return UsbOption{ ptr: null_mut() }
    }
    fn from_libusb(libusb: *mut libusb_context) -> Result<Self>{
        let libusb_ctx = LibusbContext::new(libusb)?;

        let s = Self{
            ctx: Arc::new(Context{
                context: libusb_ctx,
                event_controller: Arc::new(EventController::new(libusb)),
            }),
        };

        s.work_event();
        Ok(s)
    }

    fn work_event(&self){
        let con = self.ctx.event_controller.clone();

        std::thread::spawn(move || {

            let mut ctx ={
                con.ctx.lock().unwrap().clone()
            };

            unsafe {
                while !ctx.is_exit{
                    // debug!("ctx: {:?}", ctx);
                    if ctx.device_count > 0 {
                        // debug!("wait even");
                        libusb_handle_events(con.libusb.0);
                        // debug!("even ok");
                        ctx = con.ctx.lock().unwrap().clone()
                    }else{
                        // debug!("wait cvar");
                        let mut g = con.ctx.lock().unwrap();
                        g = con.cond.wait(g).unwrap();
                        ctx = g.clone();
                        // debug!("cvar ok");
                    }
                }
                libusb_exit(con.libusb.0);
                // debug!("event_finish");
            }
        });

    }


    pub fn init_default()->Result<Self>{
        Self::builder().init()
    }

    pub fn device_list(&self)->Result<DeviceList>{

        let list = unsafe {
            let mut devs_raw = mem::MaybeUninit::<*const *mut libusb_device>::uninit();
            let cnt = libusb_get_device_list(self.ctx.context.0, devs_raw.as_mut_ptr());
            check_err(cnt as _)?;

            let devs_raw = devs_raw.assume_init();
            DeviceList{
                ptr: devs_raw,
                i: 0,
                length: cnt as _,
                manager: self.clone()
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

impl Drop for Context {
    fn drop(&mut self) {
        self.event_controller.exit();
    }
}

pub struct  DeviceList{
    ptr: *const *mut libusb_device,
    i: usize,
    length: usize,
    manager: UsbManager
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

            Device::new(dev, &self.manager)
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
    #[cfg(not(windows))]
    #[test]
    fn test_device_option() {
        let manager = UsbManager::builder()
            .no_device_discovery().unwrap().init().unwrap();
        let list = manager.device_list().unwrap();
        for x in list {
            let sp = x.speed();
            println!("{} speed: {:?}", x, sp);
        }
    }
    #[test]
    fn test_device_list() {
        let manager = UsbManager::init_default().unwrap();
        let list = manager.device_list().unwrap();
        for x in list {
            let sp = x.speed();
            println!("{} speed: {:?}", x, sp);
        }
    }

    // #[test]
    // fn test_device_pid_vid() {
    //     let manager = UsbManager::init_default().unwrap();
    //     let r = manager.open_device_with_vid_pid(0x1d50,0x6089);
    //     match r {
    //         Ok(device) => {
    //             println!("{} speed: {:?}", device, device.speed());
    //         }
    //         Err(_) => {
    //             println!("not found");
    //         }
    //     }
    // }

}