use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::models::{CompareReport, FastaguardReport};

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}

pub fn write_compare(report: &CompareReport, path: &Path) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, report)
        .with_context(|| format!("failed to write JSON report {}", path.display()))?;
    writeln!(file).with_context(|| format!("failed to write JSON report {}", path.display()))
}
