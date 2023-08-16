use crate::error::*;
use std::sync::{Arc};
use crate::define::{CtxManager, IManager, ResultFuture};
use crate::device::UsbDevice;

#[cfg(libusb)]
use crate::platform::libusb::*;

#[derive(Clone)]
pub struct UsbManager{
    ctx: Arc<Manager>
}


impl UsbManager {
    pub fn new()->Result<Self>{
        let ctx = Manager::new()?;
        let ctx = Arc::new(ctx);
        Ok(Self{
            ctx
        })
    }

    pub fn device_list(&self)->ResultFuture<Vec<UsbDevice>>{
        let ctx = self.ctx.clone();
        Box::pin(async move{
            let mut l = ctx.clone().device_list().await?;
            let mut out = vec![];
            while let Some(one) = l.pop()  {
                #[cfg(libusb)]
                let dev = UsbDevice::new(one, ctx.clone());
                out.push(dev);
            }
            Ok(out)
        })
    }
}


#[cfg(test)]
mod test{
    use super::*;

    #[tokio::test]
    async fn it_works(){
        let m = UsbManager::new().unwrap();
        let dl = m.device_list().await.unwrap();
        let d = dl[0].clone();
        // let il = d.interface_list().await.unwrap();
        println!("{}", dl.len());


    }

}