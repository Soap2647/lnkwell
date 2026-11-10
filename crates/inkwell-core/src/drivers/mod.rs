//! Vendor drivers. One module per protocol family.

pub mod hp_ledm;

use crate::maintenance::PrinterDriver;
use crate::Result;

/// Picks a driver for the device at `host`.
/// Only HP LEDM exists so far, so there is nothing to detect yet.
pub fn for_host(host: &str) -> Result<Box<dyn PrinterDriver>> {
    Ok(Box::new(hp_ledm::HpLedm::new(host)?))
}
