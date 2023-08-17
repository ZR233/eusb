use futures::channel::mpsc::*;
use log::warn;
use libusb_src::*;
use crate::platform::{Request};
use crate::error::*;
use futures::StreamExt;


pub struct RequestSender{
    tx: Sender<Result<Request>>
}

impl  RequestSender {
pub  fn send(&mut self, request: Request)->Result<()>{
        let tx = self.tx.clone();

        unsafe {
            let ptr = request.ptr.0;
            (*ptr).callback = complete_cb;
            let user_data = Box::new(UserData{
                tx,
                request: Some(request)
            });
            let user_data_ptr = Box::into_raw(user_data);
            (*ptr).user_data = user_data_ptr as _;

            let r = libusb_submit_transfer(ptr);
            check_err(r)?;
        }
        Ok(())
    }
}


pub struct RequestReceiver{
    rx: Receiver<Result<Request>>
}

impl RequestReceiver {
    pub async fn next(&mut self)->Option<Result<Request>>{
        self.rx.next().await
    }
}


pub(crate) fn request_channel(buffer: usize)->(RequestSender, RequestReceiver){
    let (tx, rx) = channel(buffer);
    let sender = RequestSender{
        tx,
    };
    let rcv = RequestReceiver{
      rx
    };
    (sender, rcv)
}
struct UserData{
    tx: Sender<Result<Request>>,
    request: Option<Request>,
}


extern "system"  fn complete_cb(data: *mut libusb_transfer){
    unsafe {
        let user_data_ptr = (*data).user_data;

        let mut user_data = Box::from_raw(user_data_ptr as  *mut UserData);

        let result = match (*data).status {
            LIBUSB_TRANSFER_COMPLETED => {
                Ok(user_data.request.take().unwrap())
            },
            LIBUSB_TRANSFER_OVERFLOW => Err(Error::Overflow),
            LIBUSB_TRANSFER_TIMED_OUT => Err(Error::Timeout),
            LIBUSB_TRANSFER_CANCELLED => Err(Error::Cancelled),
            LIBUSB_TRANSFER_STALL => Err(Error::NotSupported),
            LIBUSB_TRANSFER_NO_DEVICE => Err(Error::NoDevice),
            LIBUSB_TRANSFER_ERROR |_ => Err(Error::Other("Unknown".to_string())),
        };
        match user_data.tx.try_send(result){
            Ok(_) => {}
            Err(e) => {
                warn!("Data rcv too slow!");
            }
        };
    }
}