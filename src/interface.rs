use std::ffi::{c_int};
use std::ptr::{slice_from_raw_parts_mut};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use futures::StreamExt;
use log::{debug, warn};
use libusb_src::*;
use crate::define::EndpointDirection;
use crate::error::*;
use crate::transfer::TransferBase;


pub struct  Interface{
    number: c_int,
    pub(crate) dev_handle: *mut libusb_device_handle,
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

    // async fn bulk_transfer(
    //     &self,
    //     request: BulkTransferRequest,
    //     direction: EndpointDirection,
    //     buf: &mut [u8],
    //     transfer_type: u8,
    // )->Result<usize>{
    //     let mut transfer = Transfer::bulk(
    //         &self,
    //         request,
    //         direction,
    //         buf,
    //     )?;
    //     unsafe {
    //         (*transfer.ptr).transfer_type = transfer_type;
    //     }
    //
    //     let mut rx = transfer.set_complete_cb();
    //
    //     Transfer::submit(transfer)?;
    //
    //     let r = rx.next().await.ok_or(Error::NotFound
    //     )??;
    //
    //     Ok(r.actual_length())
    // }
    pub async fn bulk_transfer_in(&self, request: BulkTransferRequest) -> Result<TransferBase>{
        let mut transfer = TransferBase::bulk(
            self,
                request,
                EndpointDirection::In,
                &[],

        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_BULK;
        }
        let t2 = transfer.submit()?.await?;

        Ok(t2)
    }
    pub async fn bulk_transfer_out(&self, request: BulkTransferRequest, data: &mut[u8])-> Result<()> {
        let mut transfer = TransferBase::bulk(
            self,
            request,
            EndpointDirection::Out,
            data,
        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_BULK;
        }
        let t2 = transfer.submit()?.await?;
        if t2.actual_length() != data.len() {
            return  Err(Error::Io(format!("send {}, actual {}", data.len(), t2.actual_length())))
        }
        Ok(())


        // let actual_length = self.bulk_transfer(
        //     request,
        //     EndpointDirection::Out,
        //     data,
        //     LIBUSB_TRANSFER_TYPE_BULK
        // ).await?;
        //
        // if actual_length != data.len() {
        //     return  Err(Error::Io(format!("send {}, actual {}", data.len(), actual_length)))
        // }
        // Ok(())
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
                let mut callback_data = Box::new(TransferIn::new(
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

    pub async fn interrupt_transfer_in(&self, request: BulkTransferRequest) -> Result<Vec<u8>>{
        let mut transfer = TransferBase::bulk(
            self,
            request,
            EndpointDirection::In,
            &[],

        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_INTERRUPT;
        }
        let t2 = transfer.submit()?.await?;

        let mut data = Vec::with_capacity(t2.actual_length());
        for i in 0..t2.actual_length() {
            data.push(t2.buff[i]);
        }
        Ok(data)
        // let mut buf = vec![0u8; request.package_len as _];
        // let actual_length = self.bulk_transfer(
        //     request,
        //     EndpointDirection::In,
        //     buf.as_mut_slice(),
        //     LIBUSB_TRANSFER_TYPE_INTERRUPT
        // ).await?;
        // buf.resize(actual_length, 0);
        // Ok(buf)
    }
    pub async fn interrupt_transfer_out(&self, request: BulkTransferRequest, data: &mut[u8])-> Result<()> {
        let mut transfer = TransferBase::bulk(
            self,
            request,
            EndpointDirection::Out,
            data,
        )?;
        unsafe {
            (*transfer.ptr.0).transfer_type=LIBUSB_TRANSFER_TYPE_INTERRUPT;
        }
        let t2 = transfer.submit()?.await?;
        if t2.actual_length() != data.len() {
            return  Err(Error::Io(format!("send {}, actual {}", data.len(), t2.actual_length())))
        }
        Ok(())
        // let actual_length = self.bulk_transfer(
        //     request,
        //     EndpointDirection::Out,
        //     data,
        //     LIBUSB_TRANSFER_TYPE_INTERRUPT
        // ).await?;
        //
        // if actual_length != data.len() {
        //     return  Err(Error::Io(format!("send {}, actual {}", data.len(), actual_length)))
        // }
        // Ok(())
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



struct TransferIn {
    buff: Vec<u8>,
    tx: futures::channel::mpsc::Sender<Vec<u8>>,
    cancel: Arc<BulkInCancel>,
    ptr: *mut libusb_transfer,
}

impl TransferIn {
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

impl Drop for TransferIn {
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

        let data_ptr = (*data).user_data as *mut TransferIn;

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
