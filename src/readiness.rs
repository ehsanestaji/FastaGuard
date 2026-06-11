use serde::{Deserialize, Serialize};

use crate::models::{Finding, VerdictStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessScope {
    Single,
    Compare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReadinessStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub overall: ReadinessOverall,
    pub categories: Vec<ReadinessCategory>,
}

impl ReadinessReport {
    pub fn category(&self, id: &str) -> Option<&ReadinessCategory> {
        self.categories.iter().find(|category| category.id == id)
    }
}

impl Default for ReadinessReport {
    fn default() -> Self {
        build_readiness(VerdictStatus::Pass, &[], &[], ReadinessScope::Single, None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessOverall {
    pub status: ReadinessStatus,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessCategory {
    pub id: String,
    pub label: String,
    pub target: Option<String>,
    pub status: ReadinessStatus,
    pub findings: Vec<String>,
}

pub fn build_readiness(
    verdict: VerdictStatus,
    blocking_findings: &[String],
    findings: &[Finding],
    scope: ReadinessScope,
    submission_target: Option<crate::submission::SubmissionTarget>,
) -> ReadinessReport {
    let mut categories = base_categories(scope);
    if let Some(target) = submission_target {
        if let Some(category) = categories
            .iter_mut()
            .find(|category| category.id == "submission")
        {
            category.target = Some(target.as_str().to_string());
        }
    }

    let mut blockers = Vec::new();
    for finding in findings {
        for category_id in category_ids_for_finding(&finding.id) {
            if let Some(category) = categories
                .iter_mut()
                .find(|category| category.id == *category_id)
            {
                category.findings.push(finding.id.clone());
                let is_blocking = blocking_findings.iter().any(|id| id == &finding.id)
                    || matches!(finding.severity, crate::models::Severity::Critical);
                let status = if is_blocking {
                    ReadinessStatus::Fail
                } else {
                    ReadinessStatus::Warn
                };
                category.status = max_status(category.status, status);
                if is_blocking {
                    blockers.push(format!("{}.{}", category_id, finding.id));
                }
            }
        }
    }

    let overall_status = if !blockers.is_empty() {
        ReadinessStatus::Fail
    } else {
        match verdict {
            VerdictStatus::Pass => ReadinessStatus::Pass,
            VerdictStatus::Warn => ReadinessStatus::Warn,
            VerdictStatus::Fail => ReadinessStatus::Fail,
        }
    };

    ReadinessReport {
        overall: ReadinessOverall {
            status: overall_status,
            blockers,
        },
        categories,
    }
}

fn base_categories(scope: ReadinessScope) -> Vec<ReadinessCategory> {
    let mut ids = vec![
        ("file", "File readiness"),
        ("structure", "Structure readiness"),
        ("alphabet", "Alphabet readiness"),
        ("index", "Index readiness"),
        ("assembly", "Assembly readiness"),
        ("submission", "Submission readiness"),
        ("machine", "Machine readiness"),
    ];
    if matches!(scope, ReadinessScope::Compare) {
        ids.insert(6, ("cohort", "Cohort readiness"));
    }
    ids.into_iter()
        .map(|(id, label)| ReadinessCategory {
            id: id.to_string(),
            label: label.to_string(),
            target: None,
            status: ReadinessStatus::Pass,
            findings: Vec::new(),
        })
        .collect()
}

fn category_ids_for_finding(id: &str) -> &'static [&'static str] {
    match id {
        "invalid_fasta_structure" => &["file", "structure", "submission"],
        "invalid_chars" => &["alphabet", "submission"],
        "duplicate_ids" | "duplicate_first_token_ids" => &["index", "submission"],
        "unsafe_ids" | "long_headers" | "reserved_header_chars" => &["index", "submission"],
        "terminal_ns" | "gap_pattern_warnings" | "gap_runs" => &["assembly", "submission"],
        "high_n_rate" | "tiny_contigs" => &["assembly", "submission"],
        "gc_outliers" | "length_outliers" | "composite_anomalies" | "expected_size_outlier" => {
            &["assembly"]
        }
        "cohort_total_length_outliers"
        | "cohort_gc_outliers"
        | "cohort_n_percent_outliers"
        | "cohort_sequence_count_outliers"
        | "cohort_n50_outliers" => &["cohort"],
        _ => &["machine"],
    }
}

fn max_status(left: ReadinessStatus, right: ReadinessStatus) -> ReadinessStatus {
    match (left, right) {
        (ReadinessStatus::Fail, _) | (_, ReadinessStatus::Fail) => ReadinessStatus::Fail,
        (ReadinessStatus::Warn, _) | (_, ReadinessStatus::Warn) => ReadinessStatus::Warn,
        _ => ReadinessStatus::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Finding, FindingCategory, FindingConfidence, Severity, VerdictStatus};

    fn finding(id: &str, severity: Severity) -> Finding {
        Finding {
            id: id.to_string(),
            category: FindingCategory::Validity,
            severity,
            confidence: FindingConfidence::High,
            requires_followup_tool: false,
            profile: "assembly".to_string(),
            affected_count: 1,
            affected_fraction: 0.5,
            message: format!("{id} message"),
            why_it_matters: format!("{id} matters"),
            suggested_next_step: format!("{id} action"),
            evidence: crate::models::empty_evidence(),
            actions: Vec::new(),
        }
    }

    #[test]
    fn duplicate_first_token_ids_fail_index_readiness() {
        let readiness = build_readiness(
            VerdictStatus::Fail,
            &["duplicate_first_token_ids".to_string()],
            &[finding("duplicate_first_token_ids", Severity::Critical)],
            ReadinessScope::Single,
            None,
        );

        assert_eq!(readiness.overall.status, ReadinessStatus::Fail);
        assert_eq!(
            readiness.overall.blockers,
            [
                "index.duplicate_first_token_ids",
                "submission.duplicate_first_token_ids"
            ]
        );
        let index = readiness.category("index").unwrap();
        assert_eq!(index.status, ReadinessStatus::Fail);
        assert_eq!(index.findings, ["duplicate_first_token_ids"]);
    }

    #[test]
    fn blockers_only_include_blocking_findings_in_failed_categories() {
        let readiness = build_readiness(
            VerdictStatus::Fail,
            &["duplicate_ids".to_string()],
            &[
                finding("duplicate_ids", Severity::Major),
                finding("unsafe_ids", Severity::Major),
            ],
            ReadinessScope::Single,
            None,
        );

        assert_eq!(readiness.overall.status, ReadinessStatus::Fail);
        assert_eq!(
            readiness.overall.blockers,
            ["index.duplicate_ids", "submission.duplicate_ids"]
        );
        let index = readiness.category("index").unwrap();
        assert_eq!(index.status, ReadinessStatus::Fail);
        assert_eq!(index.findings, ["duplicate_ids", "unsafe_ids"]);
    }

    #[test]
    fn terminal_ns_warn_submission_but_do_not_fail_overall_when_gate_passes() {
        let readiness = build_readiness(
            VerdictStatus::Warn,
            &[],
            &[finding("terminal_ns", Severity::Major)],
            ReadinessScope::Single,
            None,
        );

        assert_eq!(readiness.overall.status, ReadinessStatus::Warn);
        assert!(readiness.overall.blockers.is_empty());
        assert_eq!(
            readiness.category("submission").unwrap().status,
            ReadinessStatus::Warn
        );
    }

    #[test]
    fn submission_target_is_attached_to_submission_category() {
        let readiness = build_readiness(
            VerdictStatus::Fail,
            &["reserved_header_chars".to_string()],
            &[finding("reserved_header_chars", Severity::Minor)],
            ReadinessScope::Single,
            Some(crate::submission::SubmissionTarget::Ncbi),
        );

        let submission = readiness.category("submission").unwrap();
        assert_eq!(submission.target.as_deref(), Some("ncbi"));
        assert_eq!(submission.status, ReadinessStatus::Fail);
        assert_eq!(submission.findings, ["reserved_header_chars"]);
    }

    #[test]
    fn submission_findings_warn_when_not_blocking() {
        let readiness = build_readiness(
            VerdictStatus::Warn,
            &[],
            &[finding("long_headers", Severity::Minor)],
            ReadinessScope::Single,
            Some(crate::submission::SubmissionTarget::Generic),
        );

        let submission = readiness.category("submission").unwrap();
        assert_eq!(submission.target.as_deref(), Some("generic"));
        assert_eq!(submission.status, ReadinessStatus::Warn);
        assert!(readiness.overall.blockers.is_empty());
    }

    #[test]
    fn submission_gate_blockers_fail_submission_readiness() {
        let readiness = build_readiness(
            VerdictStatus::Fail,
            &[
                "invalid_chars".to_string(),
                "invalid_fasta_structure".to_string(),
            ],
            &[
                finding("invalid_chars", Severity::Critical),
                finding("invalid_fasta_structure", Severity::Critical),
            ],
            ReadinessScope::Single,
            Some(crate::submission::SubmissionTarget::Generic),
        );

        let submission = readiness.category("submission").unwrap();
        assert_eq!(submission.status, ReadinessStatus::Fail);
        assert_eq!(
            submission.findings,
            ["invalid_chars", "invalid_fasta_structure"]
        );
        assert!(readiness
            .overall
            .blockers
            .contains(&"submission.invalid_chars".to_string()));
        assert!(readiness
            .overall
            .blockers
            .contains(&"submission.invalid_fasta_structure".to_string()));
    }

    #[test]
    fn clean_report_has_machine_and_core_categories_pass() {
        let readiness =
            build_readiness(VerdictStatus::Pass, &[], &[], ReadinessScope::Single, None);

        assert_eq!(readiness.overall.status, ReadinessStatus::Pass);
        for id in [
            "file",
            "structure",
            "alphabet",
            "index",
            "assembly",
            "submission",
            "machine",
        ] {
            assert_eq!(
                readiness.category(id).unwrap().status,
                ReadinessStatus::Pass
            );
        }
        assert!(readiness.category("cohort").is_none());
    }
}
