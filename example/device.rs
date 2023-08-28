use log::{info, LevelFilter};
use eusb::prelude::*;

#[tokio::main]
async fn main() {
    let _ = env_logger::builder().filter_level(LevelFilter::Debug).is_test(true).try_init();

    let manager = UsbManager::init_default().unwrap();
    let devices = manager.device_list().await.unwrap();
    for device in devices {
        let mut msg = "".to_string();


        let sn = match device.serial_number().await {
            Ok(s) => { s }
            Err(_) => { "没有权限，无法获取部分信息".to_string() }
        };
        msg = format!("Device: pid: {} vid: {} sn: {}", device.pid(), device.vid(), sn);
        let cfg_list = device.config_list().unwrap();
        for cfg in &cfg_list {
            msg += format!(r"
  Configuration [{}]:
    value {}
           ", cfg.configuration, cfg.value).as_str();

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


    // let device = manager.open_device_with_vid_pid(0x1d50, 0x6089)..unwrap();
    // let c = device.get_configuration().unwrap();
    // debug!("config: {}",c);
    // device.set_configuration(1).unwrap();
    //
    // debug!("all finish");
}