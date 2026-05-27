use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
