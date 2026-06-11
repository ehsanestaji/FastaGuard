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
    write_metric(
        &mut writer,
        "readiness_status",
        readiness_status(report.readiness.overall.status),
    )?;
    write_metric(
        &mut writer,
        "readiness_blockers",
        report.readiness.overall.blockers.join(","),
    )?;
    for category in &report.readiness.categories {
        write_metric(
            &mut writer,
            &format!("readiness_{}_status", category.id),
            readiness_status(category.status),
        )?;
    }
    write_metric(&mut writer, "input_sha256", &report.provenance.input_sha256)?;

    write_metric(&mut writer, "sequence_count", report.summary.sequence_count)?;
    write_metric(&mut writer, "total_length", report.summary.total_length)?;
    write_metric(&mut writer, "min_length", report.summary.min_length)?;
    write_metric(&mut writer, "max_length", report.summary.max_length)?;
    write_metric(&mut writer, "mean_length", report.summary.mean_length)?;
    write_metric(&mut writer, "median_length", report.summary.median_length)?;
    write_metric(&mut writer, "n50", report.summary.n50)?;
    write_metric(&mut writer, "n90", report.summary.n90)?;
    write_metric(&mut writer, "l50", report.summary.l50)?;
    write_metric(&mut writer, "l90", report.summary.l90)?;
    write_metric(&mut writer, "gc_percent", report.summary.gc_percent)?;
    write_metric(&mut writer, "at_percent", report.summary.at_percent)?;
    write_metric(&mut writer, "n_percent", report.summary.n_percent)?;
    write_metric(
        &mut writer,
        "ambiguity_percent",
        report.summary.ambiguity_percent,
    )?;
    write_metric(
        &mut writer,
        "duplicate_id_count",
        report.summary.duplicate_id_count,
    )?;
    write_metric(
        &mut writer,
        "duplicate_first_token_id_count",
        report.summary.duplicate_first_token_id_count,
    )?;
    write_metric(
        &mut writer,
        "duplicate_sequence_count",
        report.summary.duplicate_sequence_count,
    )?;
    write_metric(
        &mut writer,
        "unsafe_id_count",
        report.summary.unsafe_id_count,
    )?;
    write_metric(
        &mut writer,
        "long_header_count",
        report.summary.long_header_count,
    )?;
    write_metric(
        &mut writer,
        "reserved_header_char_count",
        report.summary.reserved_header_char_count,
    )?;
    write_metric(
        &mut writer,
        "invalid_sequence_count",
        report.summary.invalid_sequence_count,
    )?;
    write_metric(
        &mut writer,
        "high_n_sequence_count",
        report.summary.high_n_sequence_count,
    )?;
    write_metric(
        &mut writer,
        "tiny_contig_count",
        report.summary.tiny_contig_count,
    )?;
    write_metric(
        &mut writer,
        "terminal_n_sequence_count",
        report.summary.terminal_n_sequence_count,
    )?;
    write_metric(
        &mut writer,
        "repeated_gap_pattern_sequence_count",
        report.summary.repeated_gap_pattern_sequence_count,
    )?;
    write_metric(&mut writer, "max_gap_run", report.summary.max_gap_run)?;
    write_metric(
        &mut writer,
        "ungapped_total_length",
        report.summary.ungapped_total_length,
    )?;

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
    let value = value.to_string();
    if value.is_empty() {
        writeln!(writer, "{metric}\t.")
    } else {
        writeln!(writer, "{metric}\t{value}")
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

    #[test]
    fn writes_v0_4_summary_counter_rows() {
        let mut report = test_report(VerdictStatus::Warn);
        report.summary.min_length = 11;
        report.summary.duplicate_id_count = 7;
        report.summary.duplicate_first_token_id_count = 1;
        report.summary.duplicate_sequence_count = 8;
        report.summary.unsafe_id_count = 2;
        report.summary.long_header_count = 3;
        report.summary.reserved_header_char_count = 4;
        report.summary.invalid_sequence_count = 9;
        report.summary.high_n_sequence_count = 10;
        report.summary.tiny_contig_count = 11;
        report.summary.terminal_n_sequence_count = 5;
        report.summary.repeated_gap_pattern_sequence_count = 6;
        report.summary.max_gap_run = 12;
        report.summary.ungapped_total_length = 94;
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("min_length\t11\n"), "{output}");
        assert!(output.contains("duplicate_id_count\t7\n"), "{output}");
        assert!(
            output.contains("duplicate_first_token_id_count\t1\n"),
            "{output}"
        );
        assert!(output.contains("duplicate_sequence_count\t8\n"), "{output}");
        assert!(output.contains("unsafe_id_count\t2\n"), "{output}");
        assert!(output.contains("long_header_count\t3\n"), "{output}");
        assert!(
            output.contains("reserved_header_char_count\t4\n"),
            "{output}"
        );
        assert!(output.contains("invalid_sequence_count\t9\n"), "{output}");
        assert!(output.contains("high_n_sequence_count\t10\n"), "{output}");
        assert!(output.contains("tiny_contig_count\t11\n"), "{output}");
        assert!(
            output.contains("terminal_n_sequence_count\t5\n"),
            "{output}"
        );
        assert!(
            output.contains("repeated_gap_pattern_sequence_count\t6\n"),
            "{output}"
        );
        assert!(output.contains("max_gap_run\t12\n"), "{output}");
        assert!(output.contains("ungapped_total_length\t94\n"), "{output}");
    }

    #[test]
    fn writes_empty_metric_values_as_explicit_marker_without_trailing_whitespace() {
        let report = test_report(VerdictStatus::Pass);
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("gate_blocking_findings\t.\n"), "{output}");
        assert!(output.contains("gate_advisory_findings\t.\n"), "{output}");
        assert!(output.contains("readiness_blockers\t.\n"), "{output}");
        for line in output.lines() {
            assert_eq!(line.split('\t').count(), 2, "{line:?} in {output}");
            assert!(!line.ends_with('\t'), "{line:?} in {output}");
            assert!(!line.ends_with(' '), "{line:?} in {output}");
        }
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
                submission_target: None,
                status,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            readiness: crate::readiness::build_readiness(
                VerdictStatus::Pass,
                &[],
                &[],
                crate::readiness::ReadinessScope::Single,
                None,
            ),
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
                submission_target: None,
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
                duplicate_first_token_id_count: 0,
                duplicate_sequence_count: 0,
                unsafe_id_count: 0,
                long_header_count: 0,
                reserved_header_char_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                terminal_n_sequence_count: 0,
                repeated_gap_pattern_sequence_count: 0,
                max_gap_run: 1,
                ungapped_total_length: 100,
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
