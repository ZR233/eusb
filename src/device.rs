use std::ffi::{c_int, c_uchar};
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::{Arc, Mutex};
use log::debug;
use libusb_src::*;
use crate::define::*;
use crate::error::*;
use crate::interface::Interface;
use crate::transfer;
use futures::StreamExt;
use crate::transfer::{ResultFuture, Transfer};

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
    pub fn descriptor(&self) -> DeviceDescriptor {
        let mut desc = DeviceDescriptor::default();
        unsafe {
            let r = libusb_get_device_descriptor(self.dev, &mut desc.data);
            if r < 0 {
                return desc;
            }
        }
        desc
    }

    pub fn config_list(&self)->Result<Vec<Config>>{
        let mut out = vec![];
        let desc = self.descriptor();
        for index in 0..desc.data.bNumConfigurations {
            let mut config = Config{
                data: null_mut(),
            };
            unsafe {
                let r = libusb_get_config_descriptor(
                    self.dev,
                    index,
                    &mut config.data,
                );
                if r >=0{
                    out.push(config);
                }
            }
        }
        Ok(out)
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
    pub fn serial_number(&self)-> String{
        let desc = self.descriptor();
        let index = desc.data.iSerialNumber;
        let mut buff = vec![0u8; 256];
        let buff_len = buff.len();
        if index > 0 {
            unsafe {
                match self.get_handle(){
                    Ok(dev) => {
                        let r = libusb_get_string_descriptor_ascii(
                            dev,
                            index,
                            buff.as_mut_ptr(),
                            buff_len as _
                        );
                        if r > 0{
                            buff.resize(r as _, 0);
                            match String::from_utf8(buff){
                                Ok(s) => {return s;}
                                Err(_) => {}
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        String::new()
    }
    pub(crate) fn get_handle(&self) -> Result<*mut libusb_device_handle> {
        let mut g = self.handle.lock().unwrap();

        unsafe {
            if g.is_null() {
                let r = libusb_open(self.dev, &mut *g);
                check_err(r)?;
                self.event_controller.open_device();

                libusb_set_auto_detach_kernel_driver(*g, 1);
            }
        }
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
        let g = self.handle.lock().unwrap();
        unsafe {
            let mut config_old: c_int = 0;
            let ptr = (&mut config_old) as *mut c_int;
            let mut r = libusb_get_configuration(*g, ptr);
            check_err(r)?;

            if config!= config_old {

                libusb_set_auto_detach_kernel_driver(*g, 0);
                let config: c_int = config as _;
                r = libusb_set_configuration(*g, config);
                if r<0{
                    libusb_set_auto_detach_kernel_driver(*g, 1);
                }
                check_err(r)?;

                libusb_set_auto_detach_kernel_driver(*g, 1);
            }
        }
        Ok(())
    }

    pub fn get_interface(&self, index: usize) -> Result<Interface> {
        let dev_handle = self.get_handle()?;
        Interface::new(dev_handle, index)
    }
    pub async fn control_transfer_in(&self, request: ControlTransferRequest, max_len: u16) -> Result<Vec<u8>>{
        let transfer = Transfer::control(
            &self,
            request,
            EndpointDirection::In,
            max_len,
            &[]
        )?;
        let tran_new = Transfer::submit_wait(transfer)?.await?;
        let mut data = Vec::with_capacity(tran_new.actual_length());
        for i in LIBUSB_CONTROL_SETUP_SIZE..LIBUSB_CONTROL_SETUP_SIZE+tran_new.actual_length() {
            data.push(tran_new.buff[i]);
        }
        Ok(data)

        // let buf_len = LIBUSB_CONTROL_SETUP_SIZE + max_len as usize;
        // let mut buf = vec![0 as c_uchar; buf_len];
        // let actual_length = self.control_transfer(
        //     request,
        //     EndpointDirection::In,
        //     buf.as_mut_slice(),
        // ).await?;
        // let mut data = Vec::with_capacity(actual_length);
        // for i in LIBUSB_CONTROL_SETUP_SIZE..LIBUSB_CONTROL_SETUP_SIZE+actual_length {
        //     data.push(buf[i] as _);
        // }
        // Ok(data)
    }
    pub async fn control_transfer_out(&self, request: ControlTransferRequest, data: &[u8])-> Result<()> {
        let transfer = Transfer::control(
            &self,
            request,
            EndpointDirection::Out,
            0,
            data
        )?;

        let tran_new = Transfer::submit_wait(transfer)?.await?;
        if tran_new.actual_length() != data.len() {
            return  Err(Error::Io(format!("send {}, actual {}", data.len(), tran_new.actual_length())))
        }
        Ok(())

        // let mut buf = Vec::with_capacity(LIBUSB_CONTROL_SETUP_SIZE+ data.len());
        // for _ in 0..LIBUSB_CONTROL_SETUP_SIZE {
        //     buf.push(0);
        // }
        // for i in 0..data.len(){
        //     buf.push(data[i] as _);
        // }
        //
        // let actual_length = self.control_transfer(
        //     request,
        //     EndpointDirection::Out,
        //     buf.as_mut_slice(),
        // ).await?;
        //
        // if actual_length != data.len() {
        //     return  Err(Error::Io(format!("send {}, actual {}", data.len(), actual_length)))
        // }
        // Ok(())
    }
    // async fn control_transfer(
    //     &self,
    //     request: ControlTransferRequest,
    //     direction: EndpointDirection,
    //     data_len: usize,
    // ) -> Result<usize> {
    //
    //     let mut transfer = transfer::Transfer::control(
    //         &self,
    //         request,
    //         direction,
    //         data_len
    //     )?;
    //     let mut rx = transfer.set_complete_cb();
    //
    //     transfer::Transfer::submit(transfer)?;
    //
    //     let r = rx.next().await.ok_or(Error::NotFound
    //     )??;
    //
    //     Ok(r.actual_length())
    // }
    // fn control_transfer(
    //     &self,
    //     request: ControlTransferRequest,
    //     direction: EndpointDirection,
    //     buf: &mut [u8],
    // ) -> Pin<Box<dyn Future<Output=Result<usize>> + Send>>  {
    //
    //     let mut transfer = transfer::Transfer::control(
    //         &self,
    //         request,
    //         direction,
    //         buf,
    //     )?;
    //     let mut rx = transfer.set_complete_cb();
    //
    //     transfer::Transfer::submit(transfer)?;
    //
    //    Pin::new( Box::new(async move{
    //         let r = rx.next().await.ok_or(Error::NotFound
    //         )??;
    //
    //         Ok(r.actual_length())
    //     }))
    // }
}


impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let des = self.descriptor();
        write!(f, r#"
pid: {}
vid: {}
"#, des.id_product(), des.id_vendor())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.dev);
            let handle = self.handle.lock().unwrap();
            if !handle.is_null() {
                self.event_controller.close_device();
                libusb_close(*handle);
            }
            debug!("Device closed");
        }
    }
}

pub struct Config{
    data: *const libusb_config_descriptor,
}

impl Config {
    pub fn value(&self)->u8{
        unsafe {
            (*self.data).bConfigurationValue
        }
    }
    pub fn max_power(&self) ->u8 {
        unsafe{
            (*self.data).bMaxPower
        }
    }
    pub fn extra(&self) -> &[u8] {
        unsafe {
            let e =    slice_from_raw_parts((*self.data).extra as *const u8, (*self.data).extra_length as usize);
            &*e
        }
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"
value: {}
max power: {}
extra: {:?}
        "#, self.value(), self.max_power() ,String::from_utf8_lossy(self.extra()))
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        if !self.data.is_null(){
            unsafe {
                libusb_free_config_descriptor(self.data);
            }
        }
    }
}

