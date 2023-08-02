pub use crate::core::*;
pub use crate::define::*;
pub use crate::device::*;

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use log::{debug, info, LevelFilter};
    use tokio::select;
    use crate::interface::{BulkChannelOption, BulkTransferRequest};
    use super::*;
    use futures::StreamExt;

    fn init() {
        let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();
    }

    async fn get_hackrf(manager: &UsbManager) -> Arc<Device> {
        manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap()
    }
    #[tokio::test]
    async fn test_device() {
        init();
        let manager = UsbManager::new().unwrap();
        let device = get_hackrf(&manager).await;
        debug!("sn: {}", device.serial_number());
        let device_descriptor = device.descriptor();

        let configs = device.config_list().unwrap();
        for config in configs {
            debug!("{}", config);
        }
        debug!("finish");
    }
    #[tokio::test]
    async fn test_control_transfer_in() {
        init();
        {
            let manager = UsbManager::new().unwrap();
            let device = get_hackrf(&manager).await;

            println!("{} speed: {:?}", device, device.speed());

            // device.set_configuration(0x1).unwrap();
            let config = device.get_configuration().unwrap();

            println!("config: {}", config);
            let mut request = ControlTransferRequest::default();
            request.recipient = UsbControlRecipient::Device;
            request.transfer_type = UsbControlTransferType::Vendor;
            request.request = 15;

            let data = device.control_transfer_in(
                request,
                30,
            ).await.unwrap();


            let version = String::from_utf8(data).unwrap();

            println!("version: {}", version);


        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        debug!("all finish");
    }


    #[tokio::test]
    async fn test_control_transfer_out() {
        {
            let manager = UsbManager::new().unwrap();
            let device = get_hackrf(&manager).await;

            let mut request = ControlTransferRequest::default();
            request.recipient = UsbControlRecipient::Device;
            request.transfer_type = UsbControlTransferType::Vendor;
            request.request = 1;
            request.value = 1;

            device.control_transfer_out(
                request,
                &[0; 0],
            ).await.unwrap();
        }
        std::thread::sleep(Duration::from_secs(1));
    }


    #[tokio::test]
    async fn test_bulk_transfer_in_channel() {
        init();

        {
            let manager = UsbManager::new().unwrap();
            let device = get_hackrf(&manager).await;
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

            let interface = device.get_interface(0).unwrap();
            let mut option = BulkChannelOption::default();
            option.request_size=2;

            let mut rx = interface.open_bulk_in_channel(BulkTransferRequest {
                endpoint: 1,
                package_len: 262144,
                timeout: Default::default(),
            }, BulkChannelOption::default()).unwrap();

            let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();

            tokio::spawn(async move {
                let mut all = 0usize;
                let start = Instant::now();
                loop {
                    select! {
                    res = rx.next() => {
                        match res{
                            Some(data)=> {
                                 all += data.len();
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
            let mut request = ControlTransferRequest::default();
            request.recipient = UsbControlRecipient::Device;
            request.transfer_type = UsbControlTransferType::Vendor;
            request.request = 1;
            request.value = 0;

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



    #[tokio::test]
    async fn test_bulk_transfer_in() {
        init();
        {
            let manager = UsbManager::new().unwrap();
            let device = get_hackrf(&manager).await;
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

            let interface = device.get_interface(0).unwrap();

            let mut all = 0usize;
            let start = Instant::now();
            for _ in 0..1000 {
                let data = interface.bulk_transfer_in(BulkTransferRequest{
                    endpoint: 1,
                    package_len: 262144,
                    timeout: Default::default(),
                }).await.unwrap();
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
}