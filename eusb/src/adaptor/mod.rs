pub(crate) mod libusb;
use std::future::Future;
#[cfg(unix)]
use std::os::fd::RawFd;
use std::pin::Pin;
use std::sync::{Arc};
use std::time::Duration;
use crate::define::*;
pub(crate) use crate::error::*;


pub(crate) struct  RequestParamControlTransfer{
    pub(crate) recipient: UsbControlRecipient,
    pub(crate) transfer_type: UsbControlTransferType,
    pub(crate) request: u8,
    pub(crate) value: u16,
    pub(crate) index: u16,
    pub(crate) timeout: Duration,
}



#[derive(Copy, Clone)]
pub enum EndpointDirection<'a>{
    In{capacity: usize},
    Out{ src: &'a [u8]}
}

pub trait IRequest {
    fn data(&mut self)->&mut[u8];
}

pub(crate) type ResultFuture<T> = Pin<Box<dyn Future<Output=Result<T>> + Send>>;

pub(crate) trait IManager{}
pub trait IInterface<R: IRequest>: Send {
    fn bulk_request(
        &self,
        endpoint: EndpointDescriptor,
        package_len: usize,
        timeout: Duration)-> Result<R>;
}


pub(crate) trait CtxDevice<R: IRequest, I: IInterface<R>>: Send {
    fn pid(&self)->u16;
    fn vid(&self)->u16;
    fn serial_number(self: &Arc<Self>)-> ResultFuture<String>;
    fn control_request(self: &Arc<Self>,
        param:RequestParamControlTransfer,
        direction: EndpointDirection
    )-> Result<R>;

    // fn bulk_request(
    //     self: &Arc<Self>,
    //     endpoint: Endpoint,
    //     package_len: usize,
    //     timeout: Duration)-> Result<R>;

    fn claim_interface(self: &Arc<Self>, num: usize) ->Result<I>;
    fn get_config(self: &Arc<Self>) ->Result<ConfigDescriptor>;
    fn set_config(self: &Arc<Self>, config_value: u8)->Result<()>;
    fn config_list(self: &Arc<Self>) ->Result<Vec<ConfigDescriptor>>;
}

pub(crate) trait CtxManager<
    R: IRequest,
    I: IInterface<R>,
    D: CtxDevice<R, I>>: Send {
    fn device_list(&self)-> ResultFuture<Vec<D>>;

    #[cfg(unix)]
    fn open_device_with_fd(&self, fd: RawFd)->Result<D>;
}
