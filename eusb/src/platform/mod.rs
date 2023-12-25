pub(crate) mod libusb;
pub(crate) use libusb::context::PlatformLibUsb;



pub(crate) trait DeviceCtx{

}

pub(crate) trait Platform<D: DeviceCtx>{
    fn new()->Self;
    async fn device_list(&self)->Vec<D>;
}
