use std::ptr::slice_from_raw_parts;
use std::sync::{Arc};
use std::time::Duration;
use libusb_src::{libusb_clear_halt, libusb_submit_transfer, LIBUSB_SUCCESS, libusb_transfer, LIBUSB_TRANSFER_CANCELLED};
use crate::define::PipConfig;
use crate::platform::libusb::device_handle::{DeviceHandle, TransferDirection};
use crate::platform::libusb::transfer::{ToResult, Transfer};
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::{FutureExt, StreamExt};
use futures::future::LocalBoxFuture;
use log::{trace, warn};
use super::errors::*;
use crate::platform::EndpointPipInInner;

pub(crate) struct EndpointPipInImpl {
    transfers: Vec<Transfer>,
    rx: Receiver<Vec<u8>>,
}

unsafe impl Send for EndpointPipInImpl {}


impl Drop for EndpointPipInImpl {
    fn drop(&mut self) {
        for t in &self.transfers {
            let _ = t.cancel();
        }
        loop {
            std::thread::sleep(Duration::from_micros(50));
            let mut all_ok = true;
            for t in &self.transfers{
                let r = t.result();
                if r.is_ok(){
                    all_ok = false;
                    break;
                }
            }
            if all_ok{
                break;
            }
        }
    }
}

impl EndpointPipInImpl {
    pub fn new(handle: &Arc<DeviceHandle>, endpoint: u8, config: PipConfig) -> Result<Self> {
        let handle_ptr = handle.ptr;
        let mut transfers = Vec::with_capacity(config.request_num);
        let (tx, rx) = channel::<Vec<u8>>(config.cache_size);

        unsafe {
            for _ in 0..transfers.capacity() {
                let tb = Box::new(tx.clone());
                let tx_ptr = Box::into_raw(tb);

                let mut transfer = Transfer::bulk_transfer(endpoint, pip_cb, TransferDirection::In { len: config.package_size }, config.timeout);
                transfer.set_handle(handle_ptr);
                transfer.set_user_data(tx_ptr as _);
                transfer.submit()?;
                transfers.push(transfer);
            }
        }
        Ok(Self {
            transfers,
            rx,
        })
    }
}


impl EndpointPipInInner for EndpointPipInImpl {
    fn next(&mut self) -> LocalBoxFuture<Option<Vec<u8>>> {
        self.rx.next().boxed_local()
    }
}


extern "system" fn pip_cb(transfer: *mut libusb_transfer) {
    unsafe {
        let tx_ptr = (*transfer).user_data as *mut Sender<Vec<u8>>;
        let result = (*transfer).to_result();
        let mut tx = Box::from_raw(tx_ptr);
        if tx.is_closed() {
            trace!("tx closed");
            (*transfer).status = LIBUSB_TRANSFER_CANCELLED;
            return;
        }

        match result {
            Ok(_) => {
                let data = (*slice_from_raw_parts(
                    (*transfer).buffer as *const u8,
                    (*transfer).actual_length as _)).to_vec();

                if tx.try_send(data).is_err() {
                    warn!("ep[{}] overflow", (*transfer).endpoint);
                }
            }
            Err(e) => {
                match e {
                    Error::Pipe => {
                        warn!("pip error");
                        if libusb_clear_halt((*transfer).dev_handle, (*transfer).endpoint) != LIBUSB_SUCCESS {
                            return;
                        }
                    }
                    _ => {
                        trace!("transfer err: {}" ,e);
                        return;
                    }
                }
            }
        }
        (*transfer).user_data = Box::into_raw(tx) as _;
        libusb_submit_transfer(transfer);
    }
}
