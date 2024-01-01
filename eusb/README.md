![Build](https://github.com/ZR233/eusb/workflows/BuildUbuntu/badge.svg)
![Build](https://github.com/ZR233/eusb/workflows/BuildWindows/badge.svg)
![Build](https://github.com/ZR233/eusb/workflows/BuildMacos/badge.svg)
![Build](https://github.com/ZR233/eusb/workflows/BuildAndroid/badge.svg)

# EUsb

The `eusb` crate provides easy way to communicate usb, with async fn.

# example
Test with device hackrf one.

```rust
use log::*;
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Info).is_test(true).try_init();

    let devices = UsbDevice::list().unwrap();
    for device in devices {
        let mut product = "".to_string();
        let mut manufacturer = "".to_string();

        if let Ok(s) = device.product() {product=s};
        if let Ok(s) = device.manufacturer() {manufacturer=s};

        let sn = match device.serial_number() {
            Ok(s) => { s }
            Err(_) => { "没有权限，无法获取部分信息".to_string() }
        };

        let bcd_usb = device.bcd_usb_version().unwrap();
        let bcd_device = device.bcd_device_version().unwrap();
        let des = device.device_descriptor().unwrap();
        let mut msg = format!(r"
Device:
  pid: 0x{:04X}
  vid: 0x{:04X}
  sn: {}
  bcd usb: {}.{}
  bcd device: {}.{}
  class: {:?}
  subclass: {:?}
  protocol: {:?}
  manufacturer: {}
  product: {}
",
                              des.idProduct,
                              des.idVendor,
                              sn,
                              bcd_usb[1],bcd_usb[2],
                              bcd_device[1],bcd_device[2],
                              device.device_class().unwrap(),
                              device.device_subclass().unwrap(),
                              device.device_protocol().unwrap(),
                              manufacturer, product);
        let cfg_list = device.config_list().unwrap();
        for cfg in &cfg_list {
            msg += format!(r"
  Configuration [{}]:
    Value {}
    MaxPower {} mA
    Extra {:?}
           ", cfg.configuration, cfg.value, cfg.max_power,cfg.extra).as_str();

            for alts in &cfg.interfaces {
                let interface = &alts.alt_settings[0];

                msg += format!(r"
    Interface [{}]:
      Alternate Setting {}
      Class: {:?}
      Subclass: {:?}
      Protocol {:?}
      Extra: {:?}
                ",
                               interface.interface,
                               interface.alt_setting,
                               interface.device_class,
                               interface.device_sub_class,
                               interface.protocol,
                               interface.extra
                ).as_str();


                for endpoint in &interface.endpoints {
                    msg += format!(r"
      Endpoint [{}]:
        Direction {:?}
        Transfer Type: {:?}
        Usage Type: {:?}
        Sync Type {:?}
        Extra: {:?}
                ",
                                   endpoint.num,
                                   endpoint.direction,
                                   endpoint.transfer_type,
                                   endpoint.usage_type,
                                   endpoint.sync_type,
                                   endpoint.extra
                    ).as_str();
                }
            }
        }


        info!("{}", msg)
    }
}


```