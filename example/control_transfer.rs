use std::time::Duration;
use log::{debug, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let manager = UsbManager::init_default().unwrap();
        let mut device = manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap();

        println!("{} speed: {:?}", device, device.speed());

        // device.set_configuration(0x1).unwrap();
        let config = device.get_configuration().unwrap();

        println!("config: {}", config);
        let mut request = ControlTransferRequest::default();
        request.recipient = UsbControlRecipient::Device;
        request.transfer_type = UsbControlTransferType::Vendor;
        request.request = 15;

        let start = Instant::now();

        let data = device.control_transfer_in(
            request,
            30,
        ).await.unwrap();
        let duration = start.elapsed();

        let version = String::from_utf8(data).unwrap();

        println!("version: {} cost: {:?}", version, duration);


    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    debug!("all finish");

}