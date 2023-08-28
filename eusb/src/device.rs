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

    pub fn request_channel(&self, buffer: usize) -> (RequestSender, RequestReceiver) {
        self.ctx.transfer_channel(buffer)
    }
}