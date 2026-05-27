use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

use crate::models::{FastaguardReport, VerdictStatus};

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
    composite_anomaly_count: u64,
    finding_count: usize,
}

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut data = BTreeMap::new();
    data.insert(sample_name(&report.input.path), summary_row(report));
    let wrapper = MultiqcReport {
        id: "fastaguard",
        section_name: "FastaGuard",
        description: "FASTA preflight QC summary",
        plot_type: "table",
        pconfig: MultiqcPlotConfig {
            id: "fastaguard_summary",
            title: "FastaGuard FASTA preflight summary",
        },
        data,
    };
    serde_json::to_writer_pretty(file, &wrapper)
        .with_context(|| format!("failed to write MultiQC report {}", path.display()))
}

fn sample_name(path: &str) -> String {
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    let name_without_gzip = file_name.strip_suffix(".gz").unwrap_or(file_name);

    Path::new(name_without_gzip)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("sample")
        .to_string()
}

fn summary_row(report: &FastaguardReport) -> MultiqcSummaryRow {
    MultiqcSummaryRow {
        verdict: verdict_status(report.verdict.status),
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
        composite_anomaly_count: affected_record_count(report, "composite_anomalies"),
        finding_count: report.findings.len(),
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

fn verdict_status(status: VerdictStatus) -> &'static str {
    match status {
        VerdictStatus::Pass => "PASS",
        VerdictStatus::Warn => "WARN",
        VerdictStatus::Fail => "FAIL",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use tempfile::NamedTempFile;

    use super::*;
    use crate::models::{
        empty_plots, Artifacts, FastaguardReport, GateDecision, InputInfo, MachineSummary,
        Provenance, ProvenanceThresholds, Scope, Summary, ToolInfo, Verdict, VerdictStatus,
    };

    #[test]
    fn writes_multiqc_custom_content_table() {
        let file = NamedTempFile::new().unwrap();

        write(&test_report(), file.path()).unwrap();

        let output: Value =
            serde_json::from_str(&fs::read_to_string(file.path()).unwrap()).unwrap();
        assert_eq!(output["id"], "fastaguard");
        assert_eq!(output["section_name"], "FastaGuard");
        assert_eq!(output["plot_type"], "table");
        assert_eq!(output["pconfig"]["id"], "fastaguard_summary");
        assert_eq!(output["data"]["sample"]["verdict"], "PASS");
        assert_eq!(output["data"]["sample"]["sequence_count"], 2);
        assert_eq!(output["data"]["sample"]["duplicate_id_count"], 0);
        assert_eq!(output["data"]["sample"]["invalid_sequence_count"], 0);
        assert_eq!(output["data"]["sample"]["high_n_sequence_count"], 0);
        assert_eq!(output["data"]["sample"]["tiny_contig_count"], 0);
        assert_eq!(output["data"]["sample"]["max_gap_run"], 1);
        assert_eq!(output["data"]["sample"]["gc_outlier_count"], 0);
        assert_eq!(output["data"]["sample"]["length_outlier_count"], 0);
        assert_eq!(output["data"]["sample"]["composite_anomaly_count"], 0);
        assert!(output.get("report").is_none(), "{output}");
    }

    #[test]
    fn sample_name_strips_common_fasta_and_gzip_extensions() {
        assert_eq!(sample_name("/data/sample.fa"), "sample");
        assert_eq!(sample_name("/data/sample.fasta.gz"), "sample");
    }

    fn test_report() -> FastaguardReport {
        FastaguardReport {
            schema_version: "0.1.0".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.1.0".to_string(),
            },
            input: InputInfo {
                path: "sample.fa".to_string(),
                profile: "assembly".to_string(),
                compressed: false,
            },
            verdict: Verdict {
                status: VerdictStatus::Pass,
                reasons: Vec::new(),
            },
            gate: GateDecision {
                mode: "none".to_string(),
                status: VerdictStatus::Pass,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            machine_summary: MachineSummary {
                verdict: VerdictStatus::Pass,
                safe_for_downstream: true,
                top_findings: Vec::new(),
                recommended_next_tools: Vec::new(),
                routing_hints: Vec::new(),
            },
            scope: Scope {
                level: "fasta_preflight".to_string(),
                can_conclude: Vec::new(),
                cannot_conclude: Vec::new(),
            },
            provenance: Provenance {
                profile: "assembly".to_string(),
                threads: 1,
                fail_on: Vec::new(),
                thresholds: ProvenanceThresholds {
                    high_n_sequence_fraction: 0.2,
                    high_global_n_fraction: 0.05,
                    min_contig_length: 200,
                    max_gap_run: 100,
                    gc_outlier_zscore: 3.0,
                },
                command: "fastaguard input.fa".to_string(),
                started_at: "2026-05-23T00:00:00Z".to_string(),
                completed_at: "2026-05-23T00:00:00Z".to_string(),
                duration_ms: 0,
                input_size_bytes: 100,
                input_sha256: "0".repeat(64),
            },
            summary: Summary {
                sequence_count: 2,
                total_length: 100,
                min_length: 40,
                max_length: 60,
                mean_length: 50.0,
                median_length: 50.0,
                n50: 60,
                n90: 40,
                l50: 1,
                l90: 2,
                gc_percent: 48.5,
                at_percent: 50.0,
                n_percent: 1.5,
                ambiguity_percent: 1.5,
                duplicate_id_count: 0,
                duplicate_sequence_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 1,
            },
            plots: empty_plots(),
            findings: Vec::new(),
            artifacts: Artifacts {
                html: "fastaguard_report.html".to_string(),
                tsv: "fastaguard.tsv".to_string(),
                multiqc: "fastaguard_mqc.json".to_string(),
            },
        }
    }
}
