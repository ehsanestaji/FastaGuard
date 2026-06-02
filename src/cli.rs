use anyhow::{anyhow, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{ArgGroup, ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::collections::BTreeSet;
use std::env::VarError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::gate::{self, GateMode};
use crate::profile::ThresholdOverrides;

#[derive(Debug, Parser)]
#[command(name = "fastaguard")]
#[command(version)]
#[command(about = "FASTA preflight QC for assembly pipelines")]
pub struct Cli {
    #[command(flatten)]
    pub contract: ContractFlags,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Debug, Clone, Args, Default)]
#[command(group(
    ArgGroup::new("contract")
        .args(["schema", "finding_catalog", "explain_finding"])
        .multiple(false)
))]
pub struct ContractFlags {
    /// Print the FastaGuard JSON Schema and exit.
    #[arg(long, global = true)]
    pub schema: bool,

    /// Print the machine-readable finding catalog and exit.
    #[arg(long, global = true)]
    pub finding_catalog: bool,

    /// Print the catalog entry for one finding ID and exit.
    #[arg(long, value_name = "ID", global = true)]
    pub explain_finding: Option<String>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Compare two or more FASTA inputs as a cohort.
    Compare(CompareArgs),
}

#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    /// Input FASTA file. Plain .fa/.fasta and gzipped .gz files are supported.
    pub input: Option<PathBuf>,

    #[command(flatten)]
    pub analysis: AnalysisArgs,

    #[command(flatten)]
    pub outputs: RunOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct CompareArgs {
    /// Input FASTA files. Plain .fa/.fasta and gzipped .gz files are supported.
    pub inputs: Vec<PathBuf>,

    #[command(flatten)]
    pub analysis: AnalysisArgs,

    #[command(flatten)]
    pub outputs: CompareOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct AnalysisArgs {
    /// QC profile. The current release supports assembly.
    #[arg(long, default_value = "assembly")]
    pub profile: String,

    /// Gate preset for pipeline-friendly failure behavior.
    #[arg(long, value_enum, default_value_t = GateMode::None)]
    pub gate: GateMode,

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

#[derive(Debug, Clone, Args)]
pub struct RunOutputArgs {
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
}

#[derive(Debug, Clone, Args)]
pub struct CompareOutputArgs {
    /// HTML cohort report path.
    #[arg(long, default_value = "cohort_report.html")]
    pub out: PathBuf,

    /// JSON cohort report path.
    #[arg(long, default_value = "cohort.json")]
    pub json: PathBuf,

    /// TSV cohort summary path.
    #[arg(long, default_value = "cohort.tsv")]
    pub tsv: PathBuf,

    /// MultiQC-compatible cohort JSON path.
    #[arg(long, default_value = "fastaguard_compare_mqc.json")]
    pub multiqc: PathBuf,
}

#[derive(Debug, Clone)]
pub enum CommandConfig {
    Run(RunConfig),
    Compare(CompareConfig),
    Contract,
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
pub struct CompareConfig {
    pub inputs: Vec<PathBuf>,
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

#[derive(Debug, Clone)]
struct ValidatedAnalysis {
    profile: String,
    gate_mode: GateMode,
    rules: RuleConfig,
    thresholds: ThresholdOverrides,
    threads: usize,
}

impl Cli {
    pub fn parse() -> Self {
        Self::try_parse_from(std::env::args_os()).unwrap_or_else(|error| error.exit())
    }

    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Self::try_parse_from(args).unwrap_or_else(|error| error.exit())
    }

    pub fn try_parse_from<I, T>(args: I) -> std::result::Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let matches = Self::command().try_get_matches_from(args)?;
        validate_no_root_run_args_before_subcommand(&matches)?;
        Self::from_arg_matches(&matches)
    }

    pub fn to_command_config(&self) -> Result<CommandConfig> {
        self.contract.validate_exclusive()?;
        if self.contract.is_requested() {
            return Ok(CommandConfig::Contract);
        }

        match &self.command {
            Some(Commands::Compare(args)) => {
                self.run.validate_unused_before_subcommand("compare")?;
                args.to_compare_config()
            }
            None => self.run.to_run_config(),
        }
    }

    pub fn to_run_config(&self) -> Result<RunConfig> {
        match self.to_command_config()? {
            CommandConfig::Run(config) => Ok(config),
            CommandConfig::Compare(_) => Err(anyhow!(
                "compare subcommand does not produce a single-run config"
            )),
            CommandConfig::Contract => Err(anyhow!(
                "contract discovery command does not produce a run config"
            )),
        }
    }
}

fn validate_no_root_run_args_before_subcommand(
    matches: &ArgMatches,
) -> std::result::Result<(), clap::Error> {
    if matches.subcommand_name().is_none() {
        return Ok(());
    }

    for arg_id in ROOT_RUN_ARG_IDS {
        if matches.value_source(arg_id) == Some(ValueSource::CommandLine) {
            return Err(clap::Error::raw(
                ErrorKind::ArgumentConflict,
                "root run arguments cannot be used with subcommands; put compare inputs and options after the subcommand",
            ));
        }
    }

    Ok(())
}

const ROOT_RUN_ARG_IDS: &[&str] = &[
    "input",
    "profile",
    "gate",
    "fail_on",
    "max_n_rate",
    "min_contig_length",
    "expected_size",
    "expected_size_tolerance",
    "threads",
    "out",
    "json",
    "tsv",
    "multiqc",
];

impl ContractFlags {
    fn validate_exclusive(&self) -> Result<()> {
        if self.requested_count() > 1 {
            return Err(anyhow!(
                "contract discovery flags are mutually exclusive; use only one of --schema, --finding-catalog, or --explain-finding"
            ));
        }
        Ok(())
    }

    pub fn is_requested(&self) -> bool {
        self.requested_count() > 0
    }

    fn requested_count(&self) -> usize {
        usize::from(self.schema)
            + usize::from(self.finding_catalog)
            + usize::from(self.explain_finding.is_some())
    }
}

impl RunArgs {
    fn validate_unused_before_subcommand(&self, subcommand: &str) -> Result<()> {
        if self.has_run_args() {
            return Err(anyhow!(
                "root run arguments cannot be used with {subcommand}; put {subcommand} inputs and options after the subcommand"
            ));
        }
        Ok(())
    }

    fn has_run_args(&self) -> bool {
        self.input.is_some() || self.analysis.has_overrides() || self.outputs.has_overrides()
    }

    fn to_run_config(&self) -> Result<CommandConfig> {
        let input = self.input.clone().ok_or_else(|| {
            anyhow!("input FASTA is required unless a contract discovery flag is used")
        })?;
        let analysis = self.analysis.validate()?;
        let provenance_timestamp_override = provenance_timestamp_override()?;

        Ok(CommandConfig::Run(RunConfig {
            input,
            profile: analysis.profile,
            gate_mode: analysis.gate_mode,
            outputs: self.outputs.output_paths(),
            rules: analysis.rules,
            thresholds: analysis.thresholds,
            threads: analysis.threads,
            command: provenance_command(),
            started_at: provenance_timestamp_override
                .clone()
                .unwrap_or_else(current_utc_timestamp),
            provenance_timestamp_override,
        }))
    }
}

impl CompareArgs {
    fn to_compare_config(&self) -> Result<CommandConfig> {
        if self.inputs.len() < 2 {
            return Err(anyhow!("compare requires at least two FASTA inputs"));
        }

        let analysis = self.analysis.validate()?;
        let provenance_timestamp_override = provenance_timestamp_override()?;

        Ok(CommandConfig::Compare(CompareConfig {
            inputs: self.inputs.clone(),
            profile: analysis.profile,
            gate_mode: analysis.gate_mode,
            outputs: self.outputs.output_paths(),
            rules: analysis.rules,
            thresholds: analysis.thresholds,
            threads: analysis.threads,
            command: provenance_command(),
            started_at: provenance_timestamp_override
                .clone()
                .unwrap_or_else(current_utc_timestamp),
            provenance_timestamp_override,
        }))
    }
}

impl AnalysisArgs {
    fn has_overrides(&self) -> bool {
        self.profile != "assembly"
            || self.gate != GateMode::None
            || !self.fail_on.is_empty()
            || self.max_n_rate.is_some()
            || self.min_contig_length.is_some()
            || self.expected_size.is_some()
            || self.expected_size_tolerance != 0.25
            || self.threads != 1
    }

    fn validate(&self) -> Result<ValidatedAnalysis> {
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

        Ok(ValidatedAnalysis {
            profile: self.profile.clone(),
            gate_mode: self.gate,
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
        })
    }
}

impl RunOutputArgs {
    fn has_overrides(&self) -> bool {
        self.out != Path::new("fastaguard_report.html")
            || self.json != Path::new("fastaguard.json")
            || self.tsv != Path::new("fastaguard.tsv")
            || self.multiqc != Path::new("fastaguard_mqc.json")
    }

    fn output_paths(&self) -> OutputPaths {
        OutputPaths {
            html: self.out.clone(),
            json: self.json.clone(),
            tsv: self.tsv.clone(),
            multiqc: self.multiqc.clone(),
        }
    }
}

impl CompareOutputArgs {
    fn output_paths(&self) -> OutputPaths {
        OutputPaths {
            html: self.out.clone(),
            json: self.json.clone(),
            tsv: self.tsv.clone(),
            multiqc: self.multiqc.clone(),
        }
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

    fn cli_with_max_n_rate(max_n_rate: Option<f64>) -> Cli {
        Cli {
            contract: ContractFlags::default(),
            command: None,
            run: RunArgs {
                input: Some(PathBuf::from("input.fa")),
                analysis: AnalysisArgs {
                    profile: "assembly".to_string(),
                    gate: GateMode::None,
                    fail_on: Vec::new(),
                    max_n_rate,
                    min_contig_length: None,
                    expected_size: None,
                    expected_size_tolerance: 0.25,
                    threads: 1,
                },
                outputs: RunOutputArgs {
                    out: PathBuf::from("fastaguard_report.html"),
                    json: PathBuf::from("fastaguard.json"),
                    tsv: PathBuf::from("fastaguard.tsv"),
                    multiqc: PathBuf::from("fastaguard_mqc.json"),
                },
            },
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
                "duplicate_first_token_ids",
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

    #[test]
    fn compare_defaults_to_compare_output_names() {
        let cli = Cli::parse_from(["fastaguard", "compare", "a.fa", "b.fa"]);
        let config = cli.to_command_config().unwrap();

        let CommandConfig::Compare(config) = config else {
            panic!("expected compare config");
        };

        assert_eq!(
            config.inputs,
            vec![PathBuf::from("a.fa"), PathBuf::from("b.fa")]
        );
        assert_eq!(config.outputs.html, PathBuf::from("cohort_report.html"));
        assert_eq!(config.outputs.json, PathBuf::from("cohort.json"));
        assert_eq!(config.outputs.tsv, PathBuf::from("cohort.tsv"));
        assert_eq!(
            config.outputs.multiqc,
            PathBuf::from("fastaguard_compare_mqc.json")
        );
    }

    #[test]
    fn compare_rejects_single_input() {
        let cli = Cli::parse_from(["fastaguard", "compare", "a.fa"]);
        let error = cli.to_command_config().unwrap_err();

        assert!(error
            .to_string()
            .contains("compare requires at least two FASTA inputs"));
    }

    #[test]
    fn compare_rejects_root_input_before_subcommand() {
        let error =
            Cli::try_parse_from(["fastaguard", "input.fa", "compare", "b.fa", "c.fa"]).unwrap_err();

        assert!(error
            .to_string()
            .contains("root run arguments cannot be used with subcommands"));
    }

    #[test]
    fn compare_rejects_root_analysis_options_before_subcommand() {
        let error =
            Cli::try_parse_from(["fastaguard", "--threads", "0", "compare", "a.fa", "b.fa"])
                .unwrap_err();

        assert!(error
            .to_string()
            .contains("root run arguments cannot be used with subcommands"));
    }

    #[test]
    fn compare_rejects_explicit_default_root_options_before_subcommand() {
        let error =
            Cli::try_parse_from(["fastaguard", "--threads", "1", "compare", "a.fa", "b.fa"])
                .unwrap_err();

        assert!(error
            .to_string()
            .contains("root run arguments cannot be used with subcommands"));
    }

    #[test]
    fn compare_accepts_analysis_options_after_subcommand() {
        let cli = Cli::parse_from(["fastaguard", "compare", "a.fa", "b.fa", "--threads", "1"]);
        let config = cli.to_command_config().unwrap();

        let CommandConfig::Compare(config) = config else {
            panic!("expected compare config");
        };

        assert_eq!(config.threads, 1);
    }

    #[test]
    fn contract_flags_remain_mutually_exclusive_after_compare() {
        let cli = Cli::parse_from(["fastaguard", "compare", "--schema", "--finding-catalog"]);
        let error = cli.to_command_config().unwrap_err();

        assert!(error
            .to_string()
            .contains("contract discovery flags are mutually exclusive"));
    }
}
