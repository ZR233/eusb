use std::sync::{Arc};
use log::{error, warn};

#[cfg(libusb)]
use crate::adaptor::libusb::*;
use crate::adaptor::CtxDevice;

#[derive(Clone)]
pub struct Device{
    ctx: Arc<CtxDeviceImpl>
}


impl Device{

    #[cfg(libusb)]
    pub(crate) fn new(mut value: CtxDeviceImpl, manager: Arc<Manager>) -> Self {
        value.manager=Some(manager);
        let ctx = Arc::new(value);
        Self{
            ctx
        }
    }

    pub fn pid(&self)->u16{
        self.ctx.pid()
    }

    pub fn vid(&self)->u16{
        self.ctx.vid()
    }

    pub async fn serial_number(&self)->String{
        match self.ctx.serial_number().await{
            Ok(s) => {s}
            Err(e) => {
                warn!("{}",e);
                String::new()
            }
        }
    }
}