use anyhow::{anyhow, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use clap::{ArgGroup, Parser};
use std::collections::BTreeSet;
use std::env::VarError;
use std::path::PathBuf;

use crate::gate::{self, GateMode};
use crate::profile::ThresholdOverrides;

#[derive(Debug, Parser)]
#[command(name = "fastaguard")]
#[command(version)]
#[command(about = "FASTA preflight QC for assembly pipelines")]
#[command(group(
    ArgGroup::new("contract")
        .args(["schema", "finding_catalog", "explain_finding"])
        .multiple(false)
))]
pub struct Cli {
    /// Input FASTA file. Plain .fa/.fasta and gzipped .gz files are supported.
    pub input: Option<PathBuf>,

    /// Print the FastaGuard JSON Schema and exit.
    #[arg(long)]
    pub schema: bool,

    /// Print the machine-readable finding catalog and exit.
    #[arg(long)]
    pub finding_catalog: bool,

    /// Print the catalog entry for one finding ID and exit.
    #[arg(long, value_name = "ID")]
    pub explain_finding: Option<String>,

    /// QC profile. The current release supports assembly.
    #[arg(long, default_value = "assembly")]
    pub profile: String,

    /// Gate preset for pipeline-friendly failure behavior.
    #[arg(long, value_enum, default_value_t = GateMode::None)]
    pub gate: GateMode,

    /// HTML report path.
    #[arg(long, default_value = "fastaguard_report.html")]
    pub out: PathBuf,

    /// JSON report path.
    #[arg(long, default_value = "fastaguard.json")]
    pub json: PathBuf,

    /// TSV summary path.
    #[arg(long, default_value = "fastaguard.tsv")]
    pub tsv: PathBuf,

    /// MultiQC-compatible JSON path.
    #[arg(long, default_value = "fastaguard_mqc.json")]
    pub multiqc: PathBuf,

    /// Comma-separated rule IDs that should fail the run when triggered.
    #[arg(long, value_delimiter = ',')]
    pub fail_on: Vec<String>,

    /// Maximum allowed global N fraction before a high_n_rate finding.
    #[arg(long)]
    pub max_n_rate: Option<f64>,

    /// Minimum contig length used for tiny_contigs finding.
    #[arg(long)]
    pub min_contig_length: Option<u64>,

    /// Expected assembly size using decimal bases, kb, mb, or gb units.
    #[arg(long, value_name = "SIZE")]
    pub expected_size: Option<String>,

    /// Fractional tolerance around --expected-size.
    #[arg(long, default_value_t = 0.25)]
    pub expected_size_tolerance: f64,

