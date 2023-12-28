use std::sync::{Arc, Weak};
use libusb_src::libusb_transfer;
use crate::define::PipConfig;
use crate::platform::EndpointInInner;
use crate::platform::libusb::device_handle::{DeviceHandle, TransferDirection};
use crate::platform::libusb::transfer::Transfer;
use futures::channel::mpsc::channel;

pub(crate) struct EndpointCtx{
    endpoint: u8,
    config: PipConfig,
    handle: Weak<DeviceHandle>,
}


pub(crate) struct EndpointInImpl{
    inner: EndpointCtx,
}


impl EndpointCtx{
    pub fn new(handle: &Arc<DeviceHandle>, endpoint: u8, config: PipConfig)->Self{
        let handle_ptr = handle.ptr;
        let handle = Arc::downgrade(handle);
        let mut transfers = Vec::with_capacity(config.pip_size);
        let (tx, rx) = channel::<Vec<u8>>(config.pip_size);


        unsafe {
            for _ in 0..config.pip_size {
                let mut transfer = Transfer::bulk_transfer(endpoint, pip_cb, TransferDirection::In {len: config.package_size}, config.timeout);
                transfer.set_handle(handle_ptr);
                let tb = Box::new(tx.clone());
                let txp = Box::into_raw(tb);
                transfer.set_user_data(txp as _);
                transfers.push(transfer);
            }
        }

        Self{
            endpoint,
            handle,
            config
        }
    }
}
impl EndpointInImpl{
    pub fn new(handle: &Arc<DeviceHandle>, endpoint: u8, config:PipConfig)->Self{
        Self{
            inner: EndpointCtx::new(handle, endpoint,config)
        }
    }
}


impl EndpointInInner for EndpointInImpl{

}


extern "system" fn pip_cb(transfer: *mut libusb_transfer){

}
