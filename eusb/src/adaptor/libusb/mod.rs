pub(crate) mod interface;
pub(crate) mod ptr;
mod manager;
mod device;
mod channel;
mod transfer;
pub use transfer::Request;
pub(crate) use manager::Manager;
pub(crate) use device::CtxDeviceImpl;
use interface::CtxInterfaceImpl;
pub use channel::RequestReceiver;
pub use channel::RequestSender;


pub type Interface = CtxInterfaceImpl;