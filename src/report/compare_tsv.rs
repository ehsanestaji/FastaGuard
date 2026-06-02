use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::models::{CompareReport, CompareSample, VerdictStatus};

const HEADER: &str = "sample_id\tinput_path\tverdict\tgate_status\treadiness_status\tsequence_count\ttotal_length\tn50\tn90\tgc_percent\tn_percent\tduplicate_id_count\tinvalid_sequence_count\thigh_n_sequence_count\ttiny_contig_count\tmax_gap_run\tgc_outlier_count\tlength_outlier_count\tfinding_count\treadiness_blockers\trecommended_next_tools\tinput_sha256";

pub fn write(report: &CompareReport, path: &Path) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "{HEADER}")
        .with_context(|| format!("failed to write TSV report {}", path.display()))?;
    for sample in &report.samples {
        write_sample(&mut writer, sample)
            .with_context(|| format!("failed to write TSV report {}", path.display()))?;
    }

    writer
        .flush()
        .with_context(|| format!("failed to write TSV report {}", path.display()))
}

fn write_sample(writer: &mut impl Write, sample: &CompareSample) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        sanitize_tsv_value(&sample.sample_id),
        sanitize_tsv_value(&sample.input_path),
        verdict_status(sample.verdict),
        verdict_status(sample.gate_status),
        readiness_status(sample.readiness_status),
        sample.sequence_count,
        sample.total_length,
        sample.n50,
        sample.n90,
        sample.gc_percent,
        sample.n_percent,
        sample.duplicate_id_count,
        sample.invalid_sequence_count,
        sample.high_n_sequence_count,
        sample.tiny_contig_count,
        sample.max_gap_run,
        sample.gc_outlier_count,
        sample.length_outlier_count,
        sample.finding_count,
        sanitize_tsv_value(&sample.readiness_blockers.join(",")),
        sanitize_tsv_value(&sample.recommended_next_tools.join(",")),
        sanitize_tsv_value(&sample.input_sha256),
    )
}

fn sanitize_tsv_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\t' | '\r' | '\n' => ' ',
            _ => character,
        })
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

    use tempfile::NamedTempFile;

    use super::*;
    use crate::models::{CompareInputInfo, CompareSummary, ToolInfo, SCHEMA_VERSION};

    #[test]
    fn writes_compare_tsv_header_and_sample_row() {
        let file = NamedTempFile::new().unwrap();

        write(&test_report(), file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(
            output.starts_with("sample_id\tinput_path\tverdict"),
            "{output}"
        );
        assert!(output.contains("sample_a\tsample_a.fa\tPASS"), "{output}");
        assert!(
            output.contains("\tduplicate_ids\tseqkit,QUAST\t"),
            "{output}"
        );
        assert!(!output.lines().any(|line| line.ends_with(' ')), "{output}");
    }

    #[test]
    fn sanitizes_string_fields_without_changing_column_count() {
        let mut report = test_report();
        let sample = &mut report.samples[0];
        sample.sample_id = "sample\tone".to_string();
        sample.input_path = "inputs/sample\none\r.fa".to_string();
        sample.readiness_blockers =
            vec!["duplicate\tids".to_string(), "invalid\nchars".to_string()];
        sample.recommended_next_tools = vec!["seqkit\rstats".to_string(), "QUAST".to_string()];
        sample.input_sha256 = "sha\twith\ncontrols".to_string();
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2, "{output}");
        let header_columns = lines[0].split('\t').count();
        let row_columns = lines[1].split('\t').count();
        assert_eq!(row_columns, header_columns, "{output}");
        assert!(lines[1].contains("sample one"), "{output}");
        assert!(lines[1].contains("inputs/sample one .fa"), "{output}");
        assert!(lines[1].contains("duplicate ids,invalid chars"), "{output}");
        assert!(lines[1].contains("seqkit stats,QUAST"), "{output}");
        assert!(lines[1].contains("sha with controls"), "{output}");
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
                readiness_categories: crate::readiness::build_readiness(
                    VerdictStatus::Pass,
                    &[],
                    &[],
                    crate::readiness::ReadinessScope::Single,
                )
                .categories,
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
