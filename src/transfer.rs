use std::future::Future;
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use futures::channel::mpsc::{channel, Receiver};
use futures::StreamExt;
use libusb_src::*;
use crate::define::{ControlTransferRequest, EndpointDirection};
use crate::error::*;
use crate::device::Device;
use crate::interface::{BulkTransferRequest, Interface};

pub struct Transfer{
    pub(crate) ptr: *mut libusb_transfer,
    pub(crate) buff: Vec<u8>,
    result_callback: Arc<Mutex<dyn FnMut(Result<Transfer>)>>,
}

unsafe impl Send for Transfer {}
unsafe impl Sync for Transfer {}

pub type ResultFuture<T> = Pin<Box<dyn Future<Output=T> + Send>>;

impl Transfer{
    pub fn new(
        iso_packets: usize,
        buff_size: usize,
    )->Result<Self>{
        let ptr = unsafe {
            let r = libusb_alloc_transfer(iso_packets as _);
            if r.is_null(){
                return Err(Error::Other("alloc transfer fail".to_string()));
            }
            r
        };

        Ok(Self{
            ptr,
            buff: vec![0; buff_size],
            result_callback:Arc::new(Mutex::new(|_|{}))
        })
    }

    pub fn control(
        device: &Device,
        request: ControlTransferRequest,
        direction: EndpointDirection,
        data_in_len: u16,
        data_out: &[u8]
    ) ->Result<Self>{
        let data_len = match direction {
            EndpointDirection::In => data_in_len,
            EndpointDirection::Out => data_out.len() as _,
        };

        let mut s = Self::new(0, LIBUSB_CONTROL_SETUP_SIZE + (data_len as usize))?;

        if direction== EndpointDirection::Out {
            for i in 0..data_out.len(){
                s.buff[i+LIBUSB_CONTROL_SETUP_SIZE] = data_out[i];
            }
        }

        unsafe {
            let buf_ptr = s.buff.as_mut_ptr();

            libusb_fill_control_setup(
                buf_ptr,
                (direction.to_libusb() | request.transfer_type.to_libusb() | request.recipient.to_libusb()) as u8,
                request.request,
                request.value,
                request.index,
                data_in_len);

            libusb_fill_control_transfer(
                s.ptr,
                device.get_handle()?,
                buf_ptr,
                Self::custom_cb,
                null_mut(),
                request.timeout.as_millis() as _,
            );
        }

        Ok(s)
    }
    pub fn bulk (
        interface: &Interface,
        request: BulkTransferRequest,
        direction: EndpointDirection,
        data_out: &[u8],
    )->Result<Self>{
        let mut s = Self::new(0, request.package_len)?;
        if direction== EndpointDirection::Out {
            for i in 0..data_out.len(){
                s.buff[i]= data_out[i]
            }
        }

        unsafe {
            let buf_ptr = s.buff.as_mut_ptr();

            libusb_fill_bulk_transfer(
                s.ptr,
                interface.dev_handle,
                (request.endpoint as u32 | direction.to_libusb()) as u8,
                buf_ptr,
                request.package_len as _,
                Self::custom_cb,
                null_mut(),
                request.timeout.as_millis() as _,
            );
        }
        Ok(s)
    }

    extern "system"  fn custom_cb(data: *mut libusb_transfer){
        unsafe {
            let user_data_ptr = (*data).user_data;

            let mut user_data = Box::from_raw(user_data_ptr as  *mut UserData);
            let cb = user_data.result_callback.clone();

            let result = match (*data).status {
                LIBUSB_TRANSFER_COMPLETED => {
                    Ok(user_data.transfer.take().unwrap())
                },
                LIBUSB_TRANSFER_OVERFLOW => Err(Error::Overflow),
                LIBUSB_TRANSFER_TIMED_OUT => Err(Error::Timeout),
                LIBUSB_TRANSFER_CANCELLED => Err(Error::Cancelled),
                LIBUSB_TRANSFER_STALL => Err(Error::NotSupported),
                LIBUSB_TRANSFER_NO_DEVICE => Err(Error::NoDevice),
                LIBUSB_TRANSFER_ERROR |_ => Err(Error::Other("Unknown".to_string())),
            };

            let mut cb = cb.lock().unwrap();
            (cb)(result);
        }
    }

    pub fn submit(transfer: Self)->Result<()>{
        unsafe {
            let ptr = transfer.ptr;
            let cb = transfer.result_callback.clone();
            let user_data = Box::new(UserData{
                transfer: Some(transfer),
                result_callback: cb,
            });

            let p =  Box::into_raw(user_data);
            (*ptr).user_data = p as _;
            check_err(libusb_submit_transfer(ptr))?;
        }
        Ok(())
    }

    pub fn submit_wait(mut transfer: Self)->Result<ResultFuture<Result<Self>>>{
        let mut rx = transfer.set_complete_cb();
        Transfer::submit(transfer)?;

        Ok( Box::pin(async move{
            let r = rx.next().await.ok_or(Error::NotFound
            )??;
            Ok(r)
        }))
    }


    #[allow(unused)]
    pub fn cancel(&self)->Result<()>{
        unsafe {
            check_err(libusb_cancel_transfer(self.ptr))?;
        }
        Ok(())
    }

    pub fn set_callback<F>(&mut self, callback: F)
        where F: FnMut (Result<Transfer>), F: 'static{
        self.result_callback = Arc::new(Mutex::new(callback));
    }

    pub(crate) fn set_complete_cb(&mut self)->Receiver<Result<Transfer>>{

        let (mut tx, rx) = channel::<Result<Transfer>>(1);

        let callback = move|result|{
            let _ = tx.try_send(result);
        };

        self.set_callback(callback);

        rx
    }

    pub fn actual_length(&self)->usize{
        (unsafe {
            (*self.ptr).actual_length
        }) as usize
    }
}
struct UserData{
    transfer: Option<Transfer>,
    result_callback: Arc<Mutex<dyn FnMut(Result<Transfer>)>>,
}


impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.ptr)
        }
    }
}

pub struct TransferIn{
    pub(crate) transfer: Option<Transfer>,
}

impl TransferIn{
    pub async fn repeat(&mut self)->Result<Self>{
        let mut transfer = self.transfer.take().unwrap();
        let r = Transfer::submit_wait(transfer)?.await?;

        Ok(Self{ transfer: Some(r) })
    }

    pub fn data(&self)->&[u8]{
        match &self.transfer {
            None => { &[] }
            Some(transfer) => {
                let len = transfer.actual_length();
                let buff = transfer.buff.as_slice();
                &buff[0..len]
            }
        }
    }
}
