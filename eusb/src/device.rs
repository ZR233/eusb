use std::sync::{Arc};
use std::time::Duration;
use crate::platform::*;
use crate::adaptor::{CtxDevice, EndpointDirection, RequestParamControlTransfer};
use crate::define::*;
use crate::error::*;

#[derive(Clone)]
pub struct Device {
    ctx: Arc<CtxDeviceImpl>,
}
fn bcd_to_version(bcd: u16) -> Vec<u16> {
    let  bcd_major = (bcd & 0xF000) >> 12;
    let  bcd_minor = (bcd & 0x0F00) >> 8;
    let  bcd_micro = (bcd & 0x00F0) >> 4;
    let  bcd_nano  = (bcd & 0x000F) >> 0;
    vec![bcd_major, bcd_minor, bcd_micro, bcd_nano]
}

impl Device {
    #[cfg(libusb)]
    pub(crate) fn new(mut value: CtxDeviceImpl, manager: Arc<Manager>) -> Self {
        value.manager = Some(manager);
        let ctx = Arc::new(value);
        Self {
            ctx
        }
    }

    pub fn pid(&self) -> u16 {
        self.ctx.pid()
    }

    pub fn vid(&self) -> u16 {
        self.ctx.vid()
    }

    pub async fn serial_number(&self) -> Result<String> {
        self.ctx.serial_number().await
    }

    pub fn config_list(&self) ->Result<Vec<ConfigDescriptor>>{
        self.ctx.config_list()
    }

    pub fn get_config(&self)->Result<ConfigDescriptor>{ self.ctx.get_config() }
    pub fn set_config(&self, config: ConfigDescriptor)->Result<()>{ self.ctx.set_config(config.value)}
    pub fn set_config_by_value(&self, config_value: u8)->Result<()>{ self.ctx.set_config(config_value)}

    pub fn claim_interface_by_num(&self, num: usize) ->  Result<Interface>{
        self.ctx.claim_interface(num)
    }
    pub fn claim_interface(&self, interface: InterfaceDescriptor) ->  Result<Interface>{
        self.ctx.claim_interface(interface.num as _)
    }

    pub async fn control_transfer_in(
        &self,
        recipient: UsbControlRecipient,
        transfer_type: UsbControlTransferType,
        request: u8,
        value: u16,
        index: u16,
        timeout: Duration,
        capacity: usize,
    ) -> Result<Request> {
        let request = self.ctx.control_request(RequestParamControlTransfer {
            recipient,
            transfer_type,
            request,
            value,
            index,
            timeout,
        }, EndpointDirection::In { capacity })?;

        let (mut tx, mut rx) = self.request_channel(1);
        tx.send(request)?;
        rx.next().await.unwrap()
    }
    pub async fn control_transfer_out(
        &self,
        recipient: UsbControlRecipient,
        transfer_type: UsbControlTransferType,
        request: u8,
        value: u16,
        index: u16,
        timeout: Duration,
        src: &mut [u8],
    ) -> Result<Request> {
        let request = self.ctx.control_request(RequestParamControlTransfer {
            recipient,
            transfer_type,
            request,
            value,
            index,
            timeout,
        }, EndpointDirection::Out { src })?;

        let (mut tx, mut rx) = self.request_channel(1);
        tx.send(request)?;
        rx.next().await.unwrap()
    }
    pub fn speed(&self)->Result<Speed>{
        self.ctx.speed()
    }
    pub fn bcd_usb(&self) -> u16 {
        self.ctx.bcd_usb()
    }
    pub fn bcd_usb_version(&self) -> Vec<u16> {
        return bcd_to_version(self.bcd_usb())
    }

    pub fn device_class(&self) -> DeviceClass {
        self.ctx.device_class()
    }


    pub fn device_subclass(&self) -> DeviceClass {
        self.ctx.device_subclass()
    }

    pub fn device_protocol(&self) -> DeviceClass {
        self.ctx.device_protocol()
    }

    pub fn max_packet_size_0(&self) -> usize {
        self.ctx.max_packet_size_0()
    }

    pub fn bcd_device(&self) -> u16 {
        self.ctx.bcd_device()
    }
    pub fn bcd_device_version(&self) -> Vec<u16> {
        return bcd_to_version(self.bcd_device())
    }
    pub fn manufacturer(&self) -> Result<String> {
        self.ctx.manufacturer()
    }

    pub fn product(&self) -> Result<String> {
        self.ctx.product()
    }
    pub fn request_channel(&self, buffer: usize) -> (RequestSender, RequestReceiver) {
        self.ctx.transfer_channel(buffer)
    }
}