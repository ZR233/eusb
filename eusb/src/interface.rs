use std::ffi::{c_int};
use std::ptr::{slice_from_raw_parts_mut};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{debug, warn};
use libusb_src::*;
use crate::define::EndpointDirection;
use crate::error::*;
use crate::prelude::TransferOut;
use crate::transfer::{Transfer, TransferIn, TransferWarp};

pub struct  InterfaceDescriptor{
    pub InterfaceNumber: u8,
    pub AlternateSetting: u8,
    pub NumEndpoints: u8,
    pub InterfaceClass: u8,
    pub InterfaceSubClass: u8,
    pub InterfaceProtocol: u8,
}

impl InterfaceDescriptor {
    pub(crate) fn from_ptr(ptr: *mut libusb_interface_descriptor){

    }
}


pub struct  Interface{
    number: c_int,
    pub(crate) dev_handle: *mut libusb_device_handle,
    is_claim: bool,
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
    pub(crate) fn new_claimed(dev_handle: *mut libusb_device_handle, index: usize) ->Result<Self>{
        let number = index as c_int;
        let mut s = Self{
            number,
            dev_handle,
            is_claim: true
        };
        s.claim()?;
        Ok(s)
    }

    pub fn claim(&mut self)->Result<()>{
        unsafe {
            let r = libusb_claim_interface(self.dev_handle, self.number);
            check_err(r)?;
        }
        Ok(())
    }


    pub fn bulk_transfer_in_request(&self, request: BulkTransferRequest)-> Result<TransferIn>{
        let transfer = Transfer::bulk(
            self,
            request,
            EndpointDirection::In,
            &[],
        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_BULK;
        }
        Ok(TransferIn::from_base(transfer))
    }
    pub fn bulk_transfer_out_request(&self, request: BulkTransferRequest, data: &mut[u8])-> Result<TransferOut> {
        let transfer = Transfer::bulk(
            self,
            request,
            EndpointDirection::Out,
            data,
        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_BULK;
        }
        Ok(TransferOut::from_base(transfer))
    }
    pub fn interrupt_transfer_in_request(&self, request: BulkTransferRequest)-> Result<TransferIn>{
        let transfer = self.bulk_transfer_in_request(request)?;
        unsafe {
            (*transfer.base.as_ref().unwrap().ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_INTERRUPT;
        }
        Ok(transfer)
    }
    pub fn interrupt_transfer_out_request(&self, request: BulkTransferRequest, data: &mut[u8])-> Result<TransferOut> {
        let transfer = self.bulk_transfer_out_request(request, data)?;
        unsafe {
            (*transfer.base.as_ref().unwrap().ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_INTERRUPT;
        }
        Ok(transfer)
    }


    pub async fn bulk_transfer_in(&self, request: BulkTransferRequest) ->Result<Vec<u8>>{
        let t1 = self.bulk_transfer_in_request(request)?;
        let t2 = t1.submit()?.await?;
        Ok(Vec::from(t2.data()))
    }
    pub async fn bulk_transfer_out(&self, request: BulkTransferRequest, data: &mut[u8])-> Result<usize> {
        let t1 = self.bulk_transfer_out_request(request, data)?;
        let t2 = t1.submit()?.await?;
        Ok(t2.actual_length())
    }

    pub fn open_bulk_in_channel(
        &self,
        request: BulkTransferRequest,
        option: BulkChannelOption,
    )->Result<futures::channel::mpsc::Receiver<Vec<u8>>>{

        let (tx, rx) = futures::channel::mpsc::channel::<Vec<u8>>(option.channel_size);
        unsafe {
            let bulk_cancel = Arc::new(BulkInCancel::new()) ;

            for _ in 0..option.request_size{
                let mut callback_data = Box::new(BulkTransferIn::new(
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
                    libusb_transfer_cb_fn_channel_in,
                    user_data as _,
                    request.timeout.as_millis() as _
                );

                let r = libusb_submit_transfer(transfer);
                check_err(r)?;

            }
        }
        Ok(rx)
    }

    pub async fn interrupt_transfer_in(&mut self, request: BulkTransferRequest) -> Result<Vec<u8>>{
        let transfer = self.interrupt_transfer_in_request(request)?;
        let t2 = transfer.submit()?.await?;
        Ok(Vec::from(t2.data()))
    }
    pub async fn interrupt_transfer_out(&mut self, request: BulkTransferRequest, data: &mut[u8])-> Result<usize> {
        let t1 = self.interrupt_transfer_out_request(request, data)?;
        let t2 = t1.submit()?.await?;
        Ok(t2.actual_length())
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        debug!("Release interface.");
        unsafe {
            if self.is_claim {
                libusb_release_interface(self.dev_handle, self.number);
            }

        }
    }
}
struct TransferPtr(*mut libusb_transfer);
unsafe impl Send for TransferPtr{}
struct BulkInCancel {
    transfers: Mutex<Vec<TransferPtr>>
}

impl BulkInCancel {
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



struct BulkTransferIn {
    buff: Vec<u8>,
    tx: futures::channel::mpsc::Sender<Vec<u8>>,
    cancel: Arc<BulkInCancel>,
    ptr: *mut libusb_transfer,
}

impl BulkTransferIn {
    fn new(
        tx: futures::channel::mpsc::Sender<Vec<u8>>,
        package_len: usize,
        cancel: &Arc<BulkInCancel>,
    ) ->Result<Self>{
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

impl Drop for BulkTransferIn {
    fn drop(&mut self) {
        self.tx.close_channel();
        self.cancel.cancel();
        unsafe {
            libusb_free_transfer(self.ptr);
        }
    }
}

extern "system" fn libusb_transfer_cb_fn_channel_in(data: *mut libusb_transfer){
    unsafe {

        let data_ptr = (*data).user_data as *mut BulkTransferIn;

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