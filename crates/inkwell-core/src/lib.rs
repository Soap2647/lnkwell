//! Vendor-neutral printer core.
//!
//! Three boundaries, each in its own module:
//! - [`print`]: sending documents (IPP Everywhere first, OS spooler as fallback)
//! - [`status`]: device state, alerts, consumables, counters
//! - [`maintenance`]: vendor-specific jobs behind the [`maintenance::PrinterDriver`] trait
//!
//! Vendor code lives only under [`drivers`].

pub mod drivers;
pub mod maintenance;
pub mod print;
pub mod status;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// The host string is not a plain IP address or hostname.
    InvalidHost(String),
    Network(String),
    Parse(String),
    /// The printer or this build does not support the requested operation.
    Unsupported(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidHost(h) => write!(f, "invalid host: {h}"),
            Error::Network(m) => write!(f, "network error: {m}"),
            Error::Parse(m) => write!(f, "unexpected printer response: {m}"),
            Error::Unsupported(m) => write!(f, "not supported: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
