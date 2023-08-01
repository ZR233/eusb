pub use crate::core::*;
pub use crate::define::*;
pub use crate::device::*;

#[cfg(test)]
mod test{
    use std::sync::Arc;
    use std::time::Duration;
    use super::*;


    async fn get_hackrf(manager: &UsbManager)->Arc<Device>{
        manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap()
    }

    #[tokio::test]
    async fn test_control_transfer_in() {
        {
            let manager = UsbManager::new().unwrap();
            let mut device = get_hackrf(&manager).await;

            println!("{} speed: {:?}", device, device.speed());

            // device.set_configuration(0x1).unwrap();
            let config = device.get_configuration().unwrap();

            println!("config: {}", config);
            let mut request = ControlTransferRequest::default();
            request.recipient=UsbControlRecipient::Device;
            request.transfer_type=UsbControlTransferType::Vendor;
            request.request=15;

            let data = device.control_transfer_in(
                request,
                30
            ).await.unwrap();


            let version = String::from_utf8(data).unwrap();

            println!("version: {}", version);
        }
        std::thread::sleep(Duration::from_secs(1));
    }


    #[tokio::test]
    async fn test_control_transfer_out() {
        {
            let manager = UsbManager::new().unwrap();
            let mut device = get_hackrf(&manager).await;

            let mut request = ControlTransferRequest::default();
            request.recipient=UsbControlRecipient::Device;
            request.transfer_type=UsbControlTransferType::Vendor;
            request.request=1;
            request.value=1;

            device.control_transfer_out(
                request,
                &[0;0]
            ).await.unwrap();

        }
        std::thread::sleep(Duration::from_secs(1));
    }
}