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
        sample.sample_id,
        sample.input_path,
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
        sample.readiness_blockers.join(","),
        sample.recommended_next_tools.join(","),
        sample.input_sha256,
    )
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
