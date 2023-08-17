use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::Arc;
use std::time::Duration;
use log::trace;
use libusb_src::*;
use crate::adaptor::{EndpointDirection, IRequest, RequestParamControlTransfer};
use crate::define::*;
use crate::error::*;
use super::device::CtxDeviceImpl;

pub struct Request {
    pub(crate) ptr: Transfer,
    pub(crate) buff: Vec<u8>,
}

#[derive(Copy, Clone)]
pub(crate) struct Transfer(pub(crate) *mut libusb_transfer);

unsafe impl Sync for Transfer {}

unsafe impl Send for Transfer {}


pub trait ToLib: Sized {
    fn to_lib(self) -> u32;
}


impl ToLib for EndpointDirection<'_> {
    fn to_lib(self) -> u32 {
        (match self {
            EndpointDirection::In { .. } => LIBUSB_ENDPOINT_IN,
            EndpointDirection::Out { .. } => LIBUSB_ENDPOINT_OUT,
        }) as u32
    }
}

impl ToLib for UsbControlTransferType {
    fn to_lib(self) -> u32 {
        let t: u8 = match self {
            UsbControlTransferType::Standard => LIBUSB_REQUEST_TYPE_STANDARD,
            UsbControlTransferType::Class => LIBUSB_REQUEST_TYPE_CLASS,
            UsbControlTransferType::Vendor => LIBUSB_REQUEST_TYPE_VENDOR,
            UsbControlTransferType::Reserved => LIBUSB_REQUEST_TYPE_RESERVED,
        };
        t as _
    }
}

impl ToLib for UsbControlRecipient {
    fn to_lib(self) -> u32 {
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
impl ToLib for Endpoint {
    fn to_lib(self) -> u32 {
        match self {
            Endpoint::In {num} => (LIBUSB_ENDPOINT_IN as u32 ) | (num as u32),
            Endpoint::Out {num} => (LIBUSB_ENDPOINT_OUT as u32) | (num as u32),
        }
    }
}

impl Request {
    pub(crate) fn new(
        iso_packets: u32,
        buff_size: usize,
    ) -> Result<Self> {
        let ptr = unsafe {
            let t = libusb_alloc_transfer(iso_packets as _);
            if t.is_null() {
                return Err(Error::Other("Alloc transfer fail".to_string()));
            }
            Ok(t)
        }?;

        Ok(Self {
            ptr: Transfer(ptr),
            buff: vec![0; buff_size],
        })
    }

    pub(crate) fn control(
        device: &Arc<CtxDeviceImpl>,
        request: RequestParamControlTransfer,
        direction: EndpointDirection,
    ) -> Result<Self> {
        let data_len = match direction {
            EndpointDirection::In { capacity } => capacity,
            EndpointDirection::Out { src } => { src.len() as _ }
        };

        let mut s = Self::new(0, LIBUSB_CONTROL_SETUP_SIZE + (data_len))?;

        if let EndpointDirection::Out { src } = direction {
            for i in 0..src.len() {
                s.buff[i + LIBUSB_CONTROL_SETUP_SIZE] = src[i];
            }
        }

        unsafe {
            let buf_ptr = s.buff.as_mut_ptr();
            let rt: u32 = direction.to_lib() | request.transfer_type.to_lib() | request.recipient.to_lib();

            libusb_fill_control_setup(
                buf_ptr,
                rt as u8,
                request.request,
                request.value,
                request.index,
                data_len as u16);
            let handle = device.get_handle()?;


            libusb_fill_control_transfer(
                s.ptr.0,
                handle.0,
                buf_ptr,
                Self::empty_cb,
                null_mut(),
                request.timeout.as_millis() as _,
            );
        }

        Ok(s)
    }


    pub(crate) fn bulk(
        device: &Arc<CtxDeviceImpl>,
        endpoint: Endpoint,
        package_len: usize,
        timeout: Duration
    ) -> Result<Self> {

        let mut s = Self::new(0, package_len)?;
        let handle = device.get_handle()?.0;
        unsafe {
            let buf_ptr = s.buff.as_mut_ptr();

            libusb_fill_bulk_transfer(
                s.ptr.0,
                handle,
                endpoint.to_lib() as _,
                buf_ptr,
                package_len as _,
                Self::empty_cb,
                null_mut(),
                timeout.as_millis() as _,
            );
        }
        Ok(s)
    }


    extern "system" fn empty_cb(_: *mut libusb_transfer) {}
}

impl IRequest for Request {
    fn data(&mut self) -> &mut [u8] {
        unsafe {
            let len = (*self.ptr.0).actual_length as usize;

            if (*self.ptr.0).transfer_type == LIBUSB_TRANSFER_TYPE_CONTROL {
                let s = self.buff.as_mut_slice();
                return &mut s[LIBUSB_CONTROL_SETUP_SIZE..LIBUSB_CONTROL_SETUP_SIZE + len];
            }

            return &mut self.buff.as_mut_slice()[0..len];
        }
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        unsafe {
            libusb_free_transfer(self.ptr.0);
            trace!("Transfer release");
        }
    }
}