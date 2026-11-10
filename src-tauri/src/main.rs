#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use inkwell_core::drivers;
use inkwell_core::maintenance::MaintenanceJob;
use inkwell_core::print::{self, PrintSettings};
use inkwell_core::status::{Consumable, DeviceInfo, PrinterStatus, UsageCounters};
use serde::Serialize;

/// Everything the printer panel shows, fetched in one go.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    driver: &'static str,
    device: DeviceInfo,
    status: PrinterStatus,
    consumables: Vec<Consumable>,
    usage: UsageCounters,
    maintenance_jobs: Vec<MaintenanceJob>,
}

/// Drivers do blocking network I/O; keep it off the async runtime's threads.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> inkwell_core::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn printer_snapshot(host: String) -> Result<Snapshot, String> {
    blocking(move || {
        let driver = drivers::for_host(&host)?;
        Ok(Snapshot {
            driver: driver.name(),
            device: driver.device_info()?,
            status: driver.status()?,
            consumables: driver.consumables()?,
            usage: driver.usage()?,
            maintenance_jobs: driver.maintenance_jobs()?,
        })
    })
    .await
}

#[tauri::command]
async fn run_maintenance(host: String, job_id: String) -> Result<(), String> {
    blocking(move || drivers::for_host(&host)?.run_maintenance(&job_id)).await
}

#[tauri::command]
fn print_document(host: String, settings: PrintSettings) -> Result<(), String> {
    print::submit(&host, &settings).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![printer_snapshot, run_maintenance, print_document])
        .run(tauri::generate_context!())
        .expect("error while running Inkwell");
}
