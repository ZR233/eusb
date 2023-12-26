pub use crate::device::UsbDevice;


#[cfg(test)]
mod tests{
    use super::*;

    #[tokio::test]
    async  fn it_works(){
        let devices = UsbDevice::list().unwrap();

        for device in &devices {
            println!("{}", device);
            if let Ok(sn)= device.serial_number() {
                println!("sn: {sn}");
            }

        }
    }
}
