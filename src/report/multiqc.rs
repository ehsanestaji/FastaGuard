use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::path::Path;

use crate::models::FastaguardReport;

#[derive(Serialize)]
struct MultiqcReport<'a> {
    id: &'static str,
    section_name: &'static str,
    description: &'static str,
    report: &'a FastaguardReport,
}

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let wrapper = MultiqcReport {
        id: "fastaguard",
        section_name: "FastaGuard",
        description: "FASTA preflight QC for assembly pipelines",
        report,
    };
    serde_json::to_writer_pretty(file, &wrapper)
        .with_context(|| format!("failed to write MultiQC report {}", path.display()))
}
