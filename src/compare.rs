use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use crate::cli::{CompareConfig, RunConfig};
use crate::models::{
    CompareInputInfo, CompareReport, CompareSample, CompareSummary, FastaguardReport, ToolInfo,
    VerdictStatus, SCHEMA_VERSION, TOOL_NAME, TOOL_VERSION,
};

pub fn run_compare(config: CompareConfig) -> Result<i32> {
    let mut samples = Vec::with_capacity(config.inputs.len());
    for input in &config.inputs {
        let report = run_one_sample(&config, input)?;
        samples.push(compare_sample(input, &report));
    }

    let summary = compare_summary(&samples);
    let worst = worst_status(samples.iter().map(|sample| sample.gate_status));
    let report = CompareReport {
        schema_version: SCHEMA_VERSION.to_string(),
        report_type: "compare".to_string(),
        tool: ToolInfo {
            name: TOOL_NAME.to_string(),
            version: TOOL_VERSION.to_string(),
        },
        input: CompareInputInfo {
            profile: config.profile.clone(),
            sample_count: samples.len(),
        },
        summary,
        samples,
        cohort_findings: Vec::new(),
    };

    crate::report::write_compare_all(&report, &config.outputs)?;
    Ok(exit_code(worst))
}

fn run_one_sample(config: &CompareConfig, input: &Path) -> Result<FastaguardReport> {
    crate::build_single_report(
        RunConfig {
            input: input.to_path_buf(),
            profile: config.profile.clone(),
            gate_mode: config.gate_mode,
            outputs: config.outputs.clone(),
            rules: config.rules.clone(),
            thresholds: config.thresholds,
            threads: config.threads,
            command: config.command.clone(),
            started_at: config.started_at.clone(),
            provenance_timestamp_override: config.provenance_timestamp_override.clone(),
        },
        Instant::now(),
    )
}

fn compare_sample(input: &Path, report: &FastaguardReport) -> CompareSample {
    CompareSample {
        sample_id: sample_id(input),
        input_path: report.input.path.clone(),
        verdict: report.verdict.status,
        gate_status: report.gate.status,
        readiness_status: report.readiness.overall.status,
        sequence_count: report.summary.sequence_count,
        total_length: report.summary.total_length,
        n50: report.summary.n50,
        n90: report.summary.n90,
        gc_percent: report.summary.gc_percent,
        n_percent: report.summary.n_percent,
        duplicate_id_count: report.summary.duplicate_id_count,
        invalid_sequence_count: report.summary.invalid_sequence_count,
        high_n_sequence_count: report.summary.high_n_sequence_count,
        tiny_contig_count: report.summary.tiny_contig_count,
        max_gap_run: report.summary.max_gap_run,
        gc_outlier_count: affected_record_count(report, "gc_outliers"),
        length_outlier_count: affected_record_count(report, "length_outliers"),
        finding_count: report.findings.len(),
        finding_ids: report
            .findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect(),
        readiness_blockers: report.readiness.overall.blockers.clone(),
        recommended_next_tools: report.machine_summary.recommended_next_tools.clone(),
        input_sha256: report.provenance.input_sha256.clone(),
    }
}

fn compare_summary(samples: &[CompareSample]) -> CompareSummary {
    CompareSummary {
        sample_count: samples.len(),
        pass_count: samples
            .iter()
            .filter(|sample| sample.gate_status == VerdictStatus::Pass)
            .count(),
        warn_count: samples
            .iter()
            .filter(|sample| sample.gate_status == VerdictStatus::Warn)
            .count(),
        fail_count: samples
            .iter()
            .filter(|sample| sample.gate_status == VerdictStatus::Fail)
            .count(),
    }
}

fn affected_record_count(report: &FastaguardReport, finding_id: &str) -> u64 {
    report
        .findings
        .iter()
        .find(|finding| finding.id == finding_id)
        .map(|finding| finding.affected_count)
        .unwrap_or(0)
}

pub(crate) fn sample_id(path: &Path) -> String {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| path.as_os_str().to_string_lossy());
    let without_gzip = file_name.strip_suffix(".gz").unwrap_or(&file_name);
    without_gzip
        .strip_suffix(".fasta")
        .or_else(|| without_gzip.strip_suffix(".fa"))
        .unwrap_or(without_gzip)
        .to_string()
}

pub(crate) fn worst_status<I>(statuses: I) -> VerdictStatus
where
    I: IntoIterator<Item = VerdictStatus>,
{
    statuses
        .into_iter()
        .fold(VerdictStatus::Pass, |worst, status| match (worst, status) {
            (VerdictStatus::Fail, _) | (_, VerdictStatus::Fail) => VerdictStatus::Fail,
            (VerdictStatus::Warn, _) | (_, VerdictStatus::Warn) => VerdictStatus::Warn,
            _ => VerdictStatus::Pass,
        })
}

fn exit_code(status: VerdictStatus) -> i32 {
    match status {
        VerdictStatus::Pass => 0,
        VerdictStatus::Warn => 1,
        VerdictStatus::Fail => 2,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn worst_status_prefers_fail_over_warn_over_pass() {
        assert_eq!(
            worst_status([
                VerdictStatus::Pass,
                VerdictStatus::Fail,
                VerdictStatus::Warn
            ]),
            VerdictStatus::Fail
        );
        assert_eq!(
            worst_status([VerdictStatus::Pass, VerdictStatus::Warn]),
            VerdictStatus::Warn
        );
        assert_eq!(worst_status([VerdictStatus::Pass]), VerdictStatus::Pass);
    }

    #[test]
    fn sample_id_uses_file_stem_without_compression_suffix() {
        assert_eq!(sample_id(Path::new("assemblies/ecoli.fa")), "ecoli");
        assert_eq!(sample_id(Path::new("assemblies/ecoli.fasta.gz")), "ecoli");
    }
}
