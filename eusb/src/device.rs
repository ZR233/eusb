use std::ffi::{c_int};
use std::fmt::{Debug, Display, Formatter};
use std::ptr::{null_mut, slice_from_raw_parts};
use std::sync::{Mutex, MutexGuard};
use log::debug;
use libusb_src::*;
#[cfg(unix)]
use std::os::unix::io::RawFd;
use crate::define::*;
use crate::error::*;
use crate::interface::Interface;
use crate::prelude::{TransferIn, UsbManager};
use crate::transfer::{Transfer, TransferOut, TransferWarp};

pub struct Device {
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<*mut libusb_device_handle>,
    manager: UsbManager,
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
        manager: &UsbManager
    ) -> Self {
        unsafe {
            libusb_ref_device(dev);
        }

        Self {
            dev,
            handle: Mutex::new(null_mut()),
            manager: manager.clone(),
        }
    }

    #[cfg(unix)]
    pub(crate) fn from_fd(
        ctx:  *mut libusb_context,
        fd: RawFd,
        manager: &UsbManager
    ) -> Result<Self> {
        unsafe {
            let mut handle= null_mut();
            check_err(libusb_wrap_sys_device(ctx, fd as _, &mut handle))?;
            let dev = libusb_get_device(handle);

            Ok(Self {
                dev,
                handle: Mutex::new(handle),
                manager: manager.clone(),
            })
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

    pub fn get_active_config_descriptor(&self)->Result<Config>{
        let mut cfg = Config{
            data: null_mut(),
        };
        unsafe {
            check_err(libusb_get_active_config_descriptor(
                self.dev,
                &mut cfg.data,
            ))?;
        }
        Ok(cfg)
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

    unsafe fn get_handle_in_mutex(&self, dev_handle: &mut MutexGuard<*mut libusb_device_handle>)->Result<*mut libusb_device_handle>{
        let mut dev_handle_ptr = **dev_handle;
        if dev_handle.is_null() {
            let r = libusb_open(self.dev, &mut dev_handle_ptr);
            check_err(r)?;
            self.manager.ctx.event_controller.open_device();

            libusb_set_auto_detach_kernel_driver(dev_handle_ptr, 1);

            **dev_handle = dev_handle_ptr;
        }


        Ok(dev_handle_ptr)
    }
    pub(crate) fn get_handle(&self) -> Result<*mut libusb_device_handle> {
        let mut g = self.handle.lock().unwrap();

        let dev = unsafe {
             self.get_handle_in_mutex(&mut g)?
        };
        Ok(dev)
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
        let mut g = self.handle.lock().unwrap();
        unsafe {
            let dev = self.get_handle_in_mutex(&mut g)?;
            let mut config_old: c_int = 0;
            let ptr = (&mut config_old) as *mut c_int;
            let mut r = libusb_get_configuration(dev, ptr);
            check_err(r)?;

            if config!= config_old {
                r = libusb_set_auto_detach_kernel_driver(dev, 0);
                let cfg = self.get_active_config_descriptor()?;
                for i in 0..cfg.num_interface(){
                    r  = libusb_kernel_driver_active(dev, i as _);
                    if r  < 0 {
                        if r  == LIBUSB_ERROR_NOT_SUPPORTED {
                            break;
                        }
                        check_err(r)?;
                    } else if r == 1 {
                        r = libusb_detach_kernel_driver(dev, i as _);
                        check_err(r)?;
                    }
                    self.get_interface_in_mutex(i, dev)?;
                }

                let config: c_int = config as _;
                r = libusb_set_configuration(dev, config);
                if r<0{
                    libusb_set_auto_detach_kernel_driver(dev, 1);
                }
                check_err(r)?;

                libusb_set_auto_detach_kernel_driver(dev, 1);
            }
        }
        Ok(())
    }
    fn get_interface_in_mutex(&self, index: usize, dev_handle: *mut libusb_device_handle) -> Result<Interface> {
        Interface::new_claimed(dev_handle, index)
    }

    pub fn get_interface(&self, index: usize) -> Result<Interface> {
        let dev_handle = self.get_handle()?;
        Interface::new_claimed(dev_handle, index)
    }

    pub fn control_transfer_in_request(&self, request: ControlTransferRequest, max_len: u16) -> Result<TransferIn>{
        let transfer = Transfer::control(
            &self,
            request,
            EndpointDirection::In,
            max_len,
            &[]
        )?;
        Ok(TransferIn::from_base(transfer))
    }
    pub fn control_transfer_out_request(&self, request: ControlTransferRequest, data: &[u8])-> Result<TransferOut> {
        let transfer = Transfer::control(
            &self,
            request,
            EndpointDirection::Out,
            0,
            data
        )?;
        Ok(TransferOut::from_base(transfer))
    }

    pub async fn control_transfer_in(&self, request: ControlTransferRequest, max_len: u16) -> Result<Vec<u8>>{
        let t1 = self.control_transfer_in_request(request, max_len)?;
        let t2 = t1.submit()?.await?;
        Ok(Vec::from(t2.data()))
    }
    pub async fn control_transfer_out(&self, request: ControlTransferRequest, data: &[u8])-> Result<usize> {
        let transfer = self.control_transfer_out_request(request, data)?;
        let tran_new =  transfer.submit()?.await?;
        Ok(tran_new.actual_length())
    }
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
                self.manager.ctx.event_controller.close_device();
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
            let e = slice_from_raw_parts((*self.data).extra as *const u8, (*self.data).extra_length as usize);
            &*e
        }
    }

    pub fn num_interface(&self)->usize{
        unsafe {
            (*self.data).bNumInterfaces as _
        }
    }

    // pub fn interfaces(&self)->Result<Vec<Interface>>{
    //     let l = vec![];
    //     unsafe {
    //         let list = &*slice_from_raw_parts((*self.data).interface, self.num_interface());
    //         for d in list{
    //             let interface =  d.altsetting.bInterfaceNumber
    //         }
    //     }
    //
    //     Ok(l)
    // }

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

