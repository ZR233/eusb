use crate::error::*;
use std::sync::{Arc};
use crate::adaptor::{CtxManager};
use crate::device::Device;
use crate::platform::*;
#[cfg(unix)]
use std::os::unix::io::RawFd;


#[derive(Clone)]
pub struct UsbManager{
    ctx: Arc<Manager>
}

pub struct UsbManagerBuilder{
    no_device_discovery: bool,
}

impl UsbManagerBuilder {
    pub fn default()->Self{
        Self{
            no_device_discovery:false
        }
    }

    #[cfg(libusb)]
    #[cfg(all(not(target_os = "android"), unix))]
    pub fn no_device_discovery(&mut self)-> &mut Self{
        self.no_device_discovery=true;
        self
    }


    #[cfg(libusb)]
    pub fn build(&mut self)->Result<UsbManager>{
        use libusb_src::*;
        use ptr::Context;
        let ctx = Context::new();
        #[cfg(target_os = "android")]
        {
            self.no_device_discovery=true;
        }


        if self.no_device_discovery {
            unsafe {

                let r = libusb_set_option(ctx.0, LIBUSB_OPTION_NO_DEVICE_DISCOVERY);
                check_err(r)?;
            }
        }
        let ctx = Manager::new(ctx)?;
        let ctx = Arc::new(ctx);
        Ok(UsbManager{
            ctx
        })

    }
}


impl UsbManager {
    pub fn builder()->UsbManagerBuilder{
        UsbManagerBuilder::default()
    }
    pub fn default()->Result<Self>{
        Self::builder().build()
    }

    #[cfg(not(target_os = "android"))]
    pub async fn device_list(&self)->Result<Vec<Device>>{
        let ctx = self.ctx.clone();

        let mut l = ctx.clone().device_list().await?;
        let mut out = vec![];
        while let Some(one) = l.pop()  {
            #[cfg(libusb)]
            let dev = Device::new(one, ctx.clone());
            out.push(dev);
        }
        Ok(out)
    }

    #[cfg(not(target_os = "android"))]
    pub async fn open_device_with_vid_pid(&self, vendor_id: usize, product_id: usize)->Result<Device>{
        let list = self.device_list().await?;
        for device in list {
            if device.vid() == vendor_id as u16 && device.pid() == product_id as u16 {
                return  Ok(device);
            }
        }
        Err(Error::NotFound)
    }

    /// Wrap a platform-specific system device handle and obtain a [Device] for the underlying device.
    ///
    /// The handle allows you to use perform I/O on the device in question.
    ///
    /// init with [UsbManagerBuilder::no_device_discovery] if you want to skip enumeration of USB devices. In particular, this might be needed on Android if you don't have authority to access USB devices in general.
    ///
    /// On Linux, the system device handle must be a valid file descriptor opened on the device node.
    ///
    /// The system device handle must remain open until [Device] is dropped. The system device handle will not be closed by [Drop].
    ///
    /// This is a non-blocking function; no requests are sent over the bus.
    #[cfg(unix)]
    pub fn open_device_with_fd(&self, fd: RawFd)->Result<Device>{
        let d= self.ctx.open_device_with_fd(fd)?;
        Ok(Device::new(d, self.ctx.clone()))
    }
}


