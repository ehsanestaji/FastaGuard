use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum SubmissionTarget {
    Generic,
    Ncbi,
}

impl SubmissionTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            SubmissionTarget::Generic => "generic",
            SubmissionTarget::Ncbi => "ncbi",
        }
    }
}
