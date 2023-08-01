use std::ffi::{c_int};
use std::mem;
use std::ptr::{null_mut, slice_from_raw_parts_mut};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{debug, warn};
use tokio::time::Instant;
use libusb_src::*;
use crate::error::*;


pub struct  Interface{
    number: c_int,
    dev_handle: *mut libusb_device_handle,
}

unsafe impl Send for Interface{}
unsafe impl Sync for Interface{}

pub struct BulkTransferRequest{
    pub endpoint: u8,
    pub package_len: usize,
    pub timeout: Duration,
}
pub struct BulkChannelOption{
    pub channel_size: usize,
    pub request_size: usize,
}

impl Default for BulkChannelOption {
    fn default() -> Self {
        Self{
            channel_size: 4,
            request_size: 4,
        }
    }
}

impl Interface {
    pub(crate) fn new(dev_handle: *mut libusb_device_handle, index: usize)->Result<Self>{
        let number = index as c_int;
        unsafe {
            let r = libusb_claim_interface(dev_handle, number);
            check_err(r)?;
        }

        Ok(Self{
            number,
            dev_handle,
        })
    }
    pub fn open_bulk_in_channel(
        &self,
        request: BulkTransferRequest,
        option: BulkChannelOption,
    )->Result<futures::channel::mpsc::Receiver<Vec<u8>>>{
        let (tx, rx) = futures::channel::mpsc::channel::<Vec<u8>>(option.channel_size);
        unsafe {
            let bulk_cancel = Arc::new(BulkCancel::new()) ;

            for _ in 0..option.request_size{
                let mut callback_data = Box::new(Transfer::new(
                    tx.clone(),
                    request.package_len,
                    &bulk_cancel,
                )?);
                let transfer = callback_data.ptr;
                let buff_ptr = callback_data.buff.as_mut_ptr();
                let user_data = Box::into_raw(callback_data);


                libusb_fill_bulk_transfer(
                    transfer,
                    self.dev_handle,
                    (request.endpoint as u32 | LIBUSB_ENDPOINT_IN as u32) as u8,
                    buff_ptr,
                    request.package_len as _,
                    libusb_transfer_cb_fn_channel,
                    user_data as _,
                    request.timeout.as_millis() as _
                );

                let r = libusb_submit_transfer(transfer);
                check_err(r)?;

            }
        }
        Ok(rx)
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        debug!("Release interface.");
        unsafe {
            libusb_release_interface(self.dev_handle, self.number);
        }
    }
}
struct TransferPtr(*mut libusb_transfer);
unsafe impl Send for TransferPtr{}
struct BulkCancel {
    transfers: Mutex<Vec<TransferPtr>>
}

impl BulkCancel {
    fn new()->Self{
        Self{
            transfers: Mutex::new(vec![])
        }
    }
    fn add(&self, transfer: *mut libusb_transfer){
        let mut l = self.transfers.lock().unwrap();
        (*l).push(TransferPtr(transfer));
    }

    fn cancel(&self){
        let mut l = self.transfers.lock().unwrap();
        while let Some(one) = l.pop() {
            unsafe {
                libusb_cancel_transfer(one.0);
            }
        }
    }
}


struct Transfer {
    buff: Vec<u8>,
    tx: futures::channel::mpsc::Sender<Vec<u8>>,
    cancel: Arc<BulkCancel>,
    ptr: *mut libusb_transfer,
}

impl Transfer {
    fn new(
        tx: futures::channel::mpsc::Sender<Vec<u8>>,
        package_len: usize,
        cancel: &Arc<BulkCancel>,
    )->Result<Self>{
        unsafe {
            let transfer = libusb_alloc_transfer(0);
            if transfer.is_null(){
                return Err(Error::NoMem);
            }
            cancel.add(transfer);
            Ok(Self{
                buff: vec![0u8; package_len],
                tx,
                cancel: cancel.clone(),
                ptr: transfer,
            })
        }

    }
}

impl Drop for Transfer {
    fn drop(&mut self) {
        self.tx.close_channel();
        self.cancel.cancel();
        unsafe {
            libusb_free_transfer(self.ptr);
        }
    }
}

extern "system" fn libusb_transfer_cb_fn_channel(data: *mut libusb_transfer){
    unsafe {

        let data_ptr = (*data).user_data as *mut Transfer;

        if (*data).status != LIBUSB_TRANSFER_COMPLETED {
            debug!("bulk transfer stop: {}", (*data).status);
            let _ = Box::from_raw(data_ptr);
            return;
        }

        let mut out = Vec::with_capacity((*data).actual_length as _);
        let buff = &*slice_from_raw_parts_mut((*data).buffer, (*data).actual_length as usize);
        for i in 0..out.capacity() {
            out.push(buff[i]);
        }

        match  (*data_ptr).tx.try_send(out){
            Ok(_) => {}
            Err(e) => {
                if e.is_full() {
                    warn!("bulk_transfer full");
                }else {
                    debug!("bulk transfer stop: channel");
                    let _ = Box::from_raw(data_ptr);
                    return;
                }
            }
        };

        match check_err(libusb_submit_transfer(data)) {
            Ok(_) => {}
            Err(e) => {
                debug!("bulk transfer stop: {}", e);
                let _ = Box::from_raw(data_ptr);
            }
        }

    }
}
