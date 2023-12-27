use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex};
use crate::platform::{DeviceCtx};
use libusb_src::*;
use crate::define::{ConfigDescriptor, DeviceDescriptor, Speed};
use crate::platform::libusb::config_descriptor_convert;
use crate::platform::libusb::device_handle::DeviceHandle;
use crate::platform::libusb::endpoint::EndpointInImpl;
use crate::platform::libusb::errors::*;

pub(crate) struct DeviceCtxImpl {
    dev: Arc<Device>,
    opened: Arc<Mutex<Option<OpenedDevice>>>,
}

struct OpenedDevice{
    handle: DeviceHandle,
}

impl OpenedDevice {
    fn new(handle: DeviceHandle)->Self{
        Self {
            handle,
        }
    }
}

impl From<Device> for DeviceCtxImpl {
    fn from(value: Device) -> Self {
        Self {
            dev: Arc::new(value),
            opened: Arc::new(Mutex::new(None)),
        }
    }
}

impl From<DeviceHandle> for DeviceCtxImpl {
    fn from(value: DeviceHandle) -> Self {
        let dev = value.get_device();
        Self{
            dev: Arc::new(dev),
            opened: Arc::new(Mutex::new(Some(OpenedDevice::new(value))))
        }
    }
}

impl DeviceCtxImpl {
    fn open(&self) -> Result {
        let g = self.opened.lock().unwrap();
        if g.is_some() {
            return Ok(());
        }
        drop(g);
        let h = self.dev.open()?;
        let mut g = self.opened.lock().unwrap();
        *g = Some(OpenedDevice::new(h));

        Ok(())
    }

    fn use_opened<F, O>(&self, f: F) -> Result<O>
        where F: FnOnce(&mut OpenedDevice) -> Result<O> {
        self.open()?;
        let mut g = self.opened.lock().unwrap();
        let h = g.as_mut().unwrap();
        f(h)
    }

    fn open_endpoint(&self, endpoint: u8)->Result{
        let cfg = self.dev.get_active_config_descriptor(None)?;
        let mut interface_num = 0;
        for alt in cfg.interfaces{
            for interface in alt.alt_settings{
                for ep in interface.endpoints{
                    if ep.num == endpoint{
                        interface_num = interface.num;
                    }
                }
            }
        }
        self.use_opened(|h|{
            h.handle.claim_interface(interface_num)?;
            Ok(())
        })?;


        Ok(())
    }
}

impl DeviceCtx for DeviceCtxImpl {
    fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        self.dev.device_descriptor()
    }

    fn serial_number(&self) -> Result<String> {
        let des = self.device_descriptor()?;
        self.use_opened(move |h| {
            h.handle.get_string_descriptor_ascii(des.iSerialNumber)
        })
    }

    fn bus_number(&self) -> u8 {
        unsafe {
            libusb_get_bus_number(self.dev.0)
        }
    }

    fn device_address(&self) -> u8 {
        unsafe {
            libusb_get_device_address(self.dev.0)
        }
    }

    fn get_active_configuration(&self) -> Result<ConfigDescriptor> {
        let g = self.opened.lock().unwrap();
        let handle = g.as_ref().map(|o| &o.handle);
        self.dev.get_active_config_descriptor(handle)
    }

    fn open_endpoint_in(&self, endpoint: u8) -> Result<EndpointInImpl> {
        self.open_endpoint(endpoint)?;
        Ok(EndpointInImpl{})
    }
}


pub(crate) struct Device(*mut libusb_device);

unsafe impl Send for Device {}

unsafe impl Sync for Device {}

impl Device {
    pub fn open(&self)->Result< DeviceHandle>{
       unsafe {
           let mut ptr = null_mut();
           check_err(libusb_open(self.0, &mut ptr))?;
           let h = DeviceHandle::from(ptr);
           Ok(h)
       }
    }

    fn speed(&self) -> Result<Speed> {
        unsafe {
            let r = check_err( libusb_get_device_speed(self.0))?;
            Ok(match r {
                LIBUSB_SPEED_LOW=> Speed::Low,
                LIBUSB_SPEED_FULL=> Speed::Full,
                LIBUSB_SPEED_HIGH=> Speed::High,
                LIBUSB_SPEED_SUPER=> Speed::Super,
                LIBUSB_SPEED_SUPER_PLUS=> Speed::SuperPlus,
                _ => Speed::Unknown
            })
        }
    }

    pub fn get_max_packet_size(&self, endpoint: usize) -> Result<usize> {
        unsafe {
            let r = check_err(libusb_get_max_packet_size(self.0, endpoint as _))?;
            Ok(r as _)
        }
    }

    fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        let out = unsafe {
            let mut des = libusb_device_descriptor::default();
            libusb_get_device_descriptor(self.0, &mut des);

            DeviceDescriptor {
                bLength: des.bLength,
                bDescriptorType: des.bDescriptorType,
                bcdUSB: des.bcdUSB,
                bDeviceClass: des.bDeviceClass,
                bDeviceSubClass: des.bDeviceSubClass,
                bDeviceProtocol: des.bDeviceProtocol,
                bMaxPacketSize0: des.bMaxPacketSize0,
                idVendor: des.idVendor,
                idProduct: des.idProduct,
                bcdDevice: des.bcdDevice,
                iManufacturer: des.iManufacturer,
                iProduct: des.iProduct,
                iSerialNumber: des.iSerialNumber,
                bNumConfigurations: des.bNumConfigurations,
            }
        };
        Ok(out)
    }

    pub fn get_active_config_descriptor(&self, handle: Option<&DeviceHandle>)->Result<ConfigDescriptor>{
        let speed = self.speed()?;
        unsafe {
            let mut raw = null();
            check_err(libusb_get_active_config_descriptor(self.0, &mut raw))?;
            let cfg = config_descriptor_convert(raw, handle, speed);
            libusb_free_config_descriptor(raw);
            Ok(cfg)
        }
    }
}


impl From<*mut libusb_device> for Device {
    fn from(value: *mut libusb_device) -> Self {
        Self(value)
    }
}


impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.0)
        }
    }
}

