use crate::platform::libusb::device::UsbDeviceCtx;
use crate::platform::Platform;

pub(crate) struct PlatformLibUsb{

}


impl Platform<UsbDeviceCtx> for PlatformLibUsb{
    fn new() -> Self {
        Self{

        }
    }

    async fn device_list(&self) -> Vec<UsbDeviceCtx> {
        vec![]
    }
}

