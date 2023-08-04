use std::time::Duration;
use log::{info, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;


#[tokio::main]
async fn main(){
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let manager = UsbManager::init_default().unwrap();
        let device = manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap();

        let mut request = ControlTransferRequest::default();
        request.recipient = UsbControlRecipient::Device;
        request.transfer_type = UsbControlTransferType::Vendor;
        request.request = 1;
        request.value = 0;

        device.control_transfer_out(
            request,
            &[0; 0],
        ).await.unwrap();

        let mut request = ControlTransferRequest::default();
        request.recipient = UsbControlRecipient::Device;
        request.transfer_type = UsbControlTransferType::Vendor;
        request.request = 1;
        request.value = 1;

        device.control_transfer_out(
            request,
            &[0; 0],
        ).await.unwrap();

        let mut interface = device.get_interface(0).unwrap();

        let mut all = 0usize;
        let start = Instant::now();
        info!("开始");
        let mut transfer = interface.bulk_transfer_in_request(BulkTransferRequest{
            endpoint: 1,
            package_len: 262144,
            timeout: Default::default(),
        }).unwrap();

        for _ in 0..1000 {
            transfer = transfer.submit().unwrap().await.unwrap();
            all += transfer.data().len();
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