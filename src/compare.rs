use anyhow::{anyhow, Result};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

use crate::cli::{CompareConfig, RunConfig};
use crate::models::{
    CohortFinding, CompareInputInfo, CompareReport, CompareSample, CompareSummary,
    FastaguardReport, Severity, ToolInfo, VerdictStatus, SCHEMA_VERSION, TOOL_NAME, TOOL_VERSION,
};
use crate::stats::outliers::{iqr_outlier_indices, zscore_outlier_indices};

pub fn run_compare(config: CompareConfig) -> Result<i32> {
    validate_unique_sample_ids(&config.inputs)?;

    let mut samples = Vec::with_capacity(config.inputs.len());
    for input in &config.inputs {
        let report = run_one_sample(&config, input)?;
        samples.push(compare_sample(input, &report));
    }

    let summary = compare_summary(&samples);
    let worst = worst_status(samples.iter().map(|sample| sample.gate_status));
    let cohort_findings = cohort_findings(&samples);
    let report = CompareReport {
        schema_version: SCHEMA_VERSION.to_string(),
        report_type: "compare".to_string(),
        tool: ToolInfo {
            name: TOOL_NAME.to_string(),
            version: TOOL_VERSION.to_string(),
        },
        input: CompareInputInfo {
            profile: config.profile.clone(),
            sample_count: usize_to_u64(samples.len()),
        },
        summary,
        samples,
        cohort_findings,
    };

    crate::report::write_compare_all(&report, &config.outputs)?;
    Ok(exit_code(worst))
}

fn validate_unique_sample_ids(inputs: &[std::path::PathBuf]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for input in inputs {
        let id = sample_id(input);
        if !seen.insert(id.clone()) {
            return Err(anyhow!("duplicate compare sample_id '{id}'"));
        }
    }
    Ok(())
}

fn run_one_sample(config: &CompareConfig, input: &Path) -> Result<FastaguardReport> {
    crate::build_single_report(
        RunConfig {
            input: input.to_path_buf(),
            profile: config.profile.clone(),
            gate_mode: config.gate_mode,
            submission_target: config.submission_target,
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
        readiness_categories: report.readiness.categories.clone(),
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
        finding_count: usize_to_u64(report.findings.len()),
        finding_ids: report
            .findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect(),
        readiness_blockers: report.readiness.overall.blockers.clone(),
        recommended_next_tools: report
            .machine_summary
            .recommended_next_tools
            .iter()
            .map(|tool| tool.tool.clone())
            .collect(),
        input_sha256: report.provenance.input_sha256.clone(),
    }
}

fn compare_summary(samples: &[CompareSample]) -> CompareSummary {
    CompareSummary {
        sample_count: usize_to_u64(samples.len()),
        pass_count: count_status(samples, VerdictStatus::Pass),
        warn_count: count_status(samples, VerdictStatus::Warn),
        fail_count: count_status(samples, VerdictStatus::Fail),
    }
}

pub(crate) fn cohort_findings(samples: &[CompareSample]) -> Vec<CohortFinding> {
    let mut findings = Vec::new();

    let total_lengths = samples
        .iter()
        .map(|sample| sample.total_length)
        .collect::<Vec<_>>();
    let length_indices = iqr_outlier_indices(&total_lengths, 1.5);
    if !length_indices.is_empty() {
        findings.push(CohortFinding {
            id: "cohort_total_length_outliers".to_string(),
            severity: Severity::Minor,
            affected_count: usize_to_u64(length_indices.len()),
            evidence: serde_json::json!({
                "records": length_indices
                    .iter()
                    .map(|index| {
                        let sample = &samples[*index];
                        serde_json::json!({
                            "sample_id": sample.sample_id,
                            "total_length": sample.total_length,
                            "reason": "total length is unusual relative to the cohort",
                        })
                    })
                    .collect::<Vec<_>>(),
            }),
        });
    }

    let gc_percentages = samples
        .iter()
        .map(|sample| sample.gc_percent)
        .collect::<Vec<_>>();
    let gc_indices = zscore_outlier_indices(&gc_percentages, 2.0);
    if !gc_indices.is_empty() {
        findings.push(CohortFinding {
            id: "cohort_gc_outliers".to_string(),
            severity: Severity::Minor,
            affected_count: usize_to_u64(gc_indices.len()),
            evidence: serde_json::json!({
                "records": gc_indices
                    .iter()
                    .map(|index| {
                        let sample = &samples[*index];
                        serde_json::json!({
                            "sample_id": sample.sample_id,
                            "gc_percent": sample.gc_percent,
                            "reason": "GC percent is unusual relative to the cohort",
                        })
                    })
                    .collect::<Vec<_>>(),
            }),
        });
    }

    findings
}

fn count_status(samples: &[CompareSample], status: VerdictStatus) -> u64 {
    usize_to_u64(
        samples
            .iter()
            .filter(|sample| sample.gate_status == status)
            .count(),
    )
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

fn usize_to_u64(value: usize) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
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

    #[test]
    fn cohort_total_length_outliers_rank_unusual_samples() {
        let samples = vec![
            sample_for_cohort("sample_a", 100_000, 50.0, 0.1, 10, 20_000),
            sample_for_cohort("sample_b", 101_000, 50.2, 0.1, 11, 20_500),
            sample_for_cohort("sample_c", 99_500, 49.8, 0.1, 10, 19_500),
            sample_for_cohort("sample_d", 102_000, 50.1, 0.1, 12, 21_000),
            sample_for_cohort("sample_e", 1_000_000, 50.0, 0.1, 20, 100_000),
        ];

        let findings = cohort_findings(&samples);

        let length_finding = findings
            .iter()
            .find(|finding| finding.id == "cohort_total_length_outliers")
            .unwrap_or_else(|| panic!("missing cohort_total_length_outliers: {findings:#?}"));
        assert_eq!(length_finding.affected_count, 1);
        assert_eq!(
            length_finding.evidence["records"][0]["sample_id"],
            "sample_e"
        );
        assert_eq!(
            length_finding.evidence["records"][0]["total_length"],
            1_000_000
        );
        assert_eq!(
            length_finding.evidence["records"][0]["reason"],
            "total length is unusual relative to the cohort"
        );
    }

    fn sample_for_cohort(
        sample_id: &str,
        total_length: u64,
        gc_percent: f64,
        n_percent: f64,
        sequence_count: u64,
        n50: u64,
    ) -> CompareSample {
        CompareSample {
            sample_id: sample_id.to_string(),
            input_path: format!("{sample_id}.fa"),
            verdict: VerdictStatus::Pass,
            gate_status: VerdictStatus::Pass,
            readiness_status: crate::readiness::ReadinessStatus::Pass,
            readiness_categories: crate::readiness::build_readiness(
                VerdictStatus::Pass,
                &[],
                &[],
                crate::readiness::ReadinessScope::Single,
            )
            .categories,
            sequence_count,
            total_length,
            n50,
            n90: n50,
            gc_percent,
            n_percent,
            duplicate_id_count: 0,
            invalid_sequence_count: 0,
            high_n_sequence_count: 0,
            tiny_contig_count: 0,
            max_gap_run: 0,
            gc_outlier_count: 0,
            length_outlier_count: 0,
            finding_count: 0,
            finding_ids: Vec::new(),
            readiness_blockers: Vec::new(),
            recommended_next_tools: Vec::new(),
            input_sha256: "0".repeat(64),
        }
    }
}
