pub use crate::manager::*;
pub use crate::define::*;
pub use crate::device::*;
pub use crate::platform::*;

#[cfg(test)]
mod test{
    use std::time::{Duration, Instant};
    use log::{debug, info, LevelFilter};
    use tokio::select;
    use crate::adaptor::{IConfig, IInterface, IRequest};
    use super::*;
    fn init() {
        let _ = env_logger::builder().filter_level(LevelFilter::Trace).is_test(true).try_init();
    }
    async fn get_hackrf(manager: &UsbManager) -> Device{
        manager.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap()
    }

    #[tokio::test]
    async fn it_works(){
        init();

        {
            let m = UsbManager::init_default().unwrap();
            let d = m.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap();
            let sn = d.serial_number().await;
            // let il = d.interface_list().await.unwrap();
            debug!("sn: {}, {}-{}", sn, d.pid(), d.vid());

            let configs = d.configs();
            for cfg in &configs{
                debug!("extra: {:?}", cfg.extra());
            }

            debug!("configs: {}", configs.len());

        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        debug!("finish");
    }
    #[tokio::test]
    async fn control_in(){
        init();

        {
            let m = UsbManager::init_default().unwrap();
            let device = get_hackrf(&m).await;
            let mut data = device.control_transfer_in(
                UsbControlRecipient::Device,
                UsbControlTransferType::Vendor,
                15,
                0,0,Duration::default(), 30
            ).await.unwrap();
            let data = data.data().to_vec();
            let version = String::from_utf8(data).unwrap();
            debug!("version: {}", version);
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        debug!("finish");
    }

    #[tokio::test]
    async fn test_bulk_transfer_in_channel() {
        init();

        {
            let manager = UsbManager::init_default().unwrap();
            let device = get_hackrf(&manager).await;

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

            let interface = device.get_interface(0).unwrap();
            interface.claim().unwrap();

            let bulk1 = device.bulk_request(Endpoint::In {num: 1}, 262144, Duration::default()).unwrap();
            let bulk2 = device.bulk_request(Endpoint::In {num: 1}, 262144, Duration::default()).unwrap();
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
}