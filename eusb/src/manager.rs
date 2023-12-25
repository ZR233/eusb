use std::sync::OnceLock;
use crate::platform::{DeviceCtx, Platform, PlatformLibUsb};


#[cfg(libusb)]
type  PlatformImpl =  PlatformLibUsb;


static  MANAGER: OnceLock<Manager> = OnceLock::new();

struct Manager{
    platform: PlatformImpl
}
impl Manager{
    pub(crate) fn get()->&'static Self{
        MANAGER.get_or_init(||{
            let platform = PlatformImpl::new();
            Self{platform}
        })
    }
}


#[cfg(test)]
mod tests{
    #[test]
    fn it_works(){



    }
}








