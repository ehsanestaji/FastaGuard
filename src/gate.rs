use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::models::{Finding, GateDecision, Severity, VerdictStatus};

pub const PIPELINE_FAIL_ON: [&str; 4] = [
    "duplicate_ids",
    "high_n_rate",
    "invalid_chars",
    "invalid_fasta_structure",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum GateMode {
    None,
    Pipeline,
}

impl GateMode {
    pub fn as_str(self) -> &'static str {
        match self {
            GateMode::None => "none",
            GateMode::Pipeline => "pipeline",
        }
    }
}

pub fn final_fail_on(mode: GateMode, explicit_rules: &[String]) -> BTreeSet<String> {
    let mut fail_on = explicit_rules
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();

    if mode == GateMode::Pipeline {
        fail_on.extend(PIPELINE_FAIL_ON.into_iter().map(ToOwned::to_owned));
    }

    fail_on
}

pub fn decision(
    mode: GateMode,
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
        status,
        blocking_findings,
        advisory_findings,
        fail_on: fail_on.iter().cloned().collect(),
    }
}
