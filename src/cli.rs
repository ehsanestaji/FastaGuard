use anyhow::{anyhow, Result};
use clap::Parser;
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::profile::ThresholdOverrides;

#[derive(Debug, Parser)]
#[command(name = "fastaguard")]
#[command(version)]
#[command(about = "FASTA preflight QC for assembly pipelines")]
pub struct Cli {
    /// Input FASTA file. Plain .fa/.fasta and gzipped .gz files are supported.
    pub input: PathBuf,

    /// QC profile. v0.1 supports assembly.
    #[arg(long, default_value = "assembly")]
    pub profile: String,

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
    #[arg(long, default_value = "fastaguard_multiqc.json")]
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

    /// Worker thread count reserved for later parallel post-processing.
    #[arg(long, default_value_t = 1)]
    pub threads: usize,
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub input: PathBuf,
    pub profile: String,
    pub outputs: OutputPaths,
    pub rules: RuleConfig,
    pub thresholds: ThresholdOverrides,
    pub threads: usize,
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
        if self.profile != "assembly" {
            return Err(anyhow!(
                "unsupported profile '{}'; v0.1 supports assembly",
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

        Ok(RunConfig {
            input: self.input.clone(),
            profile: self.profile.clone(),
            outputs: OutputPaths {
                html: self.out.clone(),
                json: self.json.clone(),
                tsv: self.tsv.clone(),
                multiqc: self.multiqc.clone(),
            },
            rules: RuleConfig {
                fail_on: normalize_rules(&self.fail_on),
            },
            thresholds: ThresholdOverrides {
                max_n_rate: self.max_n_rate,
                min_contig_length: self.min_contig_length,
            },
            threads: self.threads,
        })
    }
}

fn normalize_rules(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_with_max_n_rate(max_n_rate: Option<f64>) -> Cli {
        Cli {
            input: PathBuf::from("input.fa"),
            profile: "assembly".to_string(),
            out: PathBuf::from("fastaguard_report.html"),
            json: PathBuf::from("fastaguard.json"),
            tsv: PathBuf::from("fastaguard.tsv"),
            multiqc: PathBuf::from("fastaguard_multiqc.json"),
            fail_on: Vec::new(),
            max_n_rate,
            min_contig_length: None,
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
}
