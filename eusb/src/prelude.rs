pub use crate::device::UsbDevice;


#[cfg(test)]
mod tests {
    use log::*;
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
    }
}
