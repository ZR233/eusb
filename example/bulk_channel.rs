use std::time::Duration;
use log::{info, LevelFilter};
use tokio::select;
use tokio::time::Instant;
use eusb::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main(){
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    {
        let manager = UsbManager::init_default().unwrap();
        let device = manager.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap();
        device.control_transfer_out(
            UsbControlRecipient::Device,
            UsbControlTransferType::Vendor,
            1,0,0, Duration::default(),
            &mut [0; 0],
        ).await.unwrap();

        device.control_transfer_out(
            UsbControlRecipient::Device,
            UsbControlTransferType::Vendor,
            1,1,0, Duration::default(),
            &mut [0; 0],
        ).await.unwrap();

        let interface = device.claim_interface_by_num(0).unwrap();

        let bulk1 = interface.bulk_request(EndpointDescriptor::new(1, Direction::In), 262144, Duration::default()).unwrap();
        let bulk2 = interface.bulk_request(EndpointDescriptor::new(1, Direction::In), 262144, Duration::default()).unwrap();
        let (mut tx, mut rx) = device.request_channel(10);
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
        tx.send(bulk1).unwrap();
        tx.send(bulk2).unwrap();

        tokio::spawn(async move {
            let mut all = 0usize;
            let start = Instant::now();
            loop {
                select! {
                    res = rx.next() => {
                        match res{
                            Some(r)=> {
                                let mut result = r.unwrap();
                                 all += result.data().len();
                                 let r = tx.send(result);
                                    if r.is_err(){
                                        break;
                                    }
                            }
                            None=>break,
                        }
                    }
                    _ = (&mut stop_rx) => {
                        break;
                    }
                }
            }
            let duration = Instant::now().duration_since(start);
            let bits = (all) as f64;
            let seconds = duration.as_secs_f64();
            let mb = (bits / seconds) / 1_000_000.0;

            info!("速度：{} MB/s", mb);
            info!("接收停止");
        });


        tokio::time::sleep(Duration::from_secs(5)).await;
        // device.control_transfer_out(
        //     UsbControlRecipient::Device,
        //     UsbControlTransferType::Vendor,
        //     1,0,0, Duration::default(),
        //     &mut [0; 0],
        // ).await.unwrap();

        // let start = Instant::now();
        //
        // device.control_transfer_out(
        //     request,
        //     &[0; 0],
        // ).await.unwrap();
        // info!("send off cost: {:?}", start.elapsed());
        // tokio::time::sleep(Duration::from_secs(5)).await;
        info!("send stop");
        stop_tx.send(1).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
    info!("finish");

}