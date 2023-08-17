use crate::error::*;
use std::sync::{Arc};
use crate::adaptor::{CtxManager};
use crate::device::Device;
use crate::platform::*;


#[derive(Clone)]
pub struct UsbManager{
    ctx: Arc<Manager>
}


impl UsbManager {
    pub fn new()->Result<Self>{
        let ctx = Manager::new()?;
        let ctx = Arc::new(ctx);
        Ok(Self{
            ctx
        })
    }

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

    pub async fn open_device_with_vid_pid(&self, vendor_id: usize, product_id: usize)->Result<Device>{
        let list = self.device_list().await?;
        for device in list {
            if device.vid() == vendor_id as u16 && device.pid() == product_id as u16 {
                return  Ok(device);
            }
        }
        Err(Error::NotFound)
    }
}


