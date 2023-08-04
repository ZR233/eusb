use std::future::Future;
use std::pin::Pin;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::task::{Context, Poll};
use futures::channel::mpsc::*;
use futures::{StreamExt};
use libusb_src::*;
use crate::define::{ControlTransferRequest, EndpointDirection};
use crate::error::*;
use crate::device::Device;
use crate::interface::{BulkTransferRequest, Interface};
use pin_project::pin_project;
use crate::libusb as p;

pub(crate) trait TransferWarp{
    fn get_base(self)->Transfer;
    fn from_base(transfer: Transfer)->Self;
}
/// The generic USB transfer.
/// The user populates this structure and then submits it in order to request a in transfer.
/// After the transfer has completed, the library populates the transfer with the results and passes it back to the user.
pub struct TransferIn{
    pub(crate) base:Option<Transfer>
}

impl TransferIn{
    pub fn data(&self) ->&[u8]{
        let b = self.base.as_ref().unwrap();
        unsafe {
            match (*b.ptr.0).transfer_type {
                LIBUSB_TRANSFER_TYPE_CONTROL=>{
                    let data_ptr = libusb_control_transfer_get_data(b.ptr.0);
                    let s  = &*slice_from_raw_parts(data_ptr as *const u8, b.actual_length());
                    s
                }
                _=> &b.buff[0..b.actual_length()]
            }
        }
    }
    /// Submit a transfer.
    /// This function will fire off the USB transfer and then return a [SubmitHandle] immediately.
    pub fn submit(self) ->Result<SubmitHandle<TransferIn>>{
        Transfer::submit(self)
    }
}
/// The generic USB transfer.
/// The user populates this structure and then submits it in order to request a  out transfer.
/// After the transfer has completed, the library populates the transfer with the results and passes it back to the user.
pub struct TransferOut{
    pub(crate) base:Option<Transfer>
}
impl TransferOut{
    pub fn set_data(&mut self, src: &[u8]){
        self.base.as_mut().unwrap().set_buff(src)
    }

    pub fn actual_length(&self)->usize{
        self.base.as_ref().unwrap().actual_length()
    }

    /// Submit a transfer.
    /// This function will fire off the USB transfer and then return a [SubmitHandle] immediately.
    pub fn submit(self) ->Result<SubmitHandle<TransferOut>>{
        Transfer::submit(self)
    }
}

impl TransferWarp for TransferOut {
    fn get_base(mut self) -> Transfer {
        self.base.take().unwrap()
    }

    fn from_base(transfer: Transfer) -> Self {
        Self{
            base: Some(transfer)
        }
    }
}

impl TransferWarp for TransferIn {
    fn get_base(mut self) -> Transfer {
        self.base.take().unwrap()
    }

    fn from_base(transfer: Transfer) -> Self {
        Self{
            base: Some(transfer)
        }
    }
}



pub struct Transfer {
    pub(crate) ptr: p::Transfer,
    pub buff: Vec<u8>,
    complete_send: Sender<Result<Transfer>>,
    complete_recv: Option<Receiver<Result<Transfer>>>,
}

unsafe impl Send  for Transfer {}

pub type ResultFuture<T> = Pin<Box<dyn Future<Output=T> + Send>>;

impl TransferWarp for Transfer {
    fn get_base(self) -> Transfer {
        self
    }

    fn from_base(transfer: Transfer) -> Self {
        transfer
    }
}


impl Transfer {
    pub(crate) fn new(
        iso_packets: u32,
        buff_size: usize,
    )->Result<Self>{
        let ptr = unsafe {
            p::Transfer::new(iso_packets)
        }?;
        let (tx, rx) = channel::<Result<Transfer>>(1);
        Ok(Self{
            ptr,
            buff: vec![0; buff_size],
            complete_send: tx,
            complete_recv: Some(rx),
        })
    }

