pub mod html;
pub mod json;
pub mod multiqc;
pub mod tsv;

use anyhow::Result;

use crate::cli::OutputPaths;
use crate::models::FastaguardReport;

pub fn write_all(report: &FastaguardReport, outputs: &OutputPaths) -> Result<()> {
    json::write(report, &outputs.json)?;
    tsv::write(report, &outputs.tsv)?;
    multiqc::write(report, &outputs.multiqc)?;
    html::write(report, &outputs.html)?;
    Ok(())
}
