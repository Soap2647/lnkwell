//! Device state, alerts, consumables and counters.

use crate::Result;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub model: String,
    pub firmware: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    /// Vendor alert identifier, passed through untranslated.
    pub id: String,
    pub severity: Severity,
    /// Ink colour the alert refers to, if any (e.g. "Cyan").
    pub color: Option<String>,
    /// Localised text supplied by the printer itself, if any.
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterStatus {
    /// Vendor state categories, passed through untranslated (e.g. "ready", "inPowerSave").
    pub states: Vec<String>,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsumableKind {
    Printhead,
    Tank,
    Cartridge,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Consumable {
    pub kind: ConsumableKind,
    /// Colour label as reported by the printer: "C", "M", "Y", "K", "CMY", ...
    pub label: String,
    pub part_number: Option<String>,
    pub level_percent: Option<u8>,
    /// False when the device reports a level it does not actually measure.
    /// The UI must not present such a value as a reading.
    pub level_measured: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCounters {
    pub total_impressions: Option<u32>,
    pub mono_impressions: Option<u32>,
    pub color_impressions: Option<u32>,
    pub jam_events: Option<u32>,
    pub mispick_events: Option<u32>,
}

/// Read-only view of a printer. Generic sources (IPP, SNMP) and vendor
/// drivers can both implement this.
pub trait StatusSource: Send + Sync {
    fn device_info(&self) -> Result<DeviceInfo>;
    fn status(&self) -> Result<PrinterStatus>;
    fn consumables(&self) -> Result<Vec<Consumable>>;
    fn usage(&self) -> Result<UsageCounters>;
}
