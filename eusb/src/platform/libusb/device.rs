use std::future::Future;
use std::pin::Pin;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex};
use crate::platform::{DeviceCtx};
use libusb_src::*;
use crate::define::{ConfigDescriptor, ControlTransferRequest, DeviceDescriptor, Direction, PipConfig, Speed};
use crate::platform::libusb::{config_descriptor_convert, ToLib};
use crate::platform::libusb::device_handle::DeviceHandle;
use crate::platform::libusb::endpoint::EndpointInImpl;
use crate::platform::libusb::errors::*;

pub(crate) struct DeviceCtxImpl {
    dev: Arc<Device>,
    opened: Arc<Mutex<Option<OpenedDevice>>>,
}

struct OpenedDevice {
    handle: Arc<DeviceHandle>,
}

impl OpenedDevice {
    fn new(handle: DeviceHandle) -> Self {
        Self {
            handle: Arc::new(handle),
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
        Self {
            dev: Arc::new(dev),
            opened: Arc::new(Mutex::new(Some(OpenedDevice::new(value)))),
        }
    }
}

impl DeviceCtxImpl {
    fn open(&self) -> Result {
        open(self.dev.clone(), &self.opened)?;
        Ok(())
    }

    fn use_opened<F, O>(&self, f: F) -> Result<O>
        where F: FnOnce(&mut OpenedDevice) -> Result<O> {
        self.open()?;
        let mut g = self.opened.lock().unwrap();
        let h = g.as_mut().unwrap();
        f(h)
    }

    fn open_endpoint(&self, endpoint: u8) -> Result {
        let cfg = self.dev.get_active_config_descriptor(None)?;
        let mut interface_num = 0;
        for alt in cfg.interfaces {
            for interface in alt.alt_settings {
                for ep in interface.endpoints {
                    if ep.num == endpoint {
                        interface_num = interface.num;
                    }
                }
            }
        }
        self.use_opened(|h| {
            h.handle.claim_interface(interface_num)?;
            Ok(())
        })?;


        Ok(())
    }
}

fn open(dev: Arc<Device>, opened: &Arc<Mutex<Option<OpenedDevice>>>) -> Result<Arc<DeviceHandle>> {
    let g = opened.lock().unwrap();
    if let Some(o) = g.as_ref() {
        return Ok(o.handle.clone());
    }
    drop(g);
    let h = dev.open()?;
    let mut g = opened.lock().unwrap();
    let o = OpenedDevice::new(h);
    let h = o.handle.clone();
    *g = Some(o);
    Ok(h)
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
        let handle = g.as_ref().map(|o| o.handle.as_ref());
        self.dev.get_active_config_descriptor(handle)
    }


    fn control_transfer_in(
        &self, control_transfer_request: ControlTransferRequest, capacity: usize) -> Pin<Box<dyn Future<Output=Result<Vec<u8>>>>> {
        let rt: u32 = Direction::In.to_lib() | control_transfer_request.transfer_type.to_lib() | control_transfer_request.recipient.to_lib();
        let opened = self.opened.clone();
        let dev = self.dev.clone();
        Box::pin(async move {
            let mut data = vec![0; capacity];
            let handle = open(dev, &opened)?;
            let tran = handle.control_transfer(
                data.as_mut(), rt as _,
                control_transfer_request.request,
                control_transfer_request.value,
                control_transfer_request.index,
                control_transfer_request.timeout).await?;
            Ok(tran.control_transfer_get_data().to_vec())
        })
    }

    fn control_transfer_out(
        &self,
        control_transfer_request: ControlTransferRequest, data: &[u8]) -> Pin<Box<dyn Future<Output=Result<usize>>>> {
        let rt: u32 = Direction::In.to_lib() | control_transfer_request.transfer_type.to_lib() | control_transfer_request.recipient.to_lib();
        let opened = self.opened.clone();
        let dev = self.dev.clone();
        let mut data = data.to_vec();
        Box::pin(async move {
            let handle = open(dev, &opened)?;
            let tran = handle.control_transfer(data.as_mut(), rt as _,
                                               control_transfer_request.request,
                                               control_transfer_request.value,
                                               control_transfer_request.index,
                                               control_transfer_request.timeout).await?;
            Ok(tran.actual_length())
        })
    }

    fn bulk_transfer_pip_in(&self, endpoint: u8, pip_config: PipConfig) -> Result<EndpointInImpl> {
        let handle = open(self.dev.clone(), &self.opened)?;
        self.open_endpoint(endpoint)?;
        Ok(EndpointInImpl::new(&handle, endpoint, pip_config))
    }
}


pub(crate) struct Device(*mut libusb_device);

unsafe impl Send for Device {}

unsafe impl Sync for Device {}

impl Device {
    pub fn open(&self) -> Result<DeviceHandle> {
        unsafe {
            let mut ptr = null_mut();
            check_err(libusb_open(self.0, &mut ptr))?;
            let h = DeviceHandle::from(ptr);
            Ok(h)
        }
    }

    fn speed(&self) -> Result<Speed> {
        unsafe {
            let r = check_err(libusb_get_device_speed(self.0))?;
            Ok(match r {
                LIBUSB_SPEED_LOW => Speed::Low,
                LIBUSB_SPEED_FULL => Speed::Full,
                LIBUSB_SPEED_HIGH => Speed::High,
                LIBUSB_SPEED_SUPER => Speed::Super,
                LIBUSB_SPEED_SUPER_PLUS => Speed::SuperPlus,
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

    pub fn get_active_config_descriptor(&self, handle: Option<&DeviceHandle>) -> Result<ConfigDescriptor> {
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

