
use std::mem;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::*;
use libusb_src::*;
use crate::device::Device;


pub struct UsbManager{
    context:  *mut libusb_context,
    is_event_thread_run: Arc<AtomicBool>
}



unsafe  impl Send for UsbManager{}
unsafe impl Sync for UsbManager{}

struct Context(*mut libusb_context);
unsafe impl Send for Context{}


impl UsbManager {
    pub fn new()->Result<Self>{
        let mut context = mem::MaybeUninit::<*mut libusb_context>::uninit();

        let context = unsafe {
            let r = libusb_init(context.as_mut_ptr());
            check_err(r)?;
            context.assume_init()
        };

        let is_event_thread_run =Arc::new(AtomicBool::new(true));
        {
            let context = Context(context);
            let is_event_thread_run = is_event_thread_run.clone();

                std::thread::spawn(move || {
                    unsafe {
                        let ptr = context;
                        while is_event_thread_run.load(Ordering::SeqCst) {
                                libusb_handle_events(ptr.0);
                        }
                        println!("event_finish");
                    }
                });

        }

        Ok(Self{
            context,
            is_event_thread_run
        })
    }

    pub fn device_list(&self)->Result<DeviceList>{

        let list = unsafe {
            let mut devs_raw = mem::MaybeUninit::<*const *mut libusb_device>::uninit();
            let cnt = libusb_get_device_list(self.context, devs_raw.as_mut_ptr());
            check_err(cnt as _)?;

            let mut devs_raw = devs_raw.assume_init();
            DeviceList{
                ptr: devs_raw,
                i: 0,
                length: cnt as _
            }
        };

        Ok(list)
    }

    pub fn open_device_with_vid_pid(&self, vendor_id: usize, product_id: usize)->Result<Arc<Device>>{
        let list = self.device_list()?;
        for device in list {
            let desc= device.descriptor();
            if desc.idVendor == vendor_id as u16 && desc.idProduct == product_id as u16 {
                return  Ok(device);
            }
        }
        Err(Error::NotFound)
    }

}

impl Drop for UsbManager {
    fn drop(&mut self) {
        self.is_event_thread_run.store(false, Ordering::SeqCst);
        unsafe {
            libusb_exit(self.context);
        }
    }
}

pub struct  DeviceList{
    ptr: *const *mut libusb_device,
    i: usize,
    length: usize
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

            Device::new(dev)
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
    use std::time::Duration;
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


    #[tokio::test]
    async fn test_interface() {
        {
            let manager = UsbManager::new().unwrap();
            let mut device = manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap();

            println!("{} speed: {:?}", device, device.speed());

            // device.set_configuration(0x1).unwrap();
            let config = device.get_configuration().unwrap();

            println!("config: {}", config);

            let interface = device.get_interface(0).unwrap();

            interface.control_transfer().await.unwrap();


        }


        std::thread::sleep(Duration::from_secs(1));


    }

}