    pub(crate) fn control(
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
    pub(crate) fn bulk (
        interface: &Interface,
        request: BulkTransferRequest,
        direction: EndpointDirection,
        data_out: &[u8],
    )->Result<Self>{

        let mut s= if direction== EndpointDirection::Out {
            let mut s= Self::new(0, 0)?;
            s.buff = Vec::from(data_out);
            s
        }else{
            Self::new(0, request.package_len)?
        };

        let length = s.buff.len();

        unsafe {
            let buf_ptr = s.buff.as_mut_ptr();

            libusb_fill_bulk_transfer(
                s.ptr.0,
                interface.dev_handle,
                (request.endpoint as u32 | direction.to_libusb()) as u8,
                buf_ptr,
                length as _,
                Self::complete_cb,
                null_mut(),
                request.timeout.as_millis() as _,
            );
        }
        Ok(s)
    }

    fn set_buff(&mut self, src: &[u8]){
        unsafe {
            match (*self.ptr.0).transfer_type {
                LIBUSB_TRANSFER_TYPE_BULK|LIBUSB_TRANSFER_TYPE_INTERRUPT=>{
                    self.buff.copy_from_slice(src);
                }
                LIBUSB_TRANSFER_TYPE_CONTROL=>{
                    let buff = &mut self.buff[LIBUSB_CONTROL_SETUP_SIZE..];
                    buff.copy_from_slice(src);
                }
                _ =>{}
            }
        }
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
    fn submit<T: TransferWarp>(sw: T) ->Result<SubmitHandle<T>>{
        let mut s = sw.get_base();
        let mut inner = s.ptr;
        let mut rx = s.complete_recv.take().unwrap();
        let tx = s.complete_send.clone();
        unsafe {
            let user_data = Box::new(UserData {
                transfer: Some(s),
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
                r.complete_recv=Some(rx);
                Ok(T::from_base(r))
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
pub struct SubmitHandle<T>  {
    #[pin]
    future: ResultFuture<Result<T>>,
    inner: p::Transfer,
}

impl <T>SubmitHandle<T> {
    pub fn cancel_token(&self)->TransferCancelToken{
        TransferCancelToken(self.inner)
    }
}

impl <T>Future for SubmitHandle<T>{
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.future.poll(cx) {
            Poll::Ready(t) => { Poll::Ready(t) }
            Poll::Pending => { Poll::Pending }
        }
    }
}


struct UserData{
    transfer: Option<Transfer>,
    complete: Sender<Result<Transfer>>,
}



impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.ptr.cancel();
        }
    }
}

#[derive(Copy, Clone)]
pub struct TransferCancelToken(p::Transfer);
unsafe impl Send  for TransferCancelToken{}
unsafe impl Sync  for TransferCancelToken{}


impl TransferCancelToken {
    ///Asynchronously cancel a previously submitted transfer.
    ///
    /// This function returns immediately, but this does not indicate cancellation is complete.
    /// Your async function will be finished at some later time with a transfer [Error] of [Error::Cancelled].
    ///
    /// This function behaves differently on Darwin-based systems (macOS and iOS):
    /// - Calling this function for one transfer will cause all transfers on the same endpoint to be cancelled.
    /// Your callback function will be invoked with a transfer [Error] of [Error::Cancelled] for each transfer that was cancelled.
    /// - Calling this function also sends a ClearFeature(ENDPOINT_HALT) request for the transfer's endpoint.
    /// If the device does not handle this request correctly,
    /// the data toggle bits for the endpoint can be left out of sync between host and device,
    /// which can have unpredictable results when the next data is sent on the endpoint,
    /// including data being silently lost. A call to libusb_clear_halt will not resolve this situation,
    /// since that function uses the same request.
    /// Therefore, if your program runs on Darwin and uses a device that does not
    /// correctly implement ClearFeature(ENDPOINT_HALT) requests, it may only be safe to cancel
    /// transfers when followed by a device reset using libusb_reset_device.
    pub fn cancel(&self) ->Result<()>{
        unsafe{
            self.0.cancel()
        }
    }
}