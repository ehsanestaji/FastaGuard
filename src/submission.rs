use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum SubmissionTarget {
    Generic,
    Ncbi,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionPolicy {
    pub id: String,
    pub version: String,
    pub source_url: String,
    pub scope: String,
    pub limitations: Vec<String>,
    pub thresholds: SubmissionPolicyThresholds,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmissionPolicyThresholds {
    pub seqid_max_bytes: Option<u64>,
    pub min_record_length_bases: Option<u64>,
    pub terminal_n_prohibited: bool,
}

impl SubmissionTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            SubmissionTarget::Generic => "generic",
            SubmissionTarget::Ncbi => "ncbi",
        }
    }
}

pub fn policy_for(target: SubmissionTarget) -> SubmissionPolicy {
    match target {
        SubmissionTarget::Generic => SubmissionPolicy {
            id: "generic_submission_readiness".to_string(),
            version: "2026-08-21".to_string(),
            source_url: "https://github.com/ehsanestaji/FastaGuard/blob/main/docs/preflight-readiness.md"
                .to_string(),
            scope: "Portable FASTA-level submission-readiness checks for common identifier, alphabet, and structural hazards."
                .to_string(),
            limitations: vec![
                "Supports portability across submission workflows and does not establish acceptance by any repository."
                    .to_string(),
            ],
            thresholds: SubmissionPolicyThresholds {
                seqid_max_bytes: None,
                min_record_length_bases: None,
                terminal_n_prohibited: false,
            },
        },
        SubmissionTarget::Ncbi => SubmissionPolicy {
            id: "ncbi_genome".to_string(),
            version: "2026-08-21".to_string(),
            source_url: "https://www.ncbi.nlm.nih.gov/genbank/table2asn/".to_string(),
            scope: "NCBI genome-assembly FASTA preflight".to_string(),
            limitations: vec![
                "Does not validate annotation, taxonomy, contamination, metadata, or repository acceptance."
                    .to_string(),
            ],
            thresholds: SubmissionPolicyThresholds {
                seqid_max_bytes: Some(49),
                min_record_length_bases: Some(200),
                terminal_n_prohibited: true,
            },
        },
    }
}

pub fn policy_for_option(target: Option<SubmissionTarget>) -> Option<SubmissionPolicy> {
    target.map(policy_for)
}

#[cfg(test)]
mod tests {
    use super::{policy_for, SubmissionTarget};

    #[test]
    fn policies_identify_generic_and_ncbi_submission_readiness() {
        assert_eq!(
            policy_for(SubmissionTarget::Generic).id,
            "generic_submission_readiness"
        );
        assert_eq!(policy_for(SubmissionTarget::Ncbi).id, "ncbi_genome");
    }

    #[test]
    fn ncbi_policy_has_stable_documentation_metadata() {
        let policy = policy_for(SubmissionTarget::Ncbi);

        assert_eq!(policy.version, "2026-08-21");
        assert_eq!(
            policy.source_url,
            "https://www.ncbi.nlm.nih.gov/genbank/table2asn/"
        );
        assert_eq!(policy.scope, "NCBI genome-assembly FASTA preflight");
        assert_eq!(
            serde_json::to_value(&policy.limitations).unwrap(),
            serde_json::json!([
                "Does not validate annotation, taxonomy, contamination, metadata, or repository acceptance."
            ])
        );
        assert_eq!(policy.thresholds.seqid_max_bytes, Some(49));
    }

    #[test]
    fn generic_policy_uses_project_documentation_as_its_provenance() {
        assert_eq!(
            policy_for(SubmissionTarget::Generic).source_url,
            "https://github.com/ehsanestaji/FastaGuard/blob/main/docs/preflight-readiness.md"
        );
    }
}
