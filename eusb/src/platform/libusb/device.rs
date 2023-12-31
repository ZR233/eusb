use std::future::Future;
use std::ptr::{null, null_mut, slice_from_raw_parts, slice_from_raw_parts_mut};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use libusb_src::*;

use crate::define::{ConfigDescriptor, ControlTransferRequest, DeviceClass, DeviceDescriptor, Direction, PipConfig, Speed};
use crate::platform::{AsyncResult, DeviceCtx};
use crate::platform::libusb::{class_from_lib, config_descriptor_convert, status_to_result, ToLib};
use crate::platform::libusb::device_handle::{DeviceHandle, sync_cb, TransferDirection};
use crate::platform::libusb::endpoint::EndpointPipInImpl;
use crate::platform::libusb::errors::*;
use crate::platform::libusb::transfer::Transfer;

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
#[allow(unused)]
impl DeviceCtxImpl {
    fn open(&self) -> Result {
        open(&self.dev, &self.opened)?;
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
        let interface_num = endpoint_get_interface_num(&cfg, endpoint);
        self.use_opened(|h| {
            h.handle.claim_interface(interface_num)?;
            Ok(())
        })?;


        Ok(())
    }


    fn async_handle<'a, T, F, O>(&self, f: F) -> impl Future<Output=Result<T>>
        where F: FnOnce(Arc<Device>, Arc<DeviceHandle>) -> O + 'a + Send,
              O: Future<Output=Result<T>> + Send + 'a
    {
        let opened = self.opened.clone();
        let dev = self.dev.clone();
        async move {
            let handle = open(&dev, &opened)?;
            f(dev, handle).await
        }
    }

    fn get_configuration_descriptor(&self, index: u8) -> Result<ConfigDescriptor> {
        let g = self.opened.lock().unwrap();
        let handle = g.as_ref().map(|o| o.handle.as_ref());
        self.dev.get_config_descriptor(index, handle)
    }
}

fn open(dev: &Arc<Device>, opened: &Arc<Mutex<Option<OpenedDevice>>>) -> Result<Arc<DeviceHandle>> {
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

fn endpoint_get_interface_num(cfg: &ConfigDescriptor, endpoint: u8) -> u8 {
    for alt in &cfg.interfaces {
        for interface in &alt.alt_settings {
            for ep in &interface.endpoints {
                if ep.num == endpoint {
                    return interface.num;
                }
            }
        }
    }
    0
}

fn open_endpoint(endpoint: u8, dev: &Arc<Device>, handle: &Arc<DeviceHandle>) -> Result {
    let cfg = dev.get_active_config_descriptor(None)?;
    let interface_num = endpoint_get_interface_num(&cfg, endpoint);
    handle.claim_interface(interface_num)?;
    Ok(())
}
macro_rules! async_opened {
    ($self:ident, $a: ident, $b: ident, $f: expr) => {
         Box::pin($self.async_handle(move |$a, $b| {
            async move {
                $f
            }
        }))
    };
}

impl DeviceCtx for DeviceCtxImpl {
    fn device_descriptor(&self) -> Result<DeviceDescriptor> {
        self.dev.device_descriptor()
    }

    fn get_string_ascii(&self, index: u8) -> Result<String> {
        self.use_opened(move |h| {
            h.handle.get_string_descriptor_ascii(index)
        })
    }

    fn device_class(&self) -> Result<DeviceClass> {
        Ok(class_from_lib(self.device_descriptor()?.bDeviceClass))
    }

    fn device_subclass(&self) -> Result<DeviceClass> {
        Ok(class_from_lib(self.device_descriptor()?.bDeviceSubClass))
    }

    fn device_protocol(&self) -> Result<DeviceClass> {
        Ok(class_from_lib(self.device_descriptor()?.bDeviceProtocol))
    }

    fn config_list(&self) -> Result<Vec<ConfigDescriptor>> {
        let des = self.device_descriptor()?;
        let g = self.opened.lock().unwrap();
        let handle = g.as_ref().map(|o| o.handle.as_ref());
        let mut out = Vec::with_capacity(des.bNumConfigurations as usize);
        for i in 0..des.bNumConfigurations{
            let elem = self.dev.get_config_descriptor(i, handle)?;
            out.push(elem);
        }

        Ok(out)
    }

    fn set_config_by_value(&self, config_value: u8) -> Result {
        let cfg_old = self.get_active_configuration()?;
        if cfg_old.value == config_value {
            return Ok(());
        }

        self.use_opened(move |opened| {
            match opened.handle.set_auto_detach_kernel_driver_with_guard(false) {
                Ok(guard) => {
                    let mut interfaces = vec![];

                    for atl in &cfg_old.interfaces {
                        for interface in &atl.alt_settings {
                            interfaces.push(interface.num);
                        }
                    }

                    for i in interfaces {
                        match opened.handle.kernel_driver_active(i) {
                            Ok(active) => {
                                if active {
                                    opened.handle.detach_kernel_driver(i)?;
                                }
                            }
                            Err(e) => {
                                match e {
                                    Error::NotSupported => { break; }
                                    _ => { return Err(e); }
                                }
                            }
                        }
                        let _ = opened.handle.release_interface(i);
                    }
                    drop(guard);
                }
                Err(e) => {
                    match e {
                        Error::NotSupported => {}
                        _ => { return Err(e); }
                    }
                }
            };

            opened.handle.set_configuration(config_value)?;
            Ok(())
        })
    }

    fn serial_number(&self) -> Result<String> {
        let des = self.device_descriptor()?;
        self.get_string_ascii(des.iSerialNumber)
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
        &self, control_transfer_request: ControlTransferRequest, capacity: usize) -> AsyncResult<Vec<u8>> {
        let rt: u8 = Direction::In.to_lib() | control_transfer_request.transfer_type.to_lib() | control_transfer_request.recipient.to_lib();

        async_opened!(self, _dev, handle, {
            let tran = handle.control_transfer(
                    TransferDirection::In { len: capacity }, rt as _,
                    control_transfer_request.request,
                    control_transfer_request.value,
                    control_transfer_request.index,
                    control_transfer_request.timeout).await?;
                Ok(tran.control_transfer_get_data().to_vec())
        })
    }

    fn control_transfer_out(
        &self,
        control_transfer_request: ControlTransferRequest, data: &[u8]) -> AsyncResult<usize> {
        let rt: u8 = Direction::Out.to_lib() | control_transfer_request.transfer_type.to_lib() | control_transfer_request.recipient.to_lib();
        let data = data.to_vec();

        async_opened!(self, _dev, handle, {
            let tran = handle.control_transfer(
                TransferDirection::Out { data }, rt,
                control_transfer_request.request,
                control_transfer_request.value,
                control_transfer_request.index,
                control_transfer_request.timeout).await?;
            Ok(tran.actual_length())
        })
    }

    fn bulk_transfer_in(&self, endpoint: u8, capacity: usize, timeout: Duration) -> AsyncResult<Vec<u8>> {
        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;

            let tran = handle.bulk_transfer(
                TransferDirection::In { len: capacity },
                endpoint, timeout, false).await?;

            Ok(tran.data[..tran.actual_length()].to_vec())
        })
    }

    fn bulk_transfer_out(&self, endpoint: u8, data: &[u8], timeout: Duration) -> AsyncResult<usize> {
        let data = data.to_vec();

        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;

            let tran = handle.bulk_transfer(
                TransferDirection::Out { data },
                endpoint, timeout, false).await?;

            Ok(tran.actual_length())
        })
    }

    fn interrupt_transfer_in(&self, endpoint: u8, capacity: usize, timeout: Duration) -> AsyncResult<Vec<u8>> {
        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;

            let tran = handle.bulk_transfer(
                TransferDirection::In { len: capacity },
                endpoint, timeout, true).await?;

            Ok(tran.data[..tran.actual_length()].to_vec())
        })
    }

    fn interrupt_transfer_out(&self, endpoint: u8, data: &[u8], timeout: Duration) -> AsyncResult<usize> {
        let data = data.to_vec();

        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;

            let tran = handle.bulk_transfer(
                TransferDirection::Out { data },
                endpoint, timeout, true).await?;

            Ok(tran.actual_length())
        })
    }

    fn iso_transfer_in(&self, endpoint: u8, num_iso_packages: usize, package_capacity: usize, timeout: Duration) -> AsyncResult<Vec<Vec<u8>>> {
        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;

            let tran = handle.iso_transfer(
                TransferDirection::In { len: num_iso_packages * package_capacity },
                endpoint, num_iso_packages,timeout).await?;

            let mut packs = Vec::with_capacity(num_iso_packages);

            unsafe{
                let packs_raw = &*slice_from_raw_parts((*tran.ptr).iso_packet_desc.as_ptr(), num_iso_packages);
                let mut begin = 0;
                for raw in packs_raw{
                    status_to_result(raw.status)?;
                    packs.push(tran.data[begin.. begin + raw.actual_length as usize].to_vec());
                    begin += raw.length as usize;
                }
            }
            Ok(packs)
        })
    }

    fn iso_transfer_out(&self, endpoint: u8, mut packs: Vec<Vec<u8>>, timeout: Duration) -> AsyncResult<Vec<usize>> {
        async_opened!(self, dev, handle, {
            open_endpoint(endpoint, &dev, &handle)?;
            let mut out = vec![0usize; packs.len()];
            let pack_lens: Vec<_> = packs.iter().map(|o|o.len()).collect();
            let num_iso_packets = pack_lens.len();
            let mut data = vec![];
            while let Some(mut o) = packs.pop(){
                data.append(&mut o);
            }
            unsafe {
                let tran = Transfer::iso_transfer(endpoint, num_iso_packets as _, sync_cb,  TransferDirection::Out{ data  }, timeout);
                let mut packs_raw = &mut*slice_from_raw_parts_mut((*tran.ptr).iso_packet_desc.as_mut_ptr(), num_iso_packets);
                for (i,raw) in packs_raw.iter_mut().enumerate(){
                    raw.length = pack_lens[i] as _;
                }
                let tran_new =  handle.do_sync_transfer(tran).await?;
                packs_raw = &mut*slice_from_raw_parts_mut((*tran_new.ptr).iso_packet_desc.as_mut_ptr(), num_iso_packets);
                 for (i,raw) in packs_raw.iter_mut().enumerate(){
                    out[i] = raw.actual_length as _;
                }
            }
            Ok(out)
        })
    }


    fn bulk_transfer_pip_in(&self, endpoint: u8, pip_config: PipConfig) -> Result<EndpointPipInImpl> {
        let handle = open(&self.dev, &self.opened)?;
        self.open_endpoint(endpoint)?;
        EndpointPipInImpl::new(&handle, endpoint, pip_config)
    }
}


pub(crate) struct Device(*mut libusb_device);

unsafe impl Send for Device {}

unsafe impl Sync for Device {}

#[allow(unused)]
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

    pub fn get_config_descriptor(&self, index: u8, handle: Option<&DeviceHandle>) -> Result<ConfigDescriptor> {
        let speed = self.speed()?;
        unsafe {
            let mut raw = null();
            check_err(libusb_get_config_descriptor(self.0, index, &mut raw))?;
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

