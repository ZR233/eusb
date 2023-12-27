#![allow(unused)]

use std::time::Duration;

pub enum UsbControlRecipient {
    Device,
    SpecifiedInterface,
    Endpoint,
    Other,
    DefaultInterface
}

pub enum UsbControlTransferType {
    Standard,
    Class,
    Vendor,
    Reserved
}

pub struct ControlTransferRequest{
    pub recipient: UsbControlRecipient,
    pub transfer_type: UsbControlTransferType,
    pub request: u8,
    pub value: u16,
    pub index: u16,
    pub timeout: Duration,
}

impl Default for ControlTransferRequest {
    fn default() -> Self {
        Self{
            recipient: UsbControlRecipient::Device,
            transfer_type: UsbControlTransferType::Standard,
            request: 0,
            value: 0,
            index: 0,
            timeout: Default::default(),
        }
    }
}


#[derive(Debug)]
pub enum Direction{
    In, Out
}
#[derive(Debug)]
pub enum EndpointTransferType{
    Control, Isochronous, Bulk, Interrupt
}
#[derive(Debug)]
pub enum IsoSyncType{
    None, Async, Adaptive, Sync
}
#[derive(Debug)]
pub enum IsoUsageType{
    Data, Feedback, Implicit, Unknown(u8)
}
#[derive(Debug)]
pub enum Speed{
    Unknown, Low, Full, High, Super, SuperPlus
}

#[allow(non_snake_case)]
#[derive(Default, Debug)]
pub struct DeviceDescriptor{
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}
pub struct EndpointDescriptor {
    pub num: u8,
    pub direction: Direction,
    pub transfer_type: EndpointTransferType,
    pub sync_type: IsoSyncType,
    pub usage_type: IsoUsageType,
    pub max_packet_size: u16,
    pub interval: u8,
    pub refresh: u8,
    pub synch_address: u8,
    pub extra: Vec<u8>,
}

impl EndpointDescriptor {
    pub fn new(num: u8, direction: Direction) -> Self {
        Self{
            num,
            direction,
            transfer_type: EndpointTransferType::Control,
            sync_type: IsoSyncType::None,
            usage_type: IsoUsageType::Data,
            max_packet_size: 0,
            interval: 0,
            refresh: 0,
            synch_address: 0,
            extra: vec![],
        }
    }
}


#[derive(Debug)]
pub enum  DeviceClass{
    PerInterface,
    Audio,
    Comm,
    Hid,
    Physical,
    Image,
    Printer,
    MassStorage,
    Hub,
    Data,
    SmartCard,
    ContentSecurity,
    Video,
    PersonalHealthcare,
    DiagnosticDevice,
    Wireless,
    VendorSpec,
    Application,
}


pub struct InterfaceDescriptor {
    pub num: u8,
    pub alt_setting: u8,
    pub device_class: DeviceClass,
    pub device_sub_class: DeviceClass,
    pub protocol: DeviceClass,
    pub endpoints: Vec<EndpointDescriptor>,
    pub interface: String,
    pub extra: Vec<u8>
}
pub struct InterfaceAltSettingDescriptor {
    pub alt_settings: Vec<InterfaceDescriptor>
}

pub struct ConfigDescriptor {
    pub value: u8,
    pub interfaces: Vec<InterfaceAltSettingDescriptor>,
    pub extra: Vec<u8>,
    /// units of mA
    pub max_power: usize,
    pub configuration: String,
}

#[derive(Clone)]
pub struct PipConfig{
    pub pip_size: usize,
    pub package_size: usize,
    pub timeout: Duration,
}

impl Default for PipConfig {
    fn default() -> Self {
        Self{
            pip_size: 4,
            package_size:0,
            timeout: Default::default(),
        }
    }
}

