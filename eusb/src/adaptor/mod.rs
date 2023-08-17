pub(crate) mod libusb;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc};
use std::time::Duration;
use crate::define::*;
use crate::error::*;


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
pub(crate) trait CtxInterface: Send {
    fn claim(&self)->Result<()>;
}

pub(crate) trait CtxDevice<I: CtxInterface, R: IRequest>: Send {
    fn pid(&self)->u16;
    fn vid(&self)->u16;
    fn serial_number(self: &Arc<Self>)-> ResultFuture<String>;
    fn control_request(self: &Arc<Self>,
        param:RequestParamControlTransfer,
        direction: EndpointDirection
    )-> Result<R>;

    fn bulk_request(
        self: &Arc<Self>,
        endpoint: Endpoint,
        package_len: usize,
        timeout: Duration)-> Result<R>;

    fn get_interface(self: &Arc<Self>, index: usize)->Result<I>;
}

pub(crate) trait CtxManager<
    I: CtxInterface,
    R: IRequest,
    D: CtxDevice<I, R>>: Send {
    fn device_list(&self)-> ResultFuture<Vec<D>>;
}