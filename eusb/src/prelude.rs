pub use crate::manager::*;
pub use crate::define::*;
pub use crate::device::*;


#[cfg(test)]
mod test{
    use std::time::Duration;
    use log::{debug, LevelFilter};
    use crate::adaptor::IRequest;
    use super::*;
    fn init() {
        let _ = env_logger::builder().filter_level(LevelFilter::Trace).is_test(true).try_init();
    }

    #[tokio::test]
    async fn it_works(){
        init();

        {
            let m = UsbManager::new().unwrap();
            let d = m.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap();
            let sn = d.serial_number().await;
            // let il = d.interface_list().await.unwrap();
            debug!("sn: {}, {}-{}", sn, d.pid(), d.vid());
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        debug!("finish");
    }
    #[tokio::test]
    async fn control_in(){
        init();

        {
            let m = UsbManager::new().unwrap();
            let device = m.open_device_with_vid_pid(0x1d50, 0x6089).await.unwrap();
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
}