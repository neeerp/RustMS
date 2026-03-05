use crate::model::ValidationReport;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub fn write_json(path: &Path, report: &ValidationReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let json = serde_json::to_string_pretty(report)?;
    fs::write(path, json)?;
    Ok(())
}
