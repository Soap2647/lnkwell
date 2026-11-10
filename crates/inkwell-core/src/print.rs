//! Printing. Interface only in this draft; nothing is sent to the printer yet.
//!
//! Plan: IPP Everywhere (PWG Raster / URF) when the device supports it,
//! otherwise hand the job to the operating system's print service.

use crate::{Error, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Duplex {
    OneSided,
    /// Flip on the long edge, like turning a page in a book.
    FlipLikeBook,
    /// Flip on the short edge, like a wall calendar.
    FlipLikeCalendar,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    Draft,
    Normal,
    Best,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSettings {
    pub copies: u16,
    pub color: bool,
    pub quality: Quality,
    pub duplex: Duplex,
    /// PWG media name, e.g. "iso_a4_210x297mm".
    pub media: String,
}

pub fn submit(_host: &str, _settings: &PrintSettings) -> Result<()> {
    Err(Error::Unsupported("printing is not implemented in this draft".into()))
}
