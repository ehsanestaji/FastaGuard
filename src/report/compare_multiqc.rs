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
    headers: BTreeMap<&'static str, MultiqcHeader>,
}

#[derive(Serialize)]
struct MultiqcHeader {
    title: &'static str,
}

#[derive(Serialize)]
struct MultiqcSummaryRow {
    verdict: &'static str,
    gate_status: &'static str,
    readiness_status: &'static str,
    submission_target: String,
    submission_status: &'static str,
    submission_ready_count: u64,
    submission_warn_count: u64,
    submission_fail_count: u64,
    sequence_count: u64,
    total_length: u64,
    n50: u64,
    n90: u64,
    gc_percent: f64,
    n_percent: f64,
    duplicate_id_count: u64,
    invalid_sequence_count: u64,
    high_n_sequence_count: u64,
    tiny_contig_count: u64,
    max_gap_run: u64,
    gc_outlier_count: u64,
    length_outlier_count: u64,
    finding_count: u64,
    readiness_blockers: String,
    recommended_next_tools: String,
}

pub fn write(report: &CompareReport, path: &Path) -> Result<()> {
    let mut data = BTreeMap::new();
    for sample in &report.samples {
        data.insert(sample.sample_id.clone(), summary_row(report, sample));
    }

    let wrapper = MultiqcReport {
        id: "fastaguard_compare_summary",
        section_name: "FastaGuard Compare",
        description: "FASTA preflight readiness summary across multiple inputs",
        plot_type: "table",
        pconfig: MultiqcPlotConfig {
            id: "fastaguard_compare_summary",
            title: "FastaGuard Compare",
            headers: summary_headers(),
        },
        data,
    };

    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, &wrapper)
        .with_context(|| format!("failed to write MultiQC report {}", path.display()))?;
    writeln!(file).with_context(|| format!("failed to write MultiQC report {}", path.display()))
}

fn summary_row(report: &CompareReport, sample: &CompareSample) -> MultiqcSummaryRow {
    MultiqcSummaryRow {
        verdict: verdict_status(sample.verdict),
        gate_status: verdict_status(sample.gate_status),
        readiness_status: readiness_status(sample.readiness_status),
        submission_target: sample
            .submission_target
            .clone()
            .unwrap_or_else(|| ".".to_string()),
        submission_status: readiness_status(sample.submission_status),
        submission_ready_count: report.summary.submission_ready_count,
        submission_warn_count: report.summary.submission_warn_count,
        submission_fail_count: report.summary.submission_fail_count,
        sequence_count: sample.sequence_count,
        total_length: sample.total_length,
        n50: sample.n50,
        n90: sample.n90,
        gc_percent: sample.gc_percent,
        n_percent: sample.n_percent,
        duplicate_id_count: sample.duplicate_id_count,
        invalid_sequence_count: sample.invalid_sequence_count,
        high_n_sequence_count: sample.high_n_sequence_count,
        tiny_contig_count: sample.tiny_contig_count,
        max_gap_run: sample.max_gap_run,
        gc_outlier_count: sample.gc_outlier_count,
        length_outlier_count: sample.length_outlier_count,
        finding_count: sample.finding_count,
        readiness_blockers: sample.readiness_blockers.join(","),
        recommended_next_tools: sample.recommended_next_tools.join(","),
    }
}

fn summary_headers() -> BTreeMap<&'static str, MultiqcHeader> {
    [
        ("verdict", "Verdict"),
        ("gate_status", "Gate"),
        ("readiness_status", "Readiness"),
        ("submission_target", "Submission Target"),
        ("submission_status", "Submission Status"),
        ("submission_ready_count", "Submission Ready"),
        ("submission_warn_count", "Submission Warn"),
        ("submission_fail_count", "Submission Fail"),
        ("sequence_count", "Sequences"),
        ("total_length", "Total Length"),
        ("n50", "N50"),
        ("n90", "N90"),
        ("gc_percent", "GC%"),
        ("n_percent", "N%"),
        ("duplicate_id_count", "Duplicate IDs"),
        ("invalid_sequence_count", "Invalid Sequences"),
        ("high_n_sequence_count", "High-N Sequences"),
        ("tiny_contig_count", "Tiny Contigs"),
        ("max_gap_run", "Max Gap Run"),
        ("gc_outlier_count", "GC Outliers"),
        ("length_outlier_count", "Length Outliers"),
        ("finding_count", "Findings"),
        ("readiness_blockers", "Readiness Blockers"),
        ("recommended_next_tools", "Recommended Next Tools"),
    ]
    .into_iter()
    .map(|(id, title)| (id, MultiqcHeader { title }))
    .collect()
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
        assert_eq!(
            output["pconfig"]["headers"]["readiness_status"]["title"],
            "Readiness"
        );
        assert_eq!(
            output["pconfig"]["headers"]["submission_target"]["title"],
            "Submission Target"
        );
        assert_eq!(
            output["pconfig"]["headers"]["submission_status"]["title"],
            "Submission Status"
        );
        assert_eq!(
            output["pconfig"]["headers"]["submission_ready_count"]["title"],
            "Submission Ready"
        );
        assert_eq!(
            output["pconfig"]["headers"]["submission_warn_count"]["title"],
            "Submission Warn"
        );
        assert_eq!(
            output["pconfig"]["headers"]["submission_fail_count"]["title"],
            "Submission Fail"
        );
        assert_eq!(
            output["pconfig"]["headers"]["duplicate_id_count"]["title"],
            "Duplicate IDs"
        );
        assert_eq!(output["data"]["sample_a"]["verdict"], "PASS");
        assert_eq!(output["data"]["sample_a"]["submission_target"], "ncbi");
        assert_eq!(output["data"]["sample_a"]["submission_status"], "WARN");
        assert_eq!(output["data"]["sample_a"]["submission_ready_count"], 1);
        assert_eq!(output["data"]["sample_a"]["submission_warn_count"], 1);
        assert_eq!(output["data"]["sample_a"]["submission_fail_count"], 0);
        assert_eq!(output["data"]["sample_a"]["duplicate_id_count"], 3);
        assert_eq!(output["data"]["sample_a"]["invalid_sequence_count"], 4);
        assert_eq!(output["data"]["sample_a"]["gc_outlier_count"], 8);
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
                submission_ready_count: 1,
                submission_warn_count: 1,
                submission_fail_count: 0,
            },
            samples: vec![CompareSample {
                sample_id: "sample_a".to_string(),
                input_path: "sample_a.fa".to_string(),
                verdict: VerdictStatus::Pass,
                gate_status: VerdictStatus::Pass,
                readiness_status: crate::readiness::ReadinessStatus::Pass,
                submission_target: Some("ncbi".to_string()),
                submission_status: crate::readiness::ReadinessStatus::Warn,
                readiness_categories: crate::readiness::build_readiness(
                    VerdictStatus::Pass,
                    &[],
                    &[],
                    crate::readiness::ReadinessScope::Single,
                    None,
                )
                .categories,
                sequence_count: 2,
                total_length: 100,
                n50: 60,
                n90: 40,
                gc_percent: 50.0,
                n_percent: 0.0,
                duplicate_id_count: 3,
                invalid_sequence_count: 4,
                high_n_sequence_count: 5,
                tiny_contig_count: 6,
                max_gap_run: 7,
                gc_outlier_count: 8,
                length_outlier_count: 9,
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
