use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::models::{Finding, GateDecision, Severity, VerdictStatus};
use crate::submission::SubmissionTarget;

pub const PIPELINE_FAIL_ON: &[&str] = &[
    "duplicate_first_token_ids",
    "duplicate_ids",
    "high_n_rate",
    "invalid_chars",
    "invalid_fasta_structure",
];

pub const SUBMISSION_FAIL_ON_GENERIC: &[&str] = &[
    "duplicate_first_token_ids",
    "duplicate_ids",
    "invalid_chars",
    "invalid_fasta_structure",
    "unsafe_ids",
];

pub const SUBMISSION_FAIL_ON_NCBI: &[&str] = &[
    "duplicate_first_token_ids",
    "duplicate_ids",
    "invalid_chars",
    "invalid_fasta_structure",
    "reserved_header_chars",
    "unsafe_ids",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum GateMode {
    None,
    Pipeline,
    Submission,
}

impl GateMode {
    pub fn as_str(self) -> &'static str {
        match self {
            GateMode::None => "none",
            GateMode::Pipeline => "pipeline",
            GateMode::Submission => "submission",
        }
    }
}

pub fn final_fail_on(
    mode: GateMode,
    submission_target: Option<SubmissionTarget>,
    explicit_rules: &[String],
) -> BTreeSet<String> {
    let mut fail_on = explicit_rules
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();

    match mode {
        GateMode::Pipeline => fail_on.extend(PIPELINE_FAIL_ON.iter().map(|id| (*id).to_string())),
        GateMode::Submission => {
            let preset = match submission_target.unwrap_or(SubmissionTarget::Generic) {
                SubmissionTarget::Generic => SUBMISSION_FAIL_ON_GENERIC,
                SubmissionTarget::Ncbi => SUBMISSION_FAIL_ON_NCBI,
            };
            fail_on.extend(preset.iter().map(|id| (*id).to_string()));
        }
        GateMode::None => {}
    }

    fail_on
}

pub fn decision(
    mode: GateMode,
    submission_target: Option<SubmissionTarget>,
    status: VerdictStatus,
    findings: &[Finding],
    fail_on: &BTreeSet<String>,
) -> GateDecision {
    let mut blocking_findings = Vec::new();
    let mut advisory_findings = Vec::new();

    for finding in findings {
        if fail_on.contains(&finding.id) || finding.severity == Severity::Critical {
            blocking_findings.push(finding.id.clone());
        } else {
            advisory_findings.push(finding.id.clone());
        }
    }

    GateDecision {
        mode: mode.as_str().to_string(),
        submission_target,
        status,
        blocking_findings,
        advisory_findings,
        fail_on: fail_on.iter().cloned().collect(),
    }
}
