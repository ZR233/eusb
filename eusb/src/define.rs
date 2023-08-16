use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc};
use crate::error::*;

pub type ResultFuture<T> = Pin<Box<dyn Future<Output=Result<T>> + Send>>;

pub(crate) trait IManager{}


pub(crate) trait CtxInterface: Send {
}


pub(crate) trait CtxDevice<I: CtxInterface>: Send {
    fn interface_list(&self)-> ResultFuture<Vec<Arc<I>>>;
    fn serial_number(&self)-> ResultFuture<String>;
}

pub(crate) trait CtxManager<I: CtxInterface, D: CtxDevice<I>,>: Send {
    fn device_list(&self)-> ResultFuture<Vec<D>>;
}