use std::ffi::{c_int, c_uchar, CStr};
use std::ptr::{null, null_mut, slice_from_raw_parts};
use std::sync::{Arc, Mutex, MutexGuard};
use log::{trace};
use libusb_src::*;
pub(crate) use super::super::ResultFuture;
pub(crate) use super::super::CtxDevice;
use super::interface::Interface;
use super::Manager;
use super::ptr::*;
use crate::error::*;
use crate::platform::Request;
use crate::adaptor::libusb::channel::{request_channel, RequestReceiver, RequestSender};
use crate::adaptor::*;

pub(crate) struct CtxDeviceImpl {
    pub(crate) dev: *mut libusb_device,
    pub(crate) handle: Mutex<DeviceHandle>,
    pub(crate) manager: Option<Arc<Manager>>,

}

unsafe impl Send for CtxDeviceImpl {}
unsafe impl Sync for CtxDeviceImpl {}

fn get_string(handle: DeviceHandle, index: u8)->Result<String>{
    let mut data = vec![0 as c_uchar; 1024];
    unsafe {
        libusb_get_string_descriptor_ascii(handle.0, index, data.as_mut_ptr(), data.len() as _);
        let str = CStr::from_bytes_until_nul(&data)
            .map_err(|e|{
                Error::Other(e.to_string())
            })?;
        let str = str.to_str().map_err(|e|{
            Error::Other(e.to_string())
        })?;
        Ok(str.to_string())
    }
}
fn get_extra(
   extra: *const c_uchar,
   extra_length: c_int,
)->Vec<u8>{
    if extra_length ==0 {
        return vec![];
    }

    unsafe {
        let  data = &*slice_from_raw_parts(extra as *const u8, extra_length as usize);
        data.to_vec()
    }
}

impl CtxDeviceImpl {
    pub(crate) fn new(dev: *mut libusb_device)->Self{
        return Self{
            dev,
            handle: Mutex::new(DeviceHandle(null_mut())),
            manager: None,
        }
    }

    fn descriptor(&self) -> libusb_device_descriptor {
        let mut desc =libusb_device_descriptor::default();
        unsafe {
            let _ = libusb_get_device_descriptor(self.dev, &mut desc);
        }
        desc
    }

    pub(crate) fn get_handle_with_guard(&self) -> Result<MutexGuard<DeviceHandle>> {
        let mut g = self.handle.lock().unwrap();
        unsafe {
            if g.is_null() {
                let r = libusb_open(self.dev, &mut g.0);
                check_err(r)?;
                let manager = self.manager.clone().unwrap();
                manager.open_device();

                libusb_set_auto_detach_kernel_driver(g.0, 1);
            }
        }
        Ok(g)
    }

    pub(crate) fn get_handle(&self) -> Result<DeviceHandle> {
        let g = self.get_handle_with_guard()?;
        Ok(g.clone())
    }

    pub(crate)  fn transfer_channel(self: &Arc<Self>, buffer: usize) -> (RequestSender, RequestReceiver) {
        let (tx, rx) = request_channel(buffer);
        return (tx, rx)
    }

    fn get_active_config_ptr(self: &Arc<Self>) ->Result<ConfigDescriptorPtr>{
        unsafe {
            let mut ptr : *const libusb_config_descriptor = null();
            check_err(libusb_get_active_config_descriptor(
                self.dev,
                &mut ptr,
            ))?;
            Ok(ConfigDescriptorPtr::from(ptr))
        }
    }


    fn fill_config_descriptor(self: &Arc<Self>, config_ptr: ConfigDescriptorPtr) -> ConfigDescriptor {
        let handle = match self.get_handle(){
            Ok(h) => {Some(h)}
            Err(_) => {None}
        };

        unsafe {

            let mut alt_settings = Vec::with_capacity((*config_ptr.config).bNumInterfaces as _);
            let interface_list = &*slice_from_raw_parts(
                (*config_ptr.config).interface,
                alt_settings.capacity());
            for desc in interface_list {
                let mut alts = Vec::with_capacity((*desc).num_altsetting as _);

                let alts_ptr = &*slice_from_raw_parts(
                    (*desc).altsetting,
                    (*desc).num_altsetting as _
                );
                for interface in alts_ptr {
                    let mut endpoints = Vec::with_capacity(interface.bNumEndpoints as _);
                    let endpoint_list = &*slice_from_raw_parts(interface.endpoint, endpoints.capacity());
                    for endpoint in endpoint_list {
                        let direction = if endpoint.bEndpointAddress as u32 & LIBUSB_ENDPOINT_IN as u32 == LIBUSB_ENDPOINT_IN as u32{
                            Direction::In
                        } else { Direction::Out };
                        let num = endpoint.bEndpointAddress as u32 & LIBUSB_ENDPOINT_ADDRESS_MASK as u32;

                        let extra = get_extra(endpoint.extra, endpoint.extra_length);
                        let type_int = endpoint.bmAttributes as u32 & LIBUSB_TRANSFER_TYPE_MASK as u32;
                        let transfer_type = match type_int as u8 {
                            LIBUSB_TRANSFER_TYPE_ISOCHRONOUS => EndpointTransferType::Isochronous,
                            LIBUSB_TRANSFER_TYPE_INTERRUPT => EndpointTransferType::Interrupt,
                            LIBUSB_TRANSFER_TYPE_BULK => EndpointTransferType::Bulk,
                            LIBUSB_TRANSFER_TYPE_CONTROL => EndpointTransferType::Control,
                            _ => panic!("Transfer type error"),
                        };

                        let sync_type_int = endpoint.bmAttributes as u32 & LIBUSB_ISO_SYNC_TYPE_MASK as u32;
                        let sync_type = match sync_type_int  as u8{
                            LIBUSB_ISO_SYNC_TYPE_NONE => IsoSyncType::None,
                            LIBUSB_ISO_SYNC_TYPE_ASYNC=> IsoSyncType::Async,
                            LIBUSB_ISO_SYNC_TYPE_ADAPTIVE => IsoSyncType::Adaptive,
                            LIBUSB_ISO_SYNC_TYPE_SYNC => IsoSyncType::Sync,
                            _ => panic!("Iso sync type error {}", sync_type_int),
                        };

                        let usage_type_int = (endpoint.bmAttributes as u32 & LIBUSB_ISO_USAGE_TYPE_MASK as u32) as u8;
                        let usage_type = match usage_type_int {
                            LIBUSB_ISO_USAGE_TYPE_DATA => IsoUsageType::Data,
                            LIBUSB_ISO_USAGE_TYPE_FEEDBACK => IsoUsageType::Feedback,
                            LIBUSB_ISO_USAGE_TYPE_IMPLICIT => IsoUsageType::Implicit,
                            _ => IsoUsageType::Unknown(usage_type_int),
                        };


                        endpoints.push(EndpointDescriptor{
                            num: num as _,
                            direction,
                            transfer_type,
                            sync_type,
                            usage_type,
                            max_packet_size: endpoint.wMaxPacketSize,
                            interval: endpoint.bInterval,
                            refresh: endpoint.bRefresh,
                            synch_address: endpoint.bSynchAddress,
                            extra,
                        });
                    }
                    let extra = get_extra(interface.extra, interface.extra_length);
                    let mut interface_string = String::new();

                    match handle {
                        None => {}
                        Some(h) => {
                            match get_string(h, interface.iInterface) {
                                Ok(s) => {interface_string=s}
                                Err(_) => {}
                            }
                        }
                    }

                    alts.push(InterfaceDescriptor{
                        num: interface.bInterfaceNumber,
                        alt_setting: interface.bAlternateSetting,
                        device_class: class_from_lib(interface.bInterfaceClass),
                        device_sub_class: class_from_lib(interface.bInterfaceSubClass),
                        protocol: class_from_lib(interface.bInterfaceProtocol),
                        interface: interface_string,
                        endpoints,
                        extra
                    })
                }


                alt_settings.push(InterfaceAltSettingDescriptor{
                    alt_settings: alts,
                })
            }
            let extra = get_extra((*config_ptr.config).extra, (*config_ptr.config).extra_length);
            let mut configuration = String::new();
            match handle {
                None => {}
                Some(h) => {
                    match get_string(h, (*config_ptr.config).iConfiguration) {
                        Ok(s) => {configuration=s}
                        Err(_) => {}
                    }
                }
            }
            let mut max_power = 0;
            let speed = self.speed();
            match speed {
                Ok(s) => {
                  let p =  match s {
                        Speed::Unknown => {0}
                        Speed::Low => {0}
                        Speed::Full => {0}
                        Speed::High => {2}
                        Speed::Super => {2}
                        Speed::SuperPlus => {8}
                    } as usize;
                    max_power = p * ((*config_ptr.config).bMaxPower as usize)
                }
                Err(_) => {}
            }

            let config = ConfigDescriptor{
                value: (*config_ptr.config).bConfigurationValue,
                interfaces: alt_settings,
                extra,
                max_power,
                configuration,
            };

            config
        }
    }

    fn get_config_by_index(self: &Arc<Self>, index: u8)->Result<ConfigDescriptor>{
        let config_ptr= ConfigDescriptorPtr::new(self.dev, index)?;
        Ok(self.fill_config_descriptor(config_ptr))
    }
}

