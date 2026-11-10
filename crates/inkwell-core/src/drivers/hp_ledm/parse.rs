//! LEDM XML parsing. Pure functions over response bodies, so they can be
//! tested against recorded responses without a printer.

use crate::maintenance::{JobKind, MaintenanceJob};
use crate::status::{
    Alert, Consumable, ConsumableKind, DeviceInfo, PrinterStatus, Severity, UsageCounters,
};
use crate::{Error, Result};
use roxmltree::{Document, Node};
use std::collections::HashMap;

fn parse(xml: &str) -> Result<Document<'_>> {
    Document::parse(xml).map_err(|e| Error::Parse(e.to_string()))
}

fn named<'a, 'i>(node: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> {
    node.descendants().filter(move |n| n.tag_name().name() == name)
}

/// Trimmed, non-empty text of the first descendant with this local name.
fn text(node: Node, name: &str) -> Option<String> {
    let t = named(node, name).next()?.text()?.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Like [`text`], but only looks at direct children.
fn child_number(node: Node, name: &str) -> Option<u32> {
    node.children().find(|n| n.tag_name().name() == name)?.text()?.trim().parse().ok()
}

pub fn device_info(xml: &str) -> Result<DeviceInfo> {
    let doc = parse(xml)?;
    let info = named(doc.root(), "ProductInformation")
        .next()
        .ok_or_else(|| Error::Parse("no ProductInformation".into()))?;
    Ok(DeviceInfo {
        model: text(info, "MakeAndModel").ok_or_else(|| Error::Parse("no MakeAndModel".into()))?,
        firmware: text(info, "Revision"),
    })
}

pub fn status(xml: &str) -> Result<PrinterStatus> {
    let doc = parse(xml)?;
    let mut states = Vec::new();
    // The printer attaches localised text to some status entries; alerts
    // refer to the same text by string id.
    let mut messages = HashMap::new();
    for s in named(doc.root(), "Status") {
        if let Some(category) = text(s, "StatusCategory") {
            states.push(category);
        }
        if let (Some(id), Some(msg)) = (text(s, "StringId"), text(s, "LocString")) {
            messages.insert(id, msg);
        }
    }
    let alerts = named(doc.root(), "Alert")
        .filter_map(|a| {
            Some(Alert {
                id: text(a, "ProductStatusAlertID")?,
                severity: match text(a, "Severity").as_deref() {
                    Some("Info") => Severity::Info,
                    Some("Warning") => Severity::Warning,
                    _ => Severity::Error,
                },
                color: text(a, "AlertDetailsMarkerColor"),
                message: text(a, "StringId").and_then(|id| messages.get(&id).cloned()),
            })
        })
        .collect();
    Ok(PrinterStatus { states, alerts })
}

pub fn consumables(xml: &str) -> Result<Vec<Consumable>> {
    let doc = parse(xml)?;
    let entries: Vec<_> = named(doc.root(), "ConsumableInfo")
        .map(|c| (c, text(c, "ConsumableTypeEnum").unwrap_or_default()))
        .collect();
    // On ink-tank models the "inkCartridge" entries are the printheads.
    let has_tanks = entries.iter().any(|(_, t)| t == "inkTank");
    Ok(entries
        .into_iter()
        .map(|(c, kind)| {
            let kind = match kind.as_str() {
                "inkTank" => ConsumableKind::Tank,
                "inkCartridge" if has_tanks => ConsumableKind::Printhead,
                "inkCartridge" => ConsumableKind::Cartridge,
                _ => ConsumableKind::Other,
            };
            Consumable {
                kind,
                label: text(c, "ConsumableLabelCode").unwrap_or_default(),
                part_number: text(c, "ConsumableSelectibilityNumber"),
                level_percent: text(c, "ConsumablePercentageLevelRemaining")
                    .and_then(|v| v.parse().ok()),
                // Tanks have no level sensor: the device reports a fixed value
                // (seen: always 100) that says nothing about the real level.
                level_measured: kind != ConsumableKind::Tank,
            }
        })
        .collect())
}

pub fn usage(xml: &str) -> Result<UsageCounters> {
    let doc = parse(xml)?;
    let Some(unit) = named(doc.root(), "PrinterSubunit").next() else {
        return Ok(UsageCounters::default());
    };
    Ok(UsageCounters {
        total_impressions: child_number(unit, "TotalImpressions"),
        mono_impressions: child_number(unit, "MonochromeImpressions"),
        color_impressions: child_number(unit, "ColorImpressions"),
        jam_events: child_number(unit, "JamEvents"),
        mispick_events: child_number(unit, "MispickEvents"),
    })
}

/// Maps the device's internal print job types to maintenance jobs.
/// Job types that are not maintenance (demo page, network reports) are dropped.
pub fn maintenance_jobs(xml: &str) -> Result<Vec<MaintenanceJob>> {
    let doc = parse(xml)?;
    Ok(named(doc.root(), "JobType")
        .filter_map(|n| {
            let id = n.text()?.trim();
            let kind = match id {
                "cleaningPage" => JobKind::Cleaning { level: 1 },
                "cleaningPageLevel2" => JobKind::Cleaning { level: 2 },
                "cleaningPageLevel3" => JobKind::Cleaning { level: 3 },
                "pqDiagnosticsPage" => JobKind::QualityReport,
                _ => return None,
            };
            Some(MaintenanceJob { id: id.to_string(), kind })
        })
        .collect())
}

/// Tests run against responses recorded from an HP Smart Tank 510
/// (identifying fields scrubbed).
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! fixture {
        ($name:literal) => {
            include_str!(concat!("../../../tests/fixtures/", $name))
        };
    }

    #[test]
    fn reads_model_and_firmware() {
        let info = device_info(fixture!("ProductConfigDyn.xml")).unwrap();
        assert_eq!(info.model, "Smart Tank 510 series");
        // The product firmware revision, not the LEDM schema revision above it.
        assert_eq!(info.firmware.as_deref(), Some("POSPPWPP1N001.2441C.00"));
    }

    #[test]
    fn reads_states_and_alerts() {
        let st = status(fixture!("ProductStatusDyn.xml")).unwrap();
        assert_eq!(st.states, ["genuineHP", "inPowerSave"]);
        assert_eq!(st.alerts.len(), 4);
        let colors: Vec<_> = st.alerts.iter().filter_map(|a| a.color.as_deref()).collect();
        assert_eq!(colors, ["Cyan", "Magenta", "Yellow", "Black"]);
        let first = &st.alerts[0];
        assert_eq!(first.id, "genuineHP");
        assert_eq!(first.severity, Severity::Info);
        assert_eq!(first.message.as_deref(), Some("Tank Dolduruldu"));
    }

    #[test]
    fn reads_printheads_and_tanks() {
        let all = consumables(fixture!("ConsumableConfigDyn.xml")).unwrap();
        assert_eq!(all.len(), 6);

        let heads: Vec<_> = all.iter().filter(|c| c.kind == ConsumableKind::Printhead).collect();
        assert_eq!(heads.len(), 2);
        assert_eq!((heads[0].label.as_str(), heads[0].part_number.as_deref()), ("CMY", Some("M0H50A")));
        assert_eq!((heads[1].label.as_str(), heads[1].part_number.as_deref()), ("K", Some("X4E75A")));
        assert_eq!(heads[0].level_percent, None);

        let tanks: Vec<_> = all.iter().filter(|c| c.kind == ConsumableKind::Tank).collect();
        let labels: Vec<_> = tanks.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["C", "M", "Y", "K"]);
        // Reported as 100 %, but must be flagged as not a real measurement.
        assert!(tanks.iter().all(|c| c.level_percent == Some(100) && !c.level_measured));
    }

    #[test]
    fn cartridges_stay_cartridges_without_tanks() {
        let xml = "<r><ConsumableInfo><ConsumableTypeEnum>inkCartridge</ConsumableTypeEnum>\
                   <ConsumablePercentageLevelRemaining>40</ConsumablePercentageLevelRemaining>\
                   </ConsumableInfo></r>";
        let all = consumables(xml).unwrap();
        assert_eq!(all[0].kind, ConsumableKind::Cartridge);
        assert_eq!(all[0].level_percent, Some(40));
        assert!(all[0].level_measured);
    }

    #[test]
    fn reads_top_level_counters_only() {
        let u = usage(fixture!("ProductUsageDyn.xml")).unwrap();
        assert_eq!(u.total_impressions, Some(4015));
        assert_eq!(u.mono_impressions, Some(589));
        assert_eq!(u.color_impressions, Some(1862));
        assert_eq!(u.jam_events, Some(5));
        assert_eq!(u.mispick_events, Some(19));
    }

    #[test]
    fn keeps_only_maintenance_jobs() {
        let jobs = maintenance_jobs(fixture!("InternalPrintCap.xml")).unwrap();
        let got: Vec<_> = jobs.iter().map(|j| (j.id.as_str(), j.kind)).collect();
        assert_eq!(
            got,
            [
                ("cleaningPage", JobKind::Cleaning { level: 1 }),
                ("cleaningPageLevel2", JobKind::Cleaning { level: 2 }),
                ("cleaningPageLevel3", JobKind::Cleaning { level: 3 }),
                ("pqDiagnosticsPage", JobKind::QualityReport),
            ]
        );
    }

    #[test]
    fn malformed_xml_is_a_parse_error() {
        assert!(matches!(status("<not-closed"), Err(Error::Parse(_))));
    }
}
