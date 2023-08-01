use std::sync::{Condvar, Mutex};
use std::time::Duration;
use futures::channel::oneshot::*;
use log::debug;
use libusb_src::*;
use crate::error::*;

pub enum UsbControlRecipient {
    Device,
    SpecifiedInterface,
    Endpoint,
    Other,
    DefaultInterface
}
impl UsbControlRecipient {
    pub(crate) fn to_libusb(&self)->u32{
        let t: u8 = match self {
            UsbControlRecipient::Device => LIBUSB_RECIPIENT_DEVICE,
            UsbControlRecipient::Endpoint => LIBUSB_RECIPIENT_ENDPOINT,
            UsbControlRecipient::Other => LIBUSB_RECIPIENT_OTHER,
            UsbControlRecipient::DefaultInterface
            | UsbControlRecipient::SpecifiedInterface => LIBUSB_RECIPIENT_INTERFACE,
        };
        t as _
    }
}

pub enum UsbControlTransferType {
    Standard,
    Class,
    Vendor,
    Reserved
}

impl UsbControlTransferType {
    pub(crate) fn to_libusb(&self)->u32{
        let t: u8 = match self {
            UsbControlTransferType::Standard => LIBUSB_REQUEST_TYPE_STANDARD,
            UsbControlTransferType::Class => LIBUSB_REQUEST_TYPE_CLASS,
            UsbControlTransferType::Vendor => LIBUSB_REQUEST_TYPE_VENDOR,
            UsbControlTransferType::Reserved => LIBUSB_REQUEST_TYPE_RESERVED,
        };
        t as _
    }
}

pub struct  ControlTransferRequest{
    pub recipient: UsbControlRecipient,
    pub transfer_type: UsbControlTransferType,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub timeout: Duration,
}




impl Default for ControlTransferRequest {
    fn default() -> Self {
        Self{
            recipient: UsbControlRecipient::Device,
            transfer_type: UsbControlTransferType::Standard,
            request: 0,
            value: 0,
            index: 0,
            timeout: Duration::from_secs(0),
        }
    }
}

pub(crate) struct EventController{
    pub(crate) ctx: Mutex<EventControllerCtx>,
    pub(crate) cond: Condvar,
}
#[derive(Clone)]
pub(crate) struct EventControllerCtx{
    pub(crate) device_count: usize,
    pub(crate) is_exit: bool,
}

impl EventController {
    pub(crate) fn new()->Self{
        Self{
            ctx: Mutex::new(EventControllerCtx{
                device_count: 0,
                is_exit: false,
            }),
            cond: Condvar::new()
        }
    }

    pub fn open_device(&self){
        let mut ctx = self.ctx.lock().unwrap();
        (*ctx).device_count+=1;
        self.cond.notify_all();
    }

    pub fn close_device(&self){
        let mut ctx = self.ctx.lock().unwrap();
        (*ctx).device_count-=1;
        self.cond.notify_all();
    }

    pub fn exit(&self){
        let mut ctx = self.ctx.lock().unwrap();
        (*ctx).is_exit=true;
        self.cond.notify_all();
    }

}



pub(crate) extern "system" fn libusb_transfer_cb_fn(data: *mut libusb_transfer){
    unsafe {
        let ptr = (*data).user_data as *mut Sender<*mut libusb_transfer>;
        let tx = Box::from_raw(ptr);
        let _ = tx.send(data);
    }
}

pub(crate) fn transfer_channel()-> (Sender<*mut libusb_transfer>, Receiver<*mut libusb_transfer>){
    channel::<*mut libusb_transfer>()
}

pub(crate) struct Transfer{
    pub(crate) handle: *mut libusb_transfer
}

unsafe impl Send for Transfer {}

impl Transfer {
    pub(crate) fn new(iso_packets: usize)->Result< Self>{
        unsafe {
            let r = libusb_alloc_transfer(iso_packets as _);
            if r.is_null(){
                return  Err(Error::Other);
            }
            Ok(Self{
                handle:r
            })
        }
    }
}

impl Drop for Transfer {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.handle);
        }
    }
}