pub use crate::device::UsbDevice;
pub use crate::endpoint::EndpointIn;
pub use crate::define::*;

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use log::*;
    use tokio::time::Instant;
    use super::*;
    use crate::utils::test::init;

    #[tokio::test]
    async fn it_works() {
        init();

        let devices = UsbDevice::list().unwrap();

        for device in &devices {
            info!("{}", device);
            if let Ok(sn) = device.serial_number() {
                info!("sn: {sn}");
            }

            let cfg = device.get_active_configuration().unwrap();

            info!("cfg: {}", cfg.configuration);
        }
    }


    #[tokio::test]
    async fn test_hackrf() {
        init();

        let device = UsbDevice::open_with_vid_pid(0x1D50, 0x6089).unwrap();

        info!("{}", device);
        if let Ok(sn) = device.serial_number() {
            info!("sn: {sn}");
        }

        let cfg = device.get_active_configuration().unwrap();

        info!("cfg: {}", cfg.configuration);

        let start = Instant::now();

        let data = device.control_transfer_in(
            UsbControlRecipient::Device,
            UsbControlTransferType::Vendor,
            15,
            0,0,Duration::default(), 30
        ).await.unwrap();
        let duration = start.elapsed();
        let version = String::from_utf8(data).unwrap();

        println!("version: {} cost: {:?}", version, duration);
    }
}