struct AutoDetachKernelDriverGuard{
    dev: *mut libusb_device_handle
}

impl Drop for AutoDetachKernelDriverGuard {
    fn drop(&mut self) {
        unsafe {
            libusb_set_auto_detach_kernel_driver(self.dev, 1);
        }
    }
}


impl CtxDevice<Request, Interface> for CtxDeviceImpl {
    fn pid(&self) -> u16 {
        let desc = self.descriptor();
        desc.idProduct
    }

    fn vid(&self) -> u16 {
        let desc = self.descriptor();
        desc.idVendor
    }

    fn serial_number(self: &Arc<Self>) -> ResultFuture<String> {
        let desc = self.descriptor();
        let s = self.clone();
        Box::pin(async move{
            let dev = s.get_handle_with_guard()?;
            let index = desc.iSerialNumber;
            let mut buff = vec![0u8; 256];
            let buff_len = buff.len();
            if index > 0 {
                unsafe {
                    let r = libusb_get_string_descriptor_ascii(
                        dev.0,
                        index,
                        buff.as_mut_ptr(),
                        buff_len as _
                    );
                    if r > 0{
                        buff.resize(r as _, 0);
                        match String::from_utf8(buff){
                            Ok(s) => {return Ok(s);}
                            Err(_) => {}
                        }
                    }

                }
            }
            Ok(String::new())
        })
    }

    fn speed(self: &Arc<Self>) -> Result<Speed> {
        unsafe {
            let r = libusb_get_device_speed(self.dev);
            check_err(r)?;

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


    fn bcd_usb(&self) -> u16 {
        self.descriptor().bcdUSB
    }

    fn device_class(&self) -> DeviceClass {
        class_from_lib(self.descriptor().bDeviceClass)
    }


    fn device_subclass(&self) -> DeviceClass {
        class_from_lib(self.descriptor().bDeviceSubClass)
    }

    fn device_protocol(&self) -> DeviceClass {
        class_from_lib(self.descriptor().bDeviceProtocol)
    }

    fn max_packet_size_0(&self) -> usize {
        self.descriptor().bMaxPacketSize0 as usize
    }

    fn bcd_device(&self) -> u16 {
        self.descriptor().bcdDevice
    }

    fn manufacturer(self: &Arc<Self>) -> Result<String> {
        get_string(self.get_handle()?, self.descriptor().iManufacturer)
    }

    fn product(self: &Arc<Self>) -> Result<String> {
        get_string(self.get_handle()?, self.descriptor().iProduct)
    }

    fn control_request(self: &Arc<Self>, param: RequestParamControlTransfer, direction: EndpointDirection) -> Result<Request> {
        let request = Request::control(self, param, direction)?;
        Ok(request)
    }

    fn claim_interface(self: &Arc<Self>, num: usize) -> Result<Interface> {
        Interface::new(self, num)
    }

    fn get_config(self: &Arc<Self>) -> Result<ConfigDescriptor> {
        let ptr = self.get_active_config_ptr()?;
        Ok(self.fill_config_descriptor(ptr))
    }

    fn set_config(self: &Arc<Self>, config: u8)->Result<()> {

        let guard = self.get_handle_with_guard()?;
        let dev = guard.0;
        unsafe {
            let mut need_set = true;
            let mut num_interfaces = 0;
            match self.get_active_config_ptr(){
                Ok(old_cfg) => {
                    num_interfaces = (*old_cfg.config).bNumInterfaces;
                    need_set = (*old_cfg.config).bConfigurationValue != config;
                }
                Err(_) => {}
            };
            let mut r;
            // {
            if need_set {
                libusb_set_auto_detach_kernel_driver(dev, 0);
                let auto_detach = AutoDetachKernelDriverGuard{dev};

                for i in 0..num_interfaces{
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
                    libusb_release_interface(dev, i as _);
                }

                let config: c_int = config as _;
                r = libusb_set_configuration(dev, config);
                check_err(r)?;
                drop(auto_detach);
            }
        }
        Ok(())
    }

    fn config_list(self: &Arc<Self>) -> Result<Vec<ConfigDescriptor>> {
        let desc = self.descriptor();
        let mut configs = Vec::with_capacity(desc.bNumConfigurations as _);

        for i in 0..desc.bNumConfigurations{
            let cfg = self.get_config_by_index(i)?;
            configs.push(cfg);
        }

        Ok(configs)
    }
}


impl Drop for CtxDeviceImpl {
    fn drop(&mut self) {
        unsafe {
            libusb_unref_device(self.dev);
            let handle = self.handle.lock().unwrap();
            if !handle.is_null() {
                self.manager.clone().unwrap().close_device();
                libusb_close(handle.0);
                trace!("Device closed");
            }
        }
    }
}