    /// Worker thread count reserved for later parallel post-processing.
    #[arg(long, default_value_t = 1)]
    pub threads: usize,
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub input: PathBuf,
    pub profile: String,
    pub gate_mode: GateMode,
    pub outputs: OutputPaths,
    pub rules: RuleConfig,
    pub thresholds: ThresholdOverrides,
    pub threads: usize,
    pub command: String,
    pub started_at: String,
    pub provenance_timestamp_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OutputPaths {
    pub html: PathBuf,
    pub json: PathBuf,
    pub tsv: PathBuf,
    pub multiqc: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub fail_on: BTreeSet<String>,
}

impl Cli {
    pub fn to_run_config(&self) -> Result<RunConfig> {
        let input = self.input.clone().ok_or_else(|| {
            anyhow!("input FASTA is required unless a contract discovery flag is used")
        })?;
        if self.profile != "assembly" {
            return Err(anyhow!(
                "unsupported profile '{}'; the current release supports assembly",
                self.profile
            ));
        }
        if self.threads == 0 {
            return Err(anyhow!("--threads must be at least 1"));
        }
        if let Some(max_n_rate) = self.max_n_rate {
            if !max_n_rate.is_finite() || !(0.0..=1.0).contains(&max_n_rate) {
                return Err(anyhow!(
                    "--max-n-rate must be finite and between 0.0 and 1.0 inclusive"
                ));
            }
        }
        if !self.expected_size_tolerance.is_finite() || self.expected_size_tolerance < 0.0 {
            return Err(anyhow!(
                "--expected-size-tolerance must be finite and non-negative"
            ));
        }
        let expected_size_bases = self
            .expected_size
            .as_deref()
            .map(parse_expected_size)
            .transpose()?;

        let provenance_timestamp_override = provenance_timestamp_override()?;

        Ok(RunConfig {
            input,
            profile: self.profile.clone(),
            gate_mode: self.gate,
            outputs: OutputPaths {
                html: self.out.clone(),
                json: self.json.clone(),
                tsv: self.tsv.clone(),
                multiqc: self.multiqc.clone(),
            },
            rules: RuleConfig {
                fail_on: gate::final_fail_on(self.gate, &self.fail_on),
            },
            thresholds: ThresholdOverrides {
                max_n_rate: self.max_n_rate,
                min_contig_length: self.min_contig_length,
                expected_size_bases,
                expected_size_tolerance: expected_size_bases.map(|_| self.expected_size_tolerance),
            },
            threads: self.threads,
            command: provenance_command(),
            started_at: provenance_timestamp_override
                .clone()
                .unwrap_or_else(current_utc_timestamp),
            provenance_timestamp_override,
        })
    }
}

fn parse_expected_size(value: &str) -> Result<u64> {
    let normalized = value.trim().to_ascii_lowercase();
    let (number, multiplier) = if let Some(number) = normalized.strip_suffix("kb") {
        (number, 1_000_u64)
    } else if let Some(number) = normalized.strip_suffix('k') {
        (number, 1_000_u64)
    } else if let Some(number) = normalized.strip_suffix("mb") {
        (number, 1_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix('m') {
        (number, 1_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix("gb") {
        (number, 1_000_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix('g') {
        (number, 1_000_000_000_u64)
    } else if normalized.chars().all(|ch| ch.is_ascii_digit()) {
        (normalized.as_str(), 1_u64)
    } else {
        return Err(anyhow!(
            "--expected-size accepts bases, kb, mb, or gb decimal units"
        ));
    };
    let parsed = number
        .parse::<u64>()
        .map_err(|_| anyhow!("--expected-size accepts bases, kb, mb, or gb decimal units"))?;
    parsed
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow!("--expected-size is too large"))
}

fn provenance_command() -> String {
    std::env::var("FASTAGUARD_PROVENANCE_COMMAND")
        .unwrap_or_else(|_| std::env::args().collect::<Vec<_>>().join(" "))
}

fn provenance_timestamp_override() -> Result<Option<String>> {
    match std::env::var("FASTAGUARD_PROVENANCE_TIMESTAMP") {
        Ok(value) => normalize_rfc3339_timestamp(&value).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(anyhow!(
            "FASTAGUARD_PROVENANCE_TIMESTAMP must be valid Unicode RFC3339 date-time"
        )),
    }
}

fn normalize_rfc3339_timestamp(value: &str) -> Result<String> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| {
            timestamp
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        })
        .map_err(|_| {
            anyhow!(
                "FASTAGUARD_PROVENANCE_TIMESTAMP must be a valid RFC3339 date-time, got '{}'",
                value
            )
        })
}

fn current_utc_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GateMode;
    use clap::Parser;

    fn cli_with_max_n_rate(max_n_rate: Option<f64>) -> Cli {
        Cli {
            input: Some(PathBuf::from("input.fa")),
            schema: false,
            finding_catalog: false,
            explain_finding: None,
            profile: "assembly".to_string(),
            gate: GateMode::None,
            out: PathBuf::from("fastaguard_report.html"),
            json: PathBuf::from("fastaguard.json"),
            tsv: PathBuf::from("fastaguard.tsv"),
            multiqc: PathBuf::from("fastaguard_mqc.json"),
            fail_on: Vec::new(),
            max_n_rate,
            min_contig_length: None,
            expected_size: None,
            expected_size_tolerance: 0.25,
            threads: 1,
        }
    }

    #[test]
    fn max_n_rate_accepts_inclusive_fraction_bounds() {
        for max_n_rate in [0.0, 0.5, 1.0] {
            let config = cli_with_max_n_rate(Some(max_n_rate))
                .to_run_config()
                .unwrap();

            assert_eq!(config.thresholds.max_n_rate, Some(max_n_rate));
        }
    }

    #[test]
    fn max_n_rate_rejects_non_fraction_values() {
        for max_n_rate in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = cli_with_max_n_rate(Some(max_n_rate))
                .to_run_config()
                .unwrap_err();

            assert!(error
                .to_string()
                .contains("--max-n-rate must be finite and between 0.0 and 1.0"));
        }
    }

    #[test]
    fn default_multiqc_output_uses_mqc_suffix_for_auto_discovery() {
        let cli = Cli::parse_from(["fastaguard", "input.fa"]);
        let config = cli.to_run_config().unwrap();

        assert_eq!(config.outputs.multiqc, PathBuf::from("fastaguard_mqc.json"));
    }

    #[test]
    fn expected_size_parses_decimal_units() {
        let cli = Cli::parse_from([
            "fastaguard",
            "input.fa",
            "--expected-size",
            "5mb",
            "--expected-size-tolerance",
            "0.25",
        ]);
        let config = cli.to_run_config().unwrap();

        assert_eq!(config.thresholds.expected_size_bases, Some(5_000_000));
        assert_eq!(config.thresholds.expected_size_tolerance, Some(0.25));
    }

    #[test]
    fn expected_size_rejects_unknown_units() {
        let cli = Cli::parse_from(["fastaguard", "input.fa", "--expected-size", "5mib"]);
        let error = cli.to_run_config().unwrap_err();

        assert!(error
            .to_string()
            .contains("--expected-size accepts bases, kb, mb, or gb"));
    }

    #[test]
    fn gate_none_preserves_explicit_fail_rules() {
        let cli = Cli::parse_from([
            "fastaguard",
            "input.fa",
            "--gate",
            "none",
            "--fail-on",
            "gc_outliers",
        ]);
        let config = cli.to_run_config().unwrap();

        assert_eq!(config.gate_mode, GateMode::None);
        assert_eq!(
            config.rules.fail_on,
            ["gc_outliers"].into_iter().map(str::to_string).collect()
        );
    }

    #[test]
    fn gate_pipeline_adds_conservative_fail_rules() {
        let cli = Cli::parse_from(["fastaguard", "input.fa", "--gate", "pipeline"]);
        let config = cli.to_run_config().unwrap();

        assert_eq!(config.gate_mode, GateMode::Pipeline);
        assert_eq!(
            config.rules.fail_on,
            [
                "duplicate_ids",
                "high_n_rate",
                "invalid_chars",
                "invalid_fasta_structure",
            ]
            .into_iter()
            .map(str::to_string)
            .collect()
        );
    }

    #[test]
    fn gate_pipeline_unions_explicit_fail_rules() {
        let cli = Cli::parse_from([
            "fastaguard",
            "input.fa",
            "--gate",
            "pipeline",
            "--fail-on",
            "gc_outliers",
        ]);
        let config = cli.to_run_config().unwrap();

        assert!(config.rules.fail_on.contains("duplicate_ids"));
        assert!(config.rules.fail_on.contains("invalid_chars"));
        assert!(config.rules.fail_on.contains("invalid_fasta_structure"));
        assert!(config.rules.fail_on.contains("high_n_rate"));
        assert!(config.rules.fail_on.contains("gc_outliers"));
    }
}
