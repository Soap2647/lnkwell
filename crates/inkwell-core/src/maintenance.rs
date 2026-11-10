//! Vendor-specific maintenance behind a driver trait.
//!
//! A driver reports which jobs the connected device supports; the UI shows
//! only those. Nothing here is assumed to exist on every printer.

use crate::status::StatusSource;
use crate::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JobKind {
    /// Printhead cleaning. Higher levels use more ink.
    Cleaning { level: u8 },
    /// Print-quality diagnostic page.
    QualityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceJob {
    /// Driver-specific identifier, passed back to `run_maintenance`.
    pub id: String,
    pub kind: JobKind,
}

pub trait PrinterDriver: StatusSource {
    fn name(&self) -> &'static str;

    /// Jobs this particular device says it supports.
    fn maintenance_jobs(&self) -> Result<Vec<MaintenanceJob>>;

    /// Starts a job. Uses ink and paper; callers must get user consent first.
    fn run_maintenance(&self, job_id: &str) -> Result<()>;
}
