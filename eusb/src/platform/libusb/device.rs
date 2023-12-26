use crate::platform::DeviceCtx;
use libusb_src::*;
pub(crate) struct DeviceCtxImpl {
    dev: Device
}

impl From<Device> for DeviceCtxImpl{
    fn from(value: Device) -> Self {
        Self{
            dev: value
        }
    }
}

impl DeviceCtxImpl{

}

impl DeviceCtx for DeviceCtxImpl {

}


pub(crate) struct Device(*mut libusb_device);
unsafe impl Send for Device{}
unsafe impl Sync for Device{}

impl From<*mut libusb_device> for Device {
    fn from(value: *mut libusb_device) -> Self {
        Self(value)
    }
}

impl Drop for Device{
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.0)
        }
    }
}

impl Device{

}
