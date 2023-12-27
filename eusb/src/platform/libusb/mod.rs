use std::ffi::{c_int, c_uchar};
use std::ptr::slice_from_raw_parts;
use libusb_src::*;
use crate::define::*;
use crate::platform::libusb::device_handle::DeviceHandle;

pub(crate) mod context;
pub(crate) mod device;
mod errors;
mod device_handle;
pub(crate)mod manager;
mod transfer;
pub(crate) mod endpoint;


pub(crate) unsafe  fn  config_descriptor_convert(raw: *const libusb_config_descriptor, handle: Option<&DeviceHandle>, speed: Speed)->ConfigDescriptor{
    let mut alt_settings = Vec::with_capacity((*raw).bNumInterfaces as _);
    let interface_list = &*slice_from_raw_parts(
        (*raw).interface,
        alt_settings.capacity());
    for desc in interface_list {
        let mut alts = Vec::with_capacity(desc.num_altsetting as _);

        let alts_ptr = &*slice_from_raw_parts(
            desc.altsetting,
            desc.num_altsetting as _
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
                    if let Ok(s) = h.get_string_descriptor_ascii(interface.iInterface) {interface_string=s}
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
    let mut max_power = 0;
    let p =  match speed {
        Speed::Unknown => {0}
        Speed::Low => {0}
        Speed::Full => {0}
        Speed::High => {2}
        Speed::Super => {2}
        Speed::SuperPlus => {8}
    } as usize;
    max_power = p * ((*raw).bMaxPower as usize);
    let extra = get_extra((*raw).extra, (*raw).extra_length);
    let mut configuration = String::new();
    match handle {
        None => {}
        Some(h) => {
            if let Ok(s) = h.get_string_descriptor_ascii((*raw).iConfiguration) {configuration=s}
        }
    }
    ConfigDescriptor{
        value: (*raw).bConfigurationValue,
        interfaces: alt_settings,
        extra,
        max_power,
        configuration,
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
        let  data = &*slice_from_raw_parts(extra, extra_length as usize);
        data.to_vec()
    }
}
pub(crate) fn class_from_lib(class: u8)->DeviceClass{
    match class {
        LIBUSB_CLASS_PER_INTERFACE=> DeviceClass::PerInterface,
        LIBUSB_CLASS_AUDIO => DeviceClass::Audio,
        LIBUSB_CLASS_COMM => DeviceClass::Comm,
        LIBUSB_CLASS_HID => DeviceClass::Hid,
        LIBUSB_CLASS_PHYSICAL => DeviceClass::Physical,
        LIBUSB_CLASS_PRINTER => DeviceClass::Printer,
        LIBUSB_CLASS_IMAGE => DeviceClass::Image,
        LIBUSB_CLASS_MASS_STORAGE => DeviceClass::MassStorage,
        LIBUSB_CLASS_HUB => DeviceClass::Hub,
        LIBUSB_CLASS_DATA => DeviceClass::Data,
        LIBUSB_CLASS_SMART_CARD => DeviceClass::SmartCard,
        LIBUSB_CLASS_CONTENT_SECURITY => DeviceClass::ContentSecurity,
        LIBUSB_CLASS_VIDEO => DeviceClass::Video,
        LIBUSB_CLASS_PERSONAL_HEALTHCARE => DeviceClass::PersonalHealthcare,
        LIBUSB_CLASS_DIAGNOSTIC_DEVICE => DeviceClass::DiagnosticDevice,
        LIBUSB_CLASS_WIRELESS => DeviceClass::Wireless,
        LIBUSB_CLASS_APPLICATION => DeviceClass::Application,
        LIBUSB_CLASS_VENDOR_SPEC => DeviceClass::VendorSpec,
        _ => panic!("Unknown class: {}", class)
    }
}
