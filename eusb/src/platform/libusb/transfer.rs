use libusb_src::libusb_transfer;

pub(crate) struct Transfer(pub(crate) *mut libusb_transfer);
unsafe impl Sync for Transfer {}
unsafe impl Send for Transfer {}
