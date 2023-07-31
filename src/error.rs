use libusb_src::constants::*;
use std::{result, fmt};
use std::ffi::c_int;

pub type Result<T> = result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Input/output error.")]
    Io,

    #[error("Invalid parameter")]
    InvalidParam,

    #[error("Access denied (insufficient permissions)")]
    Access,

    #[error("No such device (it may have been disconnected)")]
    NoDevice,

    #[error("Entity not found")]
    NotFound,

    #[error("Resource busy ")]
    Busy,

    #[error("Operation timed out")]
    Timeout,

    #[error("Overflow")]
    Overflow,

    #[error("Pipe error")]
    Pipe,

    #[error("System call interrupted (perhaps due to signal)")]
    Interrupted,

    #[error("Insufficient memory")]
    NoMem,

    #[error("Operation not supported or unimplemented on this platform")]
    NotSupported,

    #[error("**UNKNOWN**")]
    Other,
}

pub(crate) fn check_err(r: c_int) -> Result<()> {
    if r >= 0 { Ok(()) } else {
        let e = match r {
            LIBUSB_ERROR_IO            => Error::Io,
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
            _ => Error::Other
        };

        Err(e)
    }
}