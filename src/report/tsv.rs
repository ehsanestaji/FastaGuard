use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::models::{FastaguardReport, VerdictStatus};

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "metric\tvalue")?;
    write_metric(&mut writer, "schema_version", &report.schema_version)?;
    write_metric(&mut writer, "profile", &report.input.profile)?;
    write_metric(
        &mut writer,
        "verdict",
        verdict_status(report.verdict.status),
    )?;
    write_metric(&mut writer, "gate_mode", &report.gate.mode)?;
    write_metric(
        &mut writer,
        "gate_status",
        verdict_status(report.gate.status),
    )?;
    write_metric(
        &mut writer,
        "gate_blocking_findings",
        report.gate.blocking_findings.join(","),
    )?;
    write_metric(
        &mut writer,
        "gate_advisory_findings",
        report.gate.advisory_findings.join(","),
    )?;
    write_metric(&mut writer, "input_sha256", &report.provenance.input_sha256)?;
    write_metric(&mut writer, "sequence_count", report.summary.sequence_count)?;
    write_metric(&mut writer, "total_length", report.summary.total_length)?;
    write_metric(&mut writer, "n50", report.summary.n50)?;
    write_metric(&mut writer, "n90", report.summary.n90)?;
    write_metric(&mut writer, "l50", report.summary.l50)?;
    write_metric(&mut writer, "l90", report.summary.l90)?;
    write_metric(&mut writer, "gc_percent", report.summary.gc_percent)?;
    write_metric(&mut writer, "n_percent", report.summary.n_percent)?;
    write_metric(
        &mut writer,
        "gc_outlier_count",
        affected_record_count(report, "gc_outliers"),
    )?;
    write_metric(
        &mut writer,
        "length_outlier_count",
        affected_record_count(report, "length_outliers"),
    )?;
    write_metric(
        &mut writer,
        "composite_anomaly_count",
        affected_record_count(report, "composite_anomalies"),
    )?;
    write_metric(&mut writer, "finding_count", report.findings.len())?;

    writer
        .flush()
        .with_context(|| format!("failed to write TSV report {}", path.display()))
}

fn write_metric(
    writer: &mut impl Write,
    metric: &str,
    value: impl std::fmt::Display,
) -> std::io::Result<()> {
    writeln!(writer, "{metric}\t{value}")
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

    use tempfile::NamedTempFile;

    use super::*;
    use crate::models::{
        empty_evidence, empty_plots, Artifacts, FastaguardReport, Finding, FindingCategory,
        FindingConfidence, GateDecision, InputInfo, MachineSummary, Provenance,
        ProvenanceThresholds, Scope, Severity, Summary, ToolInfo, Verdict, VerdictStatus,
    };

    #[test]
    fn writes_verdict_status_as_uppercase_schema_value() {
        let report = test_report(VerdictStatus::Warn);
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("verdict\tWARN\n"), "{output}");
    }

    #[test]
    fn writes_outlier_counts_from_matching_findings() {
        let mut report = test_report(VerdictStatus::Warn);
        report.findings = vec![
            test_finding("gc_outliers", 2),
            test_finding("length_outliers", 3),
            test_finding("composite_anomalies", 1),
        ];
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("gc_outlier_count\t2\n"), "{output}");
        assert!(output.contains("length_outlier_count\t3\n"), "{output}");
        assert!(output.contains("composite_anomaly_count\t1\n"), "{output}");
    }

    #[test]
    fn writes_gate_and_checksum_rows() {
        let mut report = test_report(VerdictStatus::Fail);
        report.gate.mode = "pipeline".to_string();
        report.gate.status = VerdictStatus::Fail;
        report.gate.blocking_findings = vec!["duplicate_ids".to_string()];
        report.gate.advisory_findings = vec!["gc_outliers".to_string()];
        report.provenance.input_sha256 = "a".repeat(64);
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let checksum = "a".repeat(64);
        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("gate_mode\tpipeline\n"), "{output}");
        assert!(output.contains("gate_status\tFAIL\n"), "{output}");
        assert!(
            output.contains("gate_blocking_findings\tduplicate_ids\n"),
            "{output}"
        );
        assert!(
            output.contains("gate_advisory_findings\tgc_outliers\n"),
            "{output}"
        );
        assert!(
            output.contains(&format!("input_sha256\t{checksum}\n")),
            "{output}"
        );
    }

    fn test_report(status: VerdictStatus) -> FastaguardReport {
        FastaguardReport {
            schema_version: "0.1.0".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.1.0".to_string(),
            },
            input: InputInfo {
                path: "input.fa".to_string(),
                profile: "assembly".to_string(),
                compressed: false,
            },
            verdict: Verdict {
                status,
                reasons: Vec::new(),
            },
            gate: GateDecision {
                mode: "none".to_string(),
                status,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            machine_summary: MachineSummary {
                verdict: status,
                safe_for_downstream: status == VerdictStatus::Pass,
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

    fn test_finding(id: &str, affected_count: u64) -> Finding {
        Finding {
            id: id.to_string(),
            category: FindingCategory::Composition,
            severity: Severity::Minor,
            confidence: FindingConfidence::Moderate,
            requires_followup_tool: false,
            profile: "assembly".to_string(),
            affected_count,
            affected_fraction: 0.0,
            message: String::new(),
            why_it_matters: String::new(),
            suggested_next_step: String::new(),
            evidence: empty_evidence(),
            actions: Vec::new(),
        }
    }
}
