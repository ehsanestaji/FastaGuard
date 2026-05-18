use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

use crate::models::FastaguardReport;

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}
