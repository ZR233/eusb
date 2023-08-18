use std::sync::{Arc};
use std::time::Duration;
use log::{warn};
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

    pub async fn serial_number(&self) -> String {
        match self.ctx.serial_number().await {
            Ok(s) => { s }
            Err(e) => {
                warn!("{}",e);
                String::new()
            }
        }
    }

    pub fn configs(&self)->Vec<Config>{
        self.ctx.configs()
    }

    pub fn get_interface(&self, num: usize)->  Result<Interface>{
        self.ctx.get_interface(num)
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


    pub fn request_channel(&self, buffer: usize) -> (RequestSender, RequestReceiver) {
        self.ctx.transfer_channel(buffer)
    }

    pub fn bulk_request(&self, endpoint: Endpoint,
                        package_len: usize,
                        timeout: Duration)->Result<Request>{
        self.ctx.bulk_request(endpoint, package_len, timeout)
    }
}