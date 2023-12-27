use std::collections::HashSet;
use std::ffi::CStr;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use log::debug;
use libusb_src::*;
use crate::manager::Manager;
use crate::platform::libusb::device::Device;
use crate::platform::libusb::transfer::Transfer;
use super::errors::*;

pub(crate) struct DeviceHandle {
    pub(crate) ptr: *mut libusb_device_handle,
    claimed: Mutex< HashSet<u8>>,
}

unsafe impl Send for DeviceHandle {}

unsafe impl Sync for DeviceHandle {}

impl From<*mut libusb_device_handle> for DeviceHandle {
    fn from(value: *mut libusb_device_handle) -> Self {
        Manager::get().platform.open_device();
        Self { ptr: value, claimed:Mutex::new( HashSet::new()) }
    }
}

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                Manager::get().platform.close_device();
                let g = self.claimed.lock().unwrap();
                for one in g.iter() {
                    let _ = self.release_interface(*one);
                }
                debug!("device close");
                libusb_close(self.ptr);
            }
        }
    }
}

impl DeviceHandle {
    pub fn claim_interface(&self, interface_number: u8) -> Result {
        unsafe {
            debug!("claim interface [{:3}]", interface_number);
            check_err(libusb_claim_interface(self.ptr, interface_number as _))?;
            let mut g = self.claimed.lock().unwrap();
            g.insert(interface_number);
            Ok(())
        }
    }
    pub fn release_interface(&self, interface_number: u8) -> Result {
        unsafe {
            check_err(libusb_release_interface(self.ptr, interface_number as _))?;
            debug!("release interface [{:3}]", interface_number);
            Ok(())
        }
    }
    pub fn get_configuration(&self) -> Result<u8> {
        unsafe {
            let mut c = 0;
            check_err(libusb_get_configuration(self.ptr, &mut c))?;
            Ok(c as _)
        }
    }

    pub fn set_configuration(&self, config_value: u8) -> Result {
        unsafe {
            check_err(libusb_set_configuration(self.ptr, config_value as _))?;
            Ok(())
        }
    }
    pub fn clear_halt(&self, endpoint: u8) -> Result {
        unsafe {
            check_err(libusb_clear_halt(self.ptr, endpoint as _))?;
            Ok(())
        }
    }

    pub fn get_string_descriptor_ascii(&self, index: u8) -> Result<String> {
        unsafe {
            let mut buff = [0u8; 1024];
            let _ = check_err(libusb_get_string_descriptor_ascii(self.ptr, index, buff.as_mut_ptr(), 1024))?;
            let c = CStr::from_ptr(buff.as_ptr() as _);
            let out = c.to_string_lossy().to_string();
            Ok(out)
        }
    }

    pub fn get_device(&self) -> Device {
        unsafe {
            let mut dev = libusb_get_device(self.ptr);
            dev = libusb_ref_device(dev);
            dev
        }.into()
    }

    pub async fn control_transfer(&self, data: &mut [u8], request_type: u8, request: u8, value: u16, index: u16, timeout: Duration)->Result<Transfer> {
        unsafe {
            let mut transfer = Transfer::control_transfer(
                sync_cb,
                data, request_type, request, value, index, timeout);

            transfer.set_handle(self.ptr);
            let future =  SyncTransfer::new();
            let b = Arc::into_raw(future.inner.clone());
            transfer.set_user_data(b as _);
            transfer.submit()?;
            future.await;
            Arc::from_raw(b);
            transfer.result()?;
            Ok(transfer)
        }
    }
}

extern "system"  fn sync_cb(transfer: *mut libusb_transfer) {
    unsafe {
        let sync = (*transfer).user_data as *const SyncTransferInner;
        (*sync).is_ok.store(true, Ordering::SeqCst);
        let wake = {
            let g = (*sync).waker.lock().unwrap();
            g.as_ref().cloned()
        };
        if let Some(w)= wake{
            w.wake();
        }
    }
}

struct SyncTransfer {
    inner: Arc<SyncTransferInner>
}
struct SyncTransferInner{
    is_ok: AtomicBool,
    waker: Mutex<Option<Waker>>
}

impl SyncTransfer {
    fn new()->Self{
        Self{
            inner: Arc::new(SyncTransferInner {
                is_ok: AtomicBool::new(false),
                waker: Mutex::new(None)
            })
        }
    }

}

impl Future for SyncTransfer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {

        if self.inner.is_ok.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            {
                let mut g = self.inner.waker.lock().unwrap();
                *g = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}





