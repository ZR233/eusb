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

pub fn check_err(r: c_int) -> Result<()> {
    if r >= 0 { Ok(()) } else {
        let e = match r {
            LIBUSB_ERROR_IO => Error::Io,
            _ => Error::Other
        };

        Err(e)
    }
}