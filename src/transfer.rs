use std::future::Future;
use std::pin::Pin;
use std::ptr::null_mut;
use std::task::{Context, Poll};
use futures::channel::mpsc::*;
use futures::StreamExt;
use libusb_src::*;
use crate::define::{ControlTransferRequest, EndpointDirection};
use crate::error::*;
use crate::device::Device;
use crate::interface::{BulkTransferRequest, Interface};
use pin_project::pin_project;
use crate::libusb as p;












pub struct TransferBase {
    pub(crate) ptr: p::Transfer,
    pub(crate) buff: Vec<u8>,
    complete_send: Sender<Result<TransferBase>>,
    complete_signal: Option<Receiver<Result<TransferBase>>>
}

unsafe impl Send  for TransferBase{}

pub type ResultFuture<T> = Pin<Box<dyn Future<Output=T> + Send>>;

impl TransferBase {
    pub fn new(
        iso_packets: u32,
        buff_size: usize,
    )->Result<Self>{
        let ptr = unsafe {
            p::Transfer::new(iso_packets)
        }?;
        let (mut tx, rx) = channel::<Result<TransferBase>>(1);
        Ok(Self{
            ptr,
            buff: vec![0; buff_size],
            complete_send: tx,
            complete_signal: Some(rx)
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
                s.ptr.0,
                device.get_handle()?,
                buf_ptr,
                Self::complete_cb,
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
                s.ptr.0,
                interface.dev_handle,
                (request.endpoint as u32 | direction.to_libusb()) as u8,
                buf_ptr,
                request.package_len as _,
                Self::complete_cb,
                null_mut(),
                request.timeout.as_millis() as _,
            );
        }
        Ok(s)
    }

    extern "system"  fn complete_cb(data: *mut libusb_transfer){
        unsafe {
            let user_data_ptr = (*data).user_data;

            let mut user_data = Box::from_raw(user_data_ptr as  *mut UserData);

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
            user_data.complete.try_send(result).unwrap();
        }
    }


    pub fn submit(mut self) ->Result<SubmitHandle>{
        let mut inner = self.ptr;
        let mut rx = self.complete_signal.take().unwrap();
        let mut tx = self.complete_send.clone();
        unsafe {
            let user_data = Box::new(UserData {
                transfer: Some(self),
                complete: tx,
            });
            let p = Box::into_raw(user_data);
            inner.set_user_data(p as _);
            inner.submit()?;
        }
        Ok(SubmitHandle{
            future: Box::pin(async move{
                let mut r = rx.next().await.ok_or(Error::NotFound
                )??;
                r.complete_signal = Some(rx);
                Ok(r)
            }),
            inner
        })
    }

    pub fn actual_length(&self)->usize{
        (unsafe {
            (*self.ptr.0).actual_length
        }) as usize
    }
}

#[pin_project]
pub struct SubmitHandle {
    #[pin]
    future: ResultFuture<Result<TransferBase>>,
    inner: p::Transfer
}

impl SubmitHandle {
    pub fn cancel(&self)->Result<()>{
        unsafe {
            self.inner.cancel()
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


struct UserData {
    transfer: Option<TransferBase>,
    complete: Sender<Result<TransferBase>>,
}



impl Drop for TransferBase {
    fn drop(&mut self) {
        unsafe {
            let _ = self.ptr.cancel();
        }
    }
}


