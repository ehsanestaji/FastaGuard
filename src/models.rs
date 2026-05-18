use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cli::RunConfig;
use crate::findings::Analysis;
use crate::metrics::AssemblyMetrics;
use crate::profile::ProfileConfig;

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const TOOL_NAME: &str = "FastaGuard";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastaguardReport {
    pub schema_version: String,
    pub tool: ToolInfo,
    pub input: InputInfo,
    pub verdict: Verdict,
    pub machine_summary: MachineSummary,
    pub scope: Scope,
    pub provenance: Provenance,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineSummary {
    pub verdict: VerdictStatus,
    pub safe_for_downstream: bool,
    pub top_findings: Vec<String>,
    pub recommended_next_tools: Vec<RecommendedTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedTool {
    pub tool: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub level: String,
    pub can_conclude: Vec<String>,
    pub cannot_conclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub profile: String,
    pub threads: usize,
    pub fail_on: Vec<String>,
    pub thresholds: ProvenanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceThresholds {
    pub high_n_sequence_fraction: f64,
    pub high_global_n_fraction: f64,
    pub min_contig_length: u64,
    pub max_gap_run: u64,
    pub gc_outlier_zscore: f64,
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
    pub evidence: FindingEvidence,
    pub actions: Vec<FindingAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingEvidence {
    pub total_records: u64,
    pub truncated: bool,
    pub records: Vec<EvidenceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub length: u64,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_fraction: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_gap_run: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gc_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingAction {
    pub action_type: String,
    pub target: String,
    pub reason: String,
    pub recommended_tool: String,
    pub requires_external_database: bool,
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
    pub fn from_analysis(
        config: RunConfig,
        profile: &ProfileConfig,
        metrics: AssemblyMetrics,
        analysis: Analysis,
    ) -> Self {
        let findings = analysis.findings;
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: ToolInfo {
                name: TOOL_NAME.to_string(),
                version: TOOL_VERSION.to_string(),
            },
            input: InputInfo {
                path: config.input.display().to_string(),
                profile: config.profile.clone(),
                compressed: path_is_gzip(&config.input),
            },
            verdict: Verdict {
                status: analysis.status,
                reasons: analysis.reasons,
            },
            machine_summary: build_machine_summary(analysis.status, &findings),
            scope: fasta_preflight_scope(),
            provenance: build_provenance(&config, profile),
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
            findings,
            artifacts: Artifacts {
                html: config.outputs.html.display().to_string(),
                tsv: config.outputs.tsv.display().to_string(),
                multiqc: config.outputs.multiqc.display().to_string(),
            },
        }
    }

    pub fn from_invalid_fasta(config: RunConfig, profile: &ProfileConfig, message: String) -> Self {
        let findings = vec![Finding {
            id: "invalid_fasta_structure".to_string(),
            severity: Severity::Critical,
            profile: config.profile.clone(),
            affected_count: 0,
            affected_fraction: 0.0,
            message,
            why_it_matters: "Structurally invalid FASTA cannot be parsed reliably by downstream tools."
                .to_string(),
            suggested_next_step:
                "Fix FASTA headers and ensure every record has sequence data before rerunning FastaGuard."
                    .to_string(),
            evidence: empty_evidence(),
            actions: finding_actions("invalid_fasta_structure"),
        }];

        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: ToolInfo {
                name: TOOL_NAME.to_string(),
                version: TOOL_VERSION.to_string(),
            },
            input: InputInfo {
                path: config.input.display().to_string(),
                profile: config.profile.clone(),
                compressed: path_is_gzip(&config.input),
            },
            verdict: Verdict {
                status: VerdictStatus::Fail,
                reasons: vec!["invalid_fasta_structure".to_string()],
            },
            machine_summary: build_machine_summary(VerdictStatus::Fail, &findings),
            scope: fasta_preflight_scope(),
            provenance: build_provenance(&config, profile),
            summary: Summary {
                sequence_count: 0,
                total_length: 0,
                min_length: 0,
                max_length: 0,
                mean_length: 0.0,
                median_length: 0.0,
                n50: 0,
                n90: 0,
                l50: 0,
                l90: 0,
                gc_percent: 0.0,
                at_percent: 0.0,
                n_percent: 0.0,
                ambiguity_percent: 0.0,
                duplicate_id_count: 0,
                duplicate_sequence_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 0,
            },
            findings,
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

pub fn finding_actions(id: &str) -> Vec<FindingAction> {
    match id {
        "duplicate_ids" => vec![action(
            "rename_records",
            "duplicate FASTA identifiers",
            "Downstream tools often assume FASTA identifiers are unique.",
            "seqkit",
            false,
        )],
        "invalid_chars" => vec![action(
            "correct_symbols",
            "records with invalid sequence symbols",
            "Invalid sequence symbols should be corrected or intentionally recoded before downstream analysis.",
            "seqkit",
            false,
        )],
        "high_n_rate" => vec![
            action(
                "inspect_records",
                "high-N scaffolds",
                "High ambiguity may indicate unresolved assembly regions or masking problems.",
                "seqkit",
                false,
            ),
            action(
                "run_assembly_qc",
                "assembly after FASTA preflight",
                "Assembly-level evaluation can show whether ambiguity affects broader assembly quality.",
                "QUAST",
                false,
            ),
        ],
        "tiny_contigs" => vec![action(
            "filter_or_review_records",
            "tiny contigs",
            "Short records may be noise, but should be reviewed before automatic removal.",
            "seqkit",
            false,
        )],
        "gap_runs" => vec![action(
            "inspect_gap_regions",
            "scaffolds with long N runs",
            "Long gaps may require gap closing, masking review, or scaffold-level inspection.",
            "QUAST",
            false,
        )],
        "duplicate_sequences" => vec![action(
            "deduplicate_or_confirm",
            "duplicate sequence records",
            "Repeated sequence records may be expected in some contexts but should be explicit.",
            "seqkit",
            false,
        )],
        "invalid_fasta_structure" => vec![action(
            "repair_fasta_structure",
            "FASTA headers and records",
            "Every record needs a valid header and sequence data before downstream tools can consume it safely.",
            "parser-aware cleanup script",
            false,
        )],
        _ => Vec::new(),
    }
}

pub fn empty_evidence() -> FindingEvidence {
    FindingEvidence {
        total_records: 0,
        truncated: false,
        records: Vec::new(),
    }
}

fn action(
    action_type: &str,
    target: &str,
    reason: &str,
    recommended_tool: &str,
    requires_external_database: bool,
) -> FindingAction {
    FindingAction {
        action_type: action_type.to_string(),
        target: target.to_string(),
        reason: reason.to_string(),
        recommended_tool: recommended_tool.to_string(),
        requires_external_database,
    }
}

fn build_machine_summary(status: VerdictStatus, findings: &[Finding]) -> MachineSummary {
    MachineSummary {
        verdict: status,
        safe_for_downstream: status == VerdictStatus::Pass,
        top_findings: findings.iter().map(|finding| finding.id.clone()).collect(),
        recommended_next_tools: recommended_next_tools(status, findings),
    }
}

fn recommended_next_tools(status: VerdictStatus, findings: &[Finding]) -> Vec<RecommendedTool> {
    let mut tools = Vec::new();

    if status == VerdictStatus::Pass {
        push_tool(
            &mut tools,
            "QUAST",
            "assembly-level evaluation after FASTA preflight passes",
        );
        push_tool(
            &mut tools,
            "BUSCO",
            "biological completeness after structural FASTA checks pass",
        );
        push_tool(
            &mut tools,
            "BlobToolKit",
            "contamination and cobiont exploration after FASTA preflight passes",
        );
        return tools;
    }

    for finding in findings {
        for action in &finding.actions {
            push_tool(&mut tools, &action.recommended_tool, &action.reason);
        }
    }

    tools
}

fn push_tool(tools: &mut Vec<RecommendedTool>, tool: &str, reason: &str) {
    if tools.iter().any(|existing| existing.tool == tool) {
        return;
    }

    tools.push(RecommendedTool {
        tool: tool.to_string(),
        reason: reason.to_string(),
    });
}

fn fasta_preflight_scope() -> Scope {
    Scope {
        level: "fasta_preflight".to_string(),
        can_conclude: vec![
            "FASTA parse validity".to_string(),
            "duplicate identifiers".to_string(),
            "invalid sequence symbols".to_string(),
            "basic structural statistics".to_string(),
            "sequence composition red flags".to_string(),
        ],
        cannot_conclude: vec![
            "biological completeness".to_string(),
            "taxonomic contamination".to_string(),
            "whole-assembly accuracy".to_string(),
            "misassembly status without alignment evidence".to_string(),
        ],
    }
}

fn build_provenance(config: &RunConfig, profile: &ProfileConfig) -> Provenance {
    Provenance {
        profile: profile.name.clone(),
        threads: config.threads,
        fail_on: config.rules.fail_on.iter().cloned().collect(),
        thresholds: ProvenanceThresholds {
            high_n_sequence_fraction: profile.high_n_sequence_fraction,
            high_global_n_fraction: profile.high_global_n_fraction,
            min_contig_length: profile.min_contig_length,
            max_gap_run: profile.max_gap_run,
            gc_outlier_zscore: profile.gc_outlier_zscore,
        },
    }
}

fn path_is_gzip(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
}
