use std::ffi::c_int;
use crate::error::*;
use libusb_src::*;


pub(crate) fn check_err(r: c_int) -> Result<i32> {
    if r >= 0 { Ok(r as _) } else {
        let e = match r {
            LIBUSB_ERROR_IO            =>Error::Io("Usb".to_string()),
            LIBUSB_ERROR_INVALID_PARAM =>Error::InvalidParam,
            LIBUSB_ERROR_ACCESS        =>Error::Access,
            LIBUSB_ERROR_NO_DEVICE     =>Error::NoDevice,
            LIBUSB_ERROR_NOT_FOUND     =>Error::NotFound,
            LIBUSB_ERROR_BUSY          =>Error::Busy,
            LIBUSB_ERROR_TIMEOUT       =>Error::Timeout,
            LIBUSB_ERROR_OVERFLOW      =>Error::Overflow,
            LIBUSB_ERROR_PIPE          =>Error::Pipe,
            LIBUSB_ERROR_INTERRUPTED   =>Error::Interrupted,
            LIBUSB_ERROR_NO_MEM        =>Error::NoMem,
            LIBUSB_ERROR_NOT_SUPPORTED =>Error::NotSupported,
            _ => Error::Other("Libusb Unknown".to_string())
        };

        Err(e)
    }
}
