# EUsb

The `eusb` crate provides easy way to communicate usb, with async fn.

tested on ubuntu and windows

# platform

|                                 Ubuntu                                  |                                 Windows                                  |                                 macOS                                  |                                 android                                  |
|:-----------------------------------------------------------------------:|:------------------------------------------------------------------------:|:----------------------------------------------------------------------:|:------------------------------------------------------------------------:|
| ![Build](https://github.com/ZR233/eusb/workflows/BuildUbuntu/badge.svg) | ![Build](https://github.com/ZR233/eusb/workflows/BuildWindows/badge.svg) | ![Build](https://github.com/ZR233/eusb/workflows/BuildMacos/badge.svg) | ![Build](https://github.com/ZR233/eusb/workflows/BuildAndroid/badge.svg) |


# example
test use hackrf one.

```rust
use rusb::prelude::*;

#[tokio::main]
async fn main(){
    let manager = UsbManager::new().unwrap();
    let mut device = manager.open_device_with_vid_pid(0x1d50, 0x6089).unwrap();
    println!("sn: {}", device.serial_number());
    let device_descriptor = device.descriptor();
    println!("{:?}", device_descriptor);
    let configs = device.config_list().unwrap();
    for config in configs {
        println!("{}", config);
    }
    let mut request = ControlTransferRequest::default();
    request.recipient = UsbControlRecipient::Device;
    request.transfer_type = UsbControlTransferType::Vendor;
    request.request = 15;

    let start = Instant::now();

    let data = device.control_transfer_in(
        request,
        30,
    ).await.unwrap();
    let duration = start.elapsed();

    let version = String::from_utf8(data).unwrap();

    println!("version: {} cost: {:?}", version, duration);

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

    println!("speed：{} MB/s", mb);
}

```

# Cross Compile

support windows linux and android, not test ios and mac.