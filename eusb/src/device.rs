use std::sync::{Arc};

#[cfg(libusb)]
use crate::platform::libusb::*;

#[derive(Clone)]
pub struct UsbDevice{
    ctx: Arc<Device>
}


impl UsbDevice{

    #[cfg(libusb)]
    pub(crate) fn new(mut value: Device, manager: Arc<Manager>) -> Self {
        value.manager=Some(manager);
        let ctx = Arc::new(value);
        Self{
            ctx
        }
    }
}