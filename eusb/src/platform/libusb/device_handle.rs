use std::collections::HashSet;
use std::ffi::CStr;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, Waker};
use std::time::Duration;
use log::{debug, trace};
use libusb_src::*;
use crate::manager::Manager;
use crate::platform::libusb::device::Device;
use crate::platform::libusb::transfer::Transfer;
use super::errors::*;
pub(crate) use super::transfer::TransferDirection;

pub(crate) struct DeviceHandle {
    pub(crate) ptr: *mut libusb_device_handle,
    claimed: RwLock<HashSet<u8>>,
}

unsafe impl Send for DeviceHandle {}

unsafe impl Sync for DeviceHandle {}

impl From<*mut libusb_device_handle> for DeviceHandle {
    fn from(value: *mut libusb_device_handle) -> Self {
        Manager::get().platform.open_device();
        Self { ptr: value, claimed: RwLock::new(HashSet::new()) }
    }
}

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                Manager::get().platform.close_device();
                let g = self.claimed.read().unwrap();
                for one in g.iter() {
                    let _ = self.release_interface(*one);
                }
                debug!("device close");
                libusb_close(self.ptr);
            }
        }
    }
}

#[allow(unused)]
impl DeviceHandle {
    pub fn claim_interface(&self, interface_number: u8) -> Result {
        unsafe {
            {
                let g = self.claimed.read().unwrap();
                if g.contains(&interface_number){
                    return Ok(())
                }
            }

            trace!("claim interface [{:3}] begin", interface_number);
            check_err(libusb_claim_interface(self.ptr, interface_number as _))?;
            let mut g = self.claimed.write().unwrap();
            g.insert(interface_number);
            debug!("claim interface [{:3}]", interface_number);
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

    pub fn set_auto_detach_kernel_driver(&self, enable: bool)->Result{
        unsafe {
            check_err( libusb_set_auto_detach_kernel_driver(self.ptr,if enable { 1 } else { 0 }))?;
        }
        Ok(())
    }
    pub fn set_auto_detach_kernel_driver_with_guard(&self, enable: bool)->Result<AutoDetachKernelDriverGuard>{
        unsafe {
            check_err( libusb_set_auto_detach_kernel_driver(self.ptr,if enable { 1 } else { 0 }))?;
        }
        Ok(AutoDetachKernelDriverGuard{dev: self.ptr})
    }

    pub fn kernel_driver_active(&self, interface_number: u8)->Result<bool>{
        let r = unsafe {
            check_err(libusb_kernel_driver_active(self.ptr,interface_number as _))?
        };
        Ok(r == 1)
    }

    pub fn detach_kernel_driver(&self, interface_number: u8)->Result{
        unsafe {
            check_err(libusb_detach_kernel_driver(self.ptr,interface_number as _))?;
        }
        Ok(())
    }

    pub fn get_device(&self) -> Device {
        unsafe {
            let mut dev = libusb_get_device(self.ptr);
            dev = libusb_ref_device(dev);
            dev
        }.into()
    }

    async fn do_sync_transfer(&self, mut transfer: Transfer) -> Result<Transfer> {
        unsafe {
            transfer.set_handle(self.ptr);
            let future = SyncTransfer::new();
            let b = Arc::into_raw(future.inner.clone());
            transfer.set_user_data(b as _);
            let wp = SyncTransferInnerWrapper(b);
            transfer.submit()?;
            future.await;
            Arc::from_raw(wp.0);
            transfer.result()?;
            Ok(transfer)
        }
    }

    pub async fn control_transfer(&self, direction: TransferDirection, request_type: u8, request: u8, value: u16, index: u16, timeout: Duration) -> Result<Transfer> {
        unsafe {
            let transfer = Transfer::control_transfer(
                sync_cb, direction, request_type, request, value, index, timeout);

            self.do_sync_transfer(transfer).await
        }
    }
    pub async fn bulk_transfer(&self, direction: TransferDirection, endpoint: u8, timeout: Duration, is_interrupt: bool) -> Result<Transfer> {
        unsafe {
            let transfer = Transfer::bulk_transfer(endpoint, sync_cb, direction, timeout);
            (*transfer.ptr).transfer_type = if is_interrupt {
                LIBUSB_TRANSFER_TYPE_INTERRUPT
            } else {
                LIBUSB_TRANSFER_TYPE_BULK
            };
            self.do_sync_transfer(transfer).await
        }
    }
}

extern "system" fn sync_cb(transfer: *mut libusb_transfer) {
    unsafe {
        let sync = (*transfer).user_data as *const SyncTransferInner;
        (*sync).is_ok.store(true, Ordering::SeqCst);
        let wake = {
            let g = (*sync).waker.lock().unwrap();
            g.as_ref().cloned()
        };
        if let Some(w) = wake {
            w.wake();
        }
    }
}

pub(crate) struct AutoDetachKernelDriverGuard{
    dev: *mut libusb_device_handle
}

impl Drop for AutoDetachKernelDriverGuard {
    fn drop(&mut self) {
        unsafe {
            libusb_set_auto_detach_kernel_driver(self.dev, 1);
        }
    }
}


struct SyncTransfer {
    inner: Arc<SyncTransferInner>,
}


struct SyncTransferInner {
    is_ok: AtomicBool,
    waker: Mutex<Option<Waker>>,
}
struct SyncTransferInnerWrapper(*const SyncTransferInner);
unsafe impl Send for SyncTransferInnerWrapper{}


impl SyncTransfer {
    fn new() -> Self {
        Self {
            inner: Arc::new(SyncTransferInner {
                is_ok: AtomicBool::new(false),
                waker: Mutex::new(None),
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





