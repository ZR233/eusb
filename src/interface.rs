use std::ffi::{c_int, c_uchar};
use libusb_src::*;
use crate::error::*;
use futures::channel::oneshot::*;
use libusb_src::*;

pub struct  Interface{
    number: c_int,
    dev_handle: *mut libusb_device_handle,
}

unsafe impl Send for Interface{}
unsafe impl Sync for Interface{}

extern "system" fn libusb_transfer_cb_fn_callback(data: *mut libusb_transfer){
    unsafe {
        let ptr = (*data).user_data as *mut Sender<*mut libusb_transfer>;
        let tx = Box::from_raw(ptr);
        let _ = tx.send(data);
    }
}

impl Interface {
    pub(crate) fn new(dev_handle: *mut libusb_device_handle, index: usize)->Result<Self>{
        let number = index as c_int;
        unsafe {
            let r = libusb_claim_interface(dev_handle, number);
            check_err(r)?
        }

        Ok(Self{
            number,
            dev_handle
        })
    }


    pub async fn control_transfer(&self, )->Result<()>{
        let mut transfer = Transfer::new(0)?;
        let (tx, rx) = channel::<*mut libusb_transfer>();

        unsafe {

            let user_data = Box::new(tx);
            let user_data = Box::into_raw(user_data);

            let mut buf:[c_uchar; LIBUSB_CONTROL_SETUP_SIZE] = [0;LIBUSB_CONTROL_SETUP_SIZE];
            let buf_ptr = buf.as_mut_ptr() ;
            libusb_fill_control_setup(buf_ptr, LIBUSB_ENDPOINT_OUT | LIBUSB_REQUEST_TYPE_VENDOR |
                LIBUSB_RECIPIENT_DEVICE, 1, 0, 0,0);

            libusb_fill_control_transfer(
                transfer.handle,
                self.dev_handle,
                buf_ptr,
                libusb_transfer_cb_fn_callback,
                user_data as _,
                0
            );

            let r=  libusb_submit_transfer(transfer.handle);
            check_err(r)?;

            let r = rx.await.map_err(|e|{
                Error::Other
            })?;

            check_err((*r).status)
        }


    }

}

impl Drop for Interface {
    fn drop(&mut self) {
        unsafe {
            libusb_release_interface(self.dev_handle, self.number);
        }
    }
}


struct Transfer{
    handle: *mut libusb_transfer
}

unsafe impl Send for Transfer {}

impl Transfer {
    fn new(iso_packets: usize)->Result< Self>{
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