use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::models::{
    CohortFinding, CompareInputInfo, CompareReport, CompareSample, CompareSummary,
    FastaguardReport, ToolInfo, VerdictStatus,
};

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, report)
        .with_context(|| format!("failed to write JSON report {}", path.display()))?;
    writeln!(file).with_context(|| format!("failed to write JSON report {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}

pub fn write_compare(report: &CompareReport, path: &Path) -> Result<()> {
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, &compare_report_view(report))
        .with_context(|| format!("failed to write JSON report {}", path.display()))?;
    writeln!(file).with_context(|| format!("failed to write JSON report {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}

pub(crate) fn compare_to_string_pretty(report: &CompareReport) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&compare_report_view(report))
}

pub(crate) fn compare_gate_can_continue(sample: &CompareSample) -> bool {
    sample.readiness_blockers.is_empty()
}

pub(crate) fn compare_submission_policy_id(target: Option<&str>) -> Option<&'static str> {
    match target {
        Some("generic") => Some("generic_submission_readiness"),
        Some("ncbi") => Some("ncbi_genome"),
        _ => None,
    }
}

#[derive(Serialize)]
struct CompareReportView<'a> {
    schema_version: &'a str,
    report_type: &'a str,
    tool: &'a ToolInfo,
    input: &'a CompareInputInfo,
    summary: &'a CompareSummary,
    samples: Vec<CompareSampleView<'a>>,
    cohort_findings: &'a [CohortFinding],
}

#[derive(Serialize)]
struct CompareSampleView<'a> {
    sample_id: &'a str,
    input_path: &'a str,
    verdict: VerdictStatus,
    gate_status: VerdictStatus,
    gate_can_continue: bool,
    readiness_status: crate::readiness::ReadinessStatus,
    submission_target: Option<&'a str>,
    submission_policy_id: Option<&'static str>,
    submission_status: crate::readiness::ReadinessStatus,
    readiness_categories: &'a [crate::readiness::ReadinessCategory],
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
    finding_ids: &'a [String],
    readiness_blockers: &'a [String],
    recommended_next_tools: &'a [String],
    input_sha256: &'a str,
}

fn compare_report_view(report: &CompareReport) -> CompareReportView<'_> {
    CompareReportView {
        schema_version: &report.schema_version,
        report_type: &report.report_type,
        tool: &report.tool,
        input: &report.input,
        summary: &report.summary,
        samples: report.samples.iter().map(compare_sample_view).collect(),
        cohort_findings: &report.cohort_findings,
    }
}

fn compare_sample_view(sample: &CompareSample) -> CompareSampleView<'_> {
    CompareSampleView {
        sample_id: &sample.sample_id,
        input_path: &sample.input_path,
        verdict: sample.verdict,
        gate_status: sample.gate_status,
        gate_can_continue: compare_gate_can_continue(sample),
        readiness_status: sample.readiness_status,
        submission_target: sample.submission_target.as_deref(),
        submission_policy_id: compare_submission_policy_id(sample.submission_target.as_deref()),
        submission_status: sample.submission_status,
        readiness_categories: &sample.readiness_categories,
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
        finding_ids: &sample.finding_ids,
        readiness_blockers: &sample.readiness_blockers,
        recommended_next_tools: &sample.recommended_next_tools,
        input_sha256: &sample.input_sha256,
    }
}
