pub(crate) mod libusb;
use std::future::Future;
#[cfg(unix)]
use std::os::fd::RawFd;
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
pub trait IInterface: Send {
    fn claim(&self)->Result<()>;
}

pub trait IConfig<I: IInterface>{
    fn with_value(value: usize)->Self;
    /// Identifier value for this configuration.
    fn configuration_value(&self)->u8;
    /// Extra descriptors.
    fn extra(&self)-> Vec<u8>;
    /// Maximum power consumption of the USB device from this bus in this configuration when the device is fully operation.
    /// Expressed in units of 2 mA when the device is operating in high-speed mode and in units of 8 mA when the device is operating in super-speed mode.
    fn max_power(&self)-> u8;
    fn interfaces(&self)->Vec<I>;
}

pub(crate) trait CtxDevice<I: IInterface, R: IRequest, C: IConfig<I>>: Send {
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

    fn configs(self: &Arc<Self>)->Vec<C>;
}

pub(crate) trait CtxManager<
    I: IInterface,
    R: IRequest,
    C: IConfig<I>,
    D: CtxDevice<I, R, C>>: Send {
    fn device_list(&self)-> ResultFuture<Vec<D>>;

    #[cfg(unix)]
    fn open_device_with_fd(&self, fd: RawFd)->Result<D>;
}
