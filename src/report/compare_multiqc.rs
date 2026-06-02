use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::models::{CompareReport, CompareSample, VerdictStatus};

#[derive(Serialize)]
struct MultiqcReport {
    id: &'static str,
    section_name: &'static str,
    description: &'static str,
    plot_type: &'static str,
    pconfig: MultiqcPlotConfig,
    data: BTreeMap<String, MultiqcSummaryRow>,
}

#[derive(Serialize)]
struct MultiqcPlotConfig {
    id: &'static str,
    title: &'static str,
}

#[derive(Serialize)]
struct MultiqcSummaryRow {
    verdict: &'static str,
    gate_status: &'static str,
    readiness_status: &'static str,
    sequence_count: u64,
    total_length: u64,
    n50: u64,
    n90: u64,
    gc_percent: f64,
    n_percent: f64,
    finding_count: u64,
    readiness_blockers: String,
    recommended_next_tools: String,
}

pub fn write(report: &CompareReport, path: &Path) -> Result<()> {
    let mut data = BTreeMap::new();
    for sample in &report.samples {
        data.insert(sample.sample_id.clone(), summary_row(sample));
    }

    let wrapper = MultiqcReport {
        id: "fastaguard_compare_summary",
        section_name: "FastaGuard Compare",
        description: "FASTA preflight readiness summary across multiple inputs",
        plot_type: "table",
        pconfig: MultiqcPlotConfig {
            id: "fastaguard_compare_summary",
            title: "FastaGuard Compare",
        },
        data,
    };

    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, &wrapper)
        .with_context(|| format!("failed to write MultiQC report {}", path.display()))?;
    writeln!(file).with_context(|| format!("failed to write MultiQC report {}", path.display()))
}

fn summary_row(sample: &CompareSample) -> MultiqcSummaryRow {
    MultiqcSummaryRow {
        verdict: verdict_status(sample.verdict),
        gate_status: verdict_status(sample.gate_status),
        readiness_status: readiness_status(sample.readiness_status),
        sequence_count: sample.sequence_count,
        total_length: sample.total_length,
        n50: sample.n50,
        n90: sample.n90,
        gc_percent: sample.gc_percent,
        n_percent: sample.n_percent,
        finding_count: sample.finding_count,
        readiness_blockers: sample.readiness_blockers.join(","),
        recommended_next_tools: sample.recommended_next_tools.join(","),
    }
}

fn verdict_status(status: VerdictStatus) -> &'static str {
    match status {
        VerdictStatus::Pass => "PASS",
        VerdictStatus::Warn => "WARN",
        VerdictStatus::Fail => "FAIL",
    }
}

fn readiness_status(status: crate::readiness::ReadinessStatus) -> &'static str {
    match status {
        crate::readiness::ReadinessStatus::Pass => "PASS",
        crate::readiness::ReadinessStatus::Warn => "WARN",
        crate::readiness::ReadinessStatus::Fail => "FAIL",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use tempfile::NamedTempFile;

    use super::*;
    use crate::models::{CompareInputInfo, CompareSummary, ToolInfo, SCHEMA_VERSION};

    #[test]
    fn writes_multiqc_compare_table() {
        let file = NamedTempFile::new().unwrap();

        write(&test_report(), file.path()).unwrap();

        let output: Value =
            serde_json::from_str(&fs::read_to_string(file.path()).unwrap()).unwrap();
        assert_eq!(output["id"], "fastaguard_compare_summary");
        assert_eq!(output["section_name"], "FastaGuard Compare");
        assert_eq!(
            output["description"],
            "FASTA preflight readiness summary across multiple inputs"
        );
        assert_eq!(output["plot_type"], "table");
        assert_eq!(output["pconfig"]["id"], "fastaguard_compare_summary");
        assert_eq!(output["pconfig"]["title"], "FastaGuard Compare");
        assert_eq!(output["data"]["sample_a"]["verdict"], "PASS");
        assert_eq!(
            output["data"]["sample_a"]["recommended_next_tools"],
            "seqkit,QUAST"
        );
    }

    fn test_report() -> CompareReport {
        CompareReport {
            schema_version: SCHEMA_VERSION.to_string(),
            report_type: "compare".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.4.0".to_string(),
            },
            input: CompareInputInfo {
                profile: "assembly".to_string(),
                sample_count: 1,
            },
            summary: CompareSummary {
                sample_count: 1,
                pass_count: 1,
                warn_count: 0,
                fail_count: 0,
            },
            samples: vec![CompareSample {
                sample_id: "sample_a".to_string(),
                input_path: "sample_a.fa".to_string(),
                verdict: VerdictStatus::Pass,
                gate_status: VerdictStatus::Pass,
                readiness_status: crate::readiness::ReadinessStatus::Pass,
                sequence_count: 2,
                total_length: 100,
                n50: 60,
                n90: 40,
                gc_percent: 50.0,
                n_percent: 0.0,
                duplicate_id_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 0,
                gc_outlier_count: 0,
                length_outlier_count: 0,
                finding_count: 1,
                finding_ids: vec!["duplicate_ids".to_string()],
                readiness_blockers: vec!["duplicate_ids".to_string()],
                recommended_next_tools: vec!["seqkit".to_string(), "QUAST".to_string()],
                input_sha256: "0".repeat(64),
            }],
            cohort_findings: Vec::new(),
        }
    }
}
