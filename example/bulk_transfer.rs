use std::time::Duration;
use log::{info, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;


#[tokio::main]
async fn main(){
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let device =UsbDevice::open_with_vid_pid(0x1d50, 0x6089).unwrap();


        if let Ok(sn) = device.serial_number() {
            info!("sn: {sn}");
        }
        device.control_transfer_out(ControlTransferRequest{
            recipient: UsbControlRecipient::Device,
            transfer_type: UsbControlTransferType::Vendor,
            request: 1,
            .. Default::default()
        }, &[]).await.unwrap();
        info!("mode off");

        device.control_transfer_out(ControlTransferRequest{
            recipient: UsbControlRecipient::Device,
            transfer_type: UsbControlTransferType::Vendor,
            request: 1,
            value: 1,
            .. Default::default()
        }, &[]).await.unwrap();

        info!("mode on");
        let mut all = 0usize;
        let start = Instant::now();
        info!("开始");

        for _ in 0..10{
            let data = device.bulk_transfer_in(1, 262144* 6, Duration::default()).await.unwrap();
            all += data.len();
        }


        let duration = Instant::now().duration_since(start);
        let bits = (all) as f64;
        let seconds = duration.as_secs_f64();
        let mb = (bits / seconds) / 1_000_000.0;

        info!("速度：{} MB/s", mb);
        info!("接收停止");
    }


    tokio::time::sleep(Duration::from_secs(1)).await;
    info!("finish");

}