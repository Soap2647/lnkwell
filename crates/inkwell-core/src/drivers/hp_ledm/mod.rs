//! HP LEDM (Lightweight Embedded Device Management): XML over plain HTTP,
//! served by the printer's embedded web server under `/DevMgmt/`.

mod parse;

use crate::maintenance::{MaintenanceJob, PrinterDriver};
use crate::status::{Consumable, DeviceInfo, PrinterStatus, StatusSource, UsageCounters};
use crate::{Error, Result};
use std::time::Duration;

const INTERNAL_PRINT_NS: &str =
    "http://www.hp.com/schemas/imaging/con/ledm/internalprintdyn/2008/03/21";

pub struct HpLedm {
    base: String,
    agent: ureq::Agent,
}

impl HpLedm {
    pub fn new(host: &str) -> Result<Self> {
        let host = host.trim();
        let plain = !host.is_empty()
            && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        if !plain {
            return Err(Error::InvalidHost(host.to_string()));
        }
        Ok(Self {
            base: format!("http://{host}"),
            agent: ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build(),
        })
    }

    fn get(&self, path: &str) -> Result<String> {
        self.agent
            .get(&format!("{}{path}", self.base))
            .call()
            .map_err(|e| Error::Network(e.to_string()))?
            .into_string()
            .map_err(|e| Error::Network(e.to_string()))
    }
}

/// Request body that asks the printer to run one internal print job.
fn internal_print_request(job_type: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <ipdyn:InternalPrintDyn xmlns:ipdyn=\"{INTERNAL_PRINT_NS}\">\
         <ipdyn:JobType>{job_type}</ipdyn:JobType>\
         </ipdyn:InternalPrintDyn>"
    )
}

impl StatusSource for HpLedm {
    fn device_info(&self) -> Result<DeviceInfo> {
        parse::device_info(&self.get("/DevMgmt/ProductConfigDyn.xml")?)
    }

    fn status(&self) -> Result<PrinterStatus> {
        parse::status(&self.get("/DevMgmt/ProductStatusDyn.xml")?)
    }

    fn consumables(&self) -> Result<Vec<Consumable>> {
        parse::consumables(&self.get("/DevMgmt/ConsumableConfigDyn.xml")?)
    }

    fn usage(&self) -> Result<UsageCounters> {
        parse::usage(&self.get("/DevMgmt/ProductUsageDyn.xml")?)
    }
}

impl PrinterDriver for HpLedm {
    fn name(&self) -> &'static str {
        "HP LEDM"
    }

    fn maintenance_jobs(&self) -> Result<Vec<MaintenanceJob>> {
        parse::maintenance_jobs(&self.get("/DevMgmt/InternalPrintCap.xml")?)
    }

    // NOTE: not yet verified against a real device. The endpoint and job type
    // names come from the printer's own InternalPrintCap.xml; the body shape
    // follows the LEDM InternalPrintDyn schema.
    fn run_maintenance(&self, job_id: &str) -> Result<()> {
        // Only send job types the device itself advertises.
        if !self.maintenance_jobs()?.iter().any(|j| j.id == job_id) {
            return Err(Error::Unsupported(format!("maintenance job {job_id}")));
        }
        self.agent
            .post(&format!("{}/DevMgmt/InternalPrintDyn.xml", self.base))
            .set("Content-Type", "text/xml")
            .send_string(&internal_print_request(job_id))
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_hosts_that_are_not_plain_names() {
        assert!(HpLedm::new("192.0.2.10").is_ok());
        assert!(HpLedm::new("printer.local").is_ok());
        assert!(HpLedm::new("").is_err());
        assert!(HpLedm::new("evil.example/path").is_err());
        assert!(HpLedm::new("user@host").is_err());
    }

    #[test]
    fn internal_print_request_is_well_formed() {
        let body = internal_print_request("cleaningPage");
        let doc = roxmltree::Document::parse(&body).unwrap();
        let job = doc.root_element().first_element_child().unwrap();
        assert_eq!(job.tag_name().namespace(), Some(INTERNAL_PRINT_NS));
        assert_eq!(job.tag_name().name(), "JobType");
        assert_eq!(job.text(), Some("cleaningPage"));
    }
}
