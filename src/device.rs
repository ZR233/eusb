use std::ffi::{c_char, c_int, c_uchar};
use std::fmt::{Display, Formatter, Pointer};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use log::debug;
use libusb_src::*;
use crate::define::*;
use crate::error::*;
use crate::interface::Interface;


pub struct Device {
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<*mut libusb_device_handle>,
    event_controller: Arc<EventController>
}

#[derive(Debug)]
pub enum UsbSpeed {
    Unknown,
    Low,
    Full,
    High,
    Super,
    SuperPlus,
}

unsafe impl Send for Device {}

unsafe impl Sync for Device {}

pub type Descriptor = libusb_device_descriptor;

impl Device {
    pub(crate) fn new(
        dev: *mut libusb_device,
        event_controller: Arc<EventController>
    ) -> Self {
        unsafe {
            libusb_ref_device(dev);
        }

        Self {
            dev,
            handle: Mutex::new(null_mut()),
            event_controller
        }
    }
    pub fn descriptor(&self) -> Descriptor {
        let mut desc = Descriptor::default();
        unsafe {
            let desc_ptr = (&mut desc) as *mut libusb_device_descriptor;
            let r = libusb_get_device_descriptor(self.dev, desc_ptr);
            if r < 0 {
                return desc;
            }
        }
        desc
    }

    pub fn speed(&self) -> UsbSpeed {
        let r = unsafe {
            libusb_get_device_speed(self.dev)
        };
        if r < 0 {
            return UsbSpeed::Unknown;
        }

        match r {
            LIBUSB_SPEED_LOW => UsbSpeed::Low,
            LIBUSB_SPEED_FULL => UsbSpeed::Full,
            LIBUSB_SPEED_HIGH => UsbSpeed::High,
            LIBUSB_SPEED_SUPER => UsbSpeed::Super,
            LIBUSB_SPEED_SUPER_PLUS => UsbSpeed::SuperPlus,
            LIBUSB_SPEED_UNKNOWN | _ => UsbSpeed::Unknown
        }
    }

    pub(crate) fn get_handle(&self) -> Result<*mut libusb_device_handle> {
        let mut g = self.handle.lock().unwrap();

        unsafe {
            if g.is_null() {
                let r = libusb_open(self.dev, &mut *g);
                check_err(r)?;
                libusb_set_auto_detach_kernel_driver(*g, 1);
            }
        }
        self.event_controller.open_device();
        Ok(*g)
    }

    pub fn get_configuration(&self) -> Result<i32> {
        unsafe {
            let mut config: c_int = 0;
            let ptr = (&mut config) as *mut c_int;

            let r = libusb_get_configuration(self.get_handle()?, ptr);
            check_err(r)?;
            Ok(config as _)
        }
    }
    pub fn set_configuration(&self, config: i32) -> Result<()> {
        unsafe {
            let config: c_int = config as _;
            let r = libusb_set_configuration(self.get_handle()?, config);
            check_err(r)?;
        }
        Ok(())
    }

    pub fn get_interface(&self, index: usize) -> Result<Interface> {
        let dev_handle = self.get_handle()?;
        Interface::new(dev_handle, index)
    }
    pub async fn control_transfer_in(&self, request: ControlTransferRequest, max_len: u32) -> Result<Vec<u8>>{
        let buf_len = LIBUSB_CONTROL_SETUP_SIZE + max_len as usize;
        let mut buf = vec![0 as c_uchar; buf_len];
        let actual_length = self.control_transfer(
            request,
            LIBUSB_ENDPOINT_IN,
            buf.as_mut_slice(),
        ).await?;
        let mut data = Vec::with_capacity(actual_length);
        for i in LIBUSB_CONTROL_SETUP_SIZE..LIBUSB_CONTROL_SETUP_SIZE+actual_length {
            data.push(buf[i] as _);
        }
        Ok(data)
    }
    pub async fn control_transfer_out(&self, request: ControlTransferRequest, data: &[u8])-> Result<()> {
        let mut buf = Vec::with_capacity(LIBUSB_CONTROL_SETUP_SIZE+ data.len());
        for _ in 0..LIBUSB_CONTROL_SETUP_SIZE {
            buf.push(0);
        }
        for i in 0..data.len(){
            buf.push(data[i] as _);
        }

        let actual_length = self.control_transfer(
            request,
            LIBUSB_ENDPOINT_OUT,
            buf.as_mut_slice(),
        ).await?;

        if actual_length != data.len() {
            return  Err(Error::Io(format!("send {}, actual {}", data.len(), actual_length)))
        }
        Ok(())
    }

    async fn control_transfer(
        &self,
        request: ControlTransferRequest,
        request_type: u8,
        buf: &mut [c_uchar],
    ) -> Result<usize> {
        let mut transfer = Transfer::new(0)?;
        let (tx, rx) = transfer_channel();

        unsafe {
            let user_data = Box::new(tx);
            let user_data = Box::into_raw(user_data);
            let buf_ptr = buf.as_mut_ptr();

            libusb_fill_control_setup(
                buf_ptr,
                (request_type as u32 | request.transfer_type.to_libusb() | request.recipient.to_libusb()) as u8,
                request.request,
                request.value,
                request.index,
                (buf.len() - LIBUSB_CONTROL_SETUP_SIZE) as _ );


            libusb_fill_control_transfer(
                transfer.handle,
                self.get_handle()?,
                buf_ptr,
                libusb_transfer_cb_fn,
                user_data as _,
                request.timeout.as_millis() as _,
            );

            let r = libusb_submit_transfer(transfer.handle);
            check_err(r)?;

            let r = rx.await.map_err(|e| {
                Error::Other
            })?;
            match (*r).status {
                LIBUSB_TRANSFER_COMPLETED => Ok((*r).actual_length as _),
                LIBUSB_TRANSFER_OVERFLOW => Err(Error::Overflow),
                LIBUSB_TRANSFER_TIMED_OUT => Err(Error::Timeout),
                LIBUSB_TRANSFER_CANCELLED => Err(Error::Cancelled),
                LIBUSB_TRANSFER_STALL => Err(Error::NotSupported),
                LIBUSB_TRANSFER_NO_DEVICE => Err(Error::NoDevice),
                LIBUSB_TRANSFER_ERROR |_ => Err(Error::Other),
            }
        }
    }
}


impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let des = self.descriptor();
        write!(f, "pid: {} vid: {}", des.idProduct, des.idVendor)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.dev);
            let mut handle = self.handle.lock().unwrap();
            if !handle.is_null() {
                libusb_close(*handle);
                self.event_controller.close_device();
            }
            debug!("Device closed");
        }
    }
}




