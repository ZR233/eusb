use std::time::Duration;
use log::{debug, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let manager = UsbManager::init_default().unwrap();
        let device = manager.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap();

        let start = Instant::now();

        let mut data = device.control_transfer_in(
            UsbControlRecipient::Device,
            UsbControlTransferType::Vendor,
            15,
            0,0,Duration::default(), 30
        ).await.unwrap();
        let data = data.data().to_vec();
        let duration = start.elapsed();

        let version = String::from_utf8(data).unwrap();

        println!("version: {} cost: {:?}", version, duration);
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    debug!("all finish");

}