use std::time::Duration;
use log::{debug, info, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let device = UsbDevice::open_with_vid_pid(0x1D50, 0x6089).unwrap();

        info!("{}", device);
        // if let Ok(sn) = device.serial_number() {
        //     info!("sn: {sn}");
        // }
        //
        // let cfg = device.get_active_configuration().unwrap();
        //
        // info!("cfg: {}", cfg.configuration);

        let start = Instant::now();

        let data = device.control_transfer_in(
            ControlTransferRequest{
                recipient: UsbControlRecipient::Device,
                transfer_type: UsbControlTransferType::Vendor,
                request: 15,
                ..Default::default()
            }
            ,30
        ).await.unwrap();


        let duration = start.elapsed();
        let version = String::from_utf8(data).unwrap();

        info!("version: {} cost: {:?}", version, duration);

    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    debug!("all finish");

}