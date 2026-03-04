use serde::Serialize;
use std::collections::BTreeMap;

pub type MapIndex = BTreeMap<i32, MapRecord>;

#[derive(Clone, Debug, Serialize)]
pub struct MapRecord {
    pub map_id: i32,
    pub portals: BTreeMap<u32, PortalRecord>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortalRecord {
    pub id: u32,
    pub name: String,
    pub portal_type: i32,
    pub x: i32,
    pub y: i32,
    pub to_map: i32,
    pub to_name: String,
    pub script: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSeverity {
    Hard,
    Soft,
    Warning,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffKind {
    MissingMap,
    MissingPortal,
    FieldMismatch,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub severity: DiffSeverity,
    pub kind: DiffKind,
    pub map_id: i32,
    pub portal_id: Option<u32>,
    pub portal_name: Option<String>,
    pub field: Option<String>,
    pub nx: Option<String>,
    pub cosmic: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MapStatus {
    Exact,
    Warning,
    SoftMismatch,
    HardMismatch,
}

#[derive(Clone, Debug, Serialize)]
pub struct MapComparison {
    pub map_id: i32,
    pub status: MapStatus,
    pub diffs: Vec<Diff>,
    pub hard_count: usize,
    pub soft_count: usize,
    pub warning_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReportSummary {
    pub maps_compared: usize,
    pub exact_maps: usize,
    pub maps_with_hard: usize,
    pub maps_with_soft: usize,
    pub maps_with_warnings: usize,
    pub hard_count: usize,
    pub soft_count: usize,
    pub warning_count: usize,
    pub should_fail: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidationReport {
    pub summary: ReportSummary,
    pub maps: Vec<MapComparison>,
}

pub fn normalize_to_name(name: &str) -> String {
    if name.trim().is_empty() {
        String::new()
    } else {
        name.to_string()
    }
}

pub fn normalize_script(script: Option<String>) -> Option<String> {
    script.and_then(|value| {
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    })
}
