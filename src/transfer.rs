use std::future::Future;
use std::pin::Pin;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use futures::channel::mpsc::*;
use futures::StreamExt;
use libusb_src::*;
use crate::define::{ControlTransferRequest, EndpointDirection};
use crate::error::*;
use crate::device::Device;
use crate::interface::{BulkTransferRequest, Interface};
use pin_project::pin_project;


pub struct TransferBase {
    pub(crate) ptr: *mut libusb_transfer,
    pub(crate) buff: Vec<u8>,
    result_callback: Arc<Mutex<dyn FnMut(Result<TransferBase>)>>,
    complete_send: Sender<Result<TransferBase>>,
    complete_signal: Option<Receiver<Result<TransferBase>>>
}

unsafe impl Send for TransferBase {}
unsafe impl Sync for TransferBase {}


struct LibusbTransfer(*mut libusb_transfer);
unsafe impl Send for LibusbTransfer {}
unsafe impl Sync for LibusbTransfer {}


pub type ResultFuture<T> = Pin<Box<dyn Future<Output=T> + Send>>;

impl TransferBase {
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
        let (mut tx, rx) = channel::<Result<TransferBase>>(1);
        Ok(Self{
            ptr,
            buff: vec![0; buff_size],
            result_callback:Arc::new(Mutex::new(|_|{})),
            complete_send: tx,
            complete_signal:  Some(rx)
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
        s.set_complete_cb2();
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
        s.set_complete_cb2();
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

    pub fn submit_wait(mut self)->Result<SubmitHandle>{
        let inner = self.ptr;
        let mut rx = self.complete_signal.take().unwrap();
        TransferBase::submit(self)?;

        Ok(SubmitHandle{
            future: Box::pin(async move{
                let mut r = rx.next().await.ok_or(Error::NotFound
                )??;
                r.complete_signal = Some(rx);
                Ok(r)
            }),
            inner: LibusbTransfer(inner)
        })
        //
        // Ok(Box::pin(async move{
        //     let mut r = rx.next().await.ok_or(Error::NotFound
        //     )??;
        //     r.complete_signal = Some(rx);
        //     Ok(r)
        // }))
    }


    #[allow(unused)]
    pub fn cancel(&self)->Result<()>{
        unsafe {
            check_err(libusb_cancel_transfer(self.ptr))?;
        }
        Ok(())
    }

    pub fn set_callback<F>(&mut self, callback: F)
        where F: FnMut (Result<TransferBase>), F: 'static{
        self.result_callback = Arc::new(Mutex::new(callback));
    }

    pub(crate) fn set_complete_cb(&mut self)->Receiver<Result<TransferBase>>{

        let (mut tx, rx) = channel::<Result<TransferBase>>(1);

        let callback = move|result|{
            let _ = tx.try_send(result);
        };

        self.set_callback(callback);

        rx
    }
    pub(crate) fn set_complete_cb2(&mut self){
        let mut tx = self.complete_send.clone();

        let callback = move|result|{
            let _ = tx.try_send(result);
        };

        self.set_callback(callback);
    }

    pub fn actual_length(&self)->usize{
        (unsafe {
            (*self.ptr).actual_length
        }) as usize
    }
}

#[pin_project]
pub struct SubmitHandle {
    #[pin]
    future: ResultFuture<Result<TransferBase>>,
    inner: LibusbTransfer
}

impl SubmitHandle {
    pub fn cancel(&self){
        unsafe {
            libusb_cancel_transfer(self.inner.0);
        }
    }
}

impl Future for SubmitHandle{
    type Output = Result<TransferBase>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(t) => { Poll::Ready(t) }
            Poll::Pending => { Poll::Pending }
        }
    }
}





struct UserData{
    transfer: Option<TransferBase>,
    result_callback: Arc<Mutex<dyn FnMut(Result<TransferBase>)>>,
}


impl Drop for TransferBase {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.ptr)
        }
    }
}













pub struct TransferIn{
    pub(crate) transfer: Option<TransferBase>,
}

impl TransferIn{
    pub async fn repeat(&mut self)->Result<Self>{
        let mut transfer = self.transfer.take().unwrap();
        let r = transfer.submit_wait()?.await?;

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
