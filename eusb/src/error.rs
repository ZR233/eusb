use std::{result};

pub type Result<T=()> = result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Input/output error: {0}")]
    Io(String),

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

    #[error("Cancelled")]
    Cancelled,

    #[error("[USB] Something wrong: {0}")]
    Other(String),
}
