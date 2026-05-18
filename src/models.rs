use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cli::RunConfig;
use crate::findings::Analysis;
use crate::metrics::AssemblyMetrics;

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const TOOL_NAME: &str = "FastaGuard";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastaguardReport {
    pub schema_version: String,
    pub tool: ToolInfo,
    pub input: InputInfo,
    pub verdict: Verdict,
    pub summary: Summary,
    pub findings: Vec<Finding>,
    pub artifacts: Artifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInfo {
    pub path: String,
    pub profile: String,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub status: VerdictStatus,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VerdictStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Minor,
    Major,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub profile: String,
    pub affected_count: u64,
    pub affected_fraction: f64,
    pub message: String,
    pub why_it_matters: String,
    pub suggested_next_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub sequence_count: u64,
    pub total_length: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub n50: u64,
    pub n90: u64,
    pub l50: u64,
    pub l90: u64,
    pub gc_percent: f64,
    pub at_percent: f64,
    pub n_percent: f64,
    pub ambiguity_percent: f64,
    pub duplicate_id_count: u64,
    pub duplicate_sequence_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    pub html: String,
    pub tsv: String,
    pub multiqc: String,
}

impl FastaguardReport {
    pub fn from_analysis(config: RunConfig, metrics: AssemblyMetrics, analysis: Analysis) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: ToolInfo {
                name: TOOL_NAME.to_string(),
                version: TOOL_VERSION.to_string(),
            },
            input: InputInfo {
                path: config.input.display().to_string(),
                profile: config.profile,
                compressed: path_is_gzip(&config.input),
            },
            verdict: Verdict {
                status: analysis.status,
                reasons: analysis.reasons,
            },
            summary: Summary {
                sequence_count: metrics.sequence_count,
                total_length: metrics.total_length,
                min_length: metrics.min_length,
                max_length: metrics.max_length,
                mean_length: metrics.mean_length,
                median_length: metrics.median_length,
                n50: metrics.n50,
                n90: metrics.n90,
                l50: metrics.l50,
                l90: metrics.l90,
                gc_percent: metrics.gc_percent,
                at_percent: metrics.at_percent,
                n_percent: metrics.n_percent,
                ambiguity_percent: metrics.ambiguity_percent,
                duplicate_id_count: metrics.duplicate_id_count,
                duplicate_sequence_count: metrics.duplicate_sequence_count,
                invalid_sequence_count: metrics.invalid_sequence_count,
                high_n_sequence_count: metrics.high_n_sequence_count,
                tiny_contig_count: metrics.tiny_contig_count,
                max_gap_run: metrics.max_gap_run,
            },
            findings: analysis.findings,
            artifacts: Artifacts {
                html: config.outputs.html.display().to_string(),
                tsv: config.outputs.tsv.display().to_string(),
                multiqc: config.outputs.multiqc.display().to_string(),
            },
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.verdict.status {
            VerdictStatus::Pass => 0,
            VerdictStatus::Warn => 1,
            VerdictStatus::Fail => 2,
        }
    }
}

fn path_is_gzip(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}
