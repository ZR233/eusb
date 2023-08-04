use std::time::Duration;
use log::{debug, LevelFilter};
use tokio::time::Instant;
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();

    let manager = UsbManager::init_default().unwrap();
    let device = manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap();
    let c = device.get_configuration().unwrap();
    debug!("config: {}",c);
    device.set_configuration(1).unwrap();

    debug!("all finish");

}