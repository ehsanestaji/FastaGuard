use crate::cli::RuleConfig;
use crate::metrics::AssemblyMetrics;
use crate::models::{Finding, Severity, VerdictStatus};
use crate::profile::ProfileConfig;

#[derive(Debug, Clone)]
pub struct Analysis {
    pub status: VerdictStatus,
    pub reasons: Vec<String>,
    pub findings: Vec<Finding>,
}

pub fn analyze(metrics: &AssemblyMetrics, profile: &ProfileConfig, rules: &RuleConfig) -> Analysis {
    let findings = build_findings(metrics, profile);
    let status = verdict_status(&findings, rules);
    let reasons = verdict_reasons(&findings, rules, status);

    Analysis {
        status,
        reasons,
        findings,
    }
}

fn build_findings(metrics: &AssemblyMetrics, profile: &ProfileConfig) -> Vec<Finding> {
    let mut findings = Vec::new();

    if metrics.duplicate_id_count > 0 {
        findings.push(finding(
            "duplicate_ids",
            Severity::Critical,
            profile,
            metrics.duplicate_id_count,
            affected_fraction(metrics.duplicate_id_count, metrics.sequence_count),
            FindingText {
                message: format!(
                    "{} duplicate FASTA IDs were found.",
                    metrics.duplicate_id_count
                ),
                why_it_matters:
                    "Duplicate IDs can break indexing, annotation, mapping, and workflow joins.",
                suggested_next_step:
                    "Rename records so every FASTA identifier is unique before running downstream tools.",
            },
        ));
    }

    if metrics.invalid_sequence_count > 0 {
        findings.push(finding(
            "invalid_chars",
            Severity::Critical,
            profile,
            metrics.invalid_sequence_count,
            affected_fraction(metrics.invalid_sequence_count, metrics.sequence_count),
            FindingText {
                message: format!(
                    "{} sequences contain invalid FASTA characters.",
                    metrics.invalid_sequence_count
                ),
                why_it_matters:
                    "Invalid characters can make parsers, aligners, or annotation tools fail or misinterpret records.",
                suggested_next_step:
                    "Inspect and correct non-IUPAC sequence symbols before continuing the workflow.",
            },
        ));
    }

    let global_n_fraction = global_n_fraction(metrics);
    if high_global_n_rate(global_n_fraction, profile) || metrics.high_n_sequence_count > 0 {
        let threshold_percent = profile.high_global_n_fraction * 100.0;
        let global_n_percent = global_n_fraction * 100.0;
        let (affected_count, affected_fraction) = if metrics.high_n_sequence_count > 0 {
            (
                metrics.high_n_sequence_count,
                affected_fraction(metrics.high_n_sequence_count, metrics.sequence_count),
            )
        } else {
            (
                metrics.sequence_count,
                affected_fraction(metrics.sequence_count, metrics.sequence_count),
            )
        };

        findings.push(finding(
            "high_n_rate",
            Severity::Major,
            profile,
            affected_count,
            affected_fraction,
            FindingText {
                message: format!(
                    "Global N is {:.2}% (threshold {:.2}%), and {} sequences exceed the per-sequence N threshold.",
                    global_n_percent, threshold_percent, metrics.high_n_sequence_count
                ),
                why_it_matters:
                    "High N content can reduce mapping confidence and fragment annotation or polishing steps.",
                suggested_next_step:
                    "Review assembly masking, coverage, and contigs with high N content before downstream analysis.",
            },
        ));
    }

    if metrics.tiny_contig_count > 0 {
        findings.push(finding(
            "tiny_contigs",
            Severity::Minor,
            profile,
            metrics.tiny_contig_count,
            affected_fraction(metrics.tiny_contig_count, metrics.sequence_count),
            FindingText {
                message: format!(
                    "{} contigs are shorter than the {} bp profile minimum.",
                    metrics.tiny_contig_count, profile.min_contig_length
                ),
                why_it_matters:
                    "Very short contigs often add noise to assembly statistics and downstream annotation.",
                suggested_next_step:
                    "Filter or review tiny contigs before using the assembly in production workflows.",
            },
        ));
    }

    if metrics.max_gap_run > profile.max_gap_run {
        findings.push(finding(
            "gap_runs",
            Severity::Major,
            profile,
            1,
            affected_fraction(1, metrics.sequence_count),
            FindingText {
                message: format!(
                    "The longest N gap run is {} bp, above the {} bp profile limit.",
                    metrics.max_gap_run, profile.max_gap_run
                ),
                why_it_matters:
                    "Long gap runs can indicate unresolved assembly regions and may disrupt mapping or annotation.",
                suggested_next_step:
                    "Inspect scaffolds with long N runs and consider gap closing or masking review.",
            },
        ));
    }

    if metrics.duplicate_sequence_count > 0 {
        findings.push(finding(
            "duplicate_sequences",
            Severity::Minor,
            profile,
            metrics.duplicate_sequence_count,
            affected_fraction(metrics.duplicate_sequence_count, metrics.sequence_count),
            FindingText {
                message: format!(
                    "{} duplicate sequence records were found.",
                    metrics.duplicate_sequence_count
                ),
                why_it_matters:
                    "Duplicate sequences can inflate assembly metrics and confuse record-level comparisons.",
                suggested_next_step:
                    "Deduplicate repeated sequence records or confirm they are expected before downstream use.",
            },
        ));
    }

    findings
}

fn finding(
    id: &str,
    severity: Severity,
    profile: &ProfileConfig,
    affected_count: u64,
    affected_fraction: f64,
    text: FindingText<'_>,
) -> Finding {
    Finding {
        id: id.to_string(),
        severity,
        profile: profile.name.clone(),
        affected_count,
        affected_fraction,
        message: text.message,
        why_it_matters: text.why_it_matters.to_string(),
        suggested_next_step: text.suggested_next_step.to_string(),
    }
}

struct FindingText<'a> {
    message: String,
    why_it_matters: &'a str,
    suggested_next_step: &'a str,
}

fn affected_fraction(affected_count: u64, sequence_count: u64) -> f64 {
    if sequence_count == 0 {
        0.0
    } else {
        affected_count as f64 / sequence_count as f64
    }
}

fn global_n_fraction(metrics: &AssemblyMetrics) -> f64 {
    let (n_count, total_length) =
        metrics
            .sequences
            .iter()
            .fold((0_u128, 0_u128), |(n_total, length_total), sequence| {
                (
                    n_total + u128::from(sequence.n_count),
                    length_total + u128::from(sequence.length),
                )
            });

    if total_length == 0 {
        0.0
    } else {
        n_count as f64 / total_length as f64
    }
}

fn high_global_n_rate(global_n_fraction: f64, profile: &ProfileConfig) -> bool {
    global_n_fraction > 0.0 && global_n_fraction >= profile.high_global_n_fraction
}

fn verdict_status(findings: &[Finding], rules: &RuleConfig) -> VerdictStatus {
    if findings.iter().any(|finding| {
        matches!(finding.severity, Severity::Critical) || rules.fail_on.contains(&finding.id)
    }) {
        VerdictStatus::Fail
    } else if findings.is_empty() {
        VerdictStatus::Pass
    } else {
        VerdictStatus::Warn
    }
}

fn verdict_reasons(findings: &[Finding], rules: &RuleConfig, status: VerdictStatus) -> Vec<String> {
    let mut reasons = Vec::new();

    for finding in findings {
        let contributes = match status {
            VerdictStatus::Pass => false,
            VerdictStatus::Warn => true,
            VerdictStatus::Fail => {
                matches!(finding.severity, Severity::Critical)
                    || rules.fail_on.contains(&finding.id)
            }
        };

        if contributes && !reasons.contains(&finding.id) {
            reasons.push(finding.id.clone());
        }
    }

    reasons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::SequenceSummary;
    use crate::profile::ThresholdOverrides;

    fn clean_metrics() -> AssemblyMetrics {
        AssemblyMetrics {
            sequence_count: 2,
            total_length: 1_000,
            min_length: 500,
            max_length: 500,
            mean_length: 500.0,
            median_length: 500.0,
            n50: 500,
            n90: 500,
            l50: 1,
            l90: 2,
            gc_percent: 50.0,
            at_percent: 50.0,
            n_percent: 0.0,
            ambiguity_percent: 0.0,
            duplicate_id_count: 0,
            duplicate_sequence_count: 0,
            invalid_sequence_count: 0,
            high_n_sequence_count: 0,
            tiny_contig_count: 0,
            max_gap_run: 0,
            sequences: Vec::new(),
        }
    }

    fn profile() -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: None,
        })
    }

    fn profile_with_max_n_rate(max_n_rate: f64) -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: Some(max_n_rate),
            min_contig_length: None,
        })
    }

    fn rules(fail_on: &[&str]) -> RuleConfig {
        RuleConfig {
            fail_on: fail_on.iter().map(|value| (*value).to_string()).collect(),
        }
    }

    fn sequence_summary(length: u64, n_count: u64) -> SequenceSummary {
        SequenceSummary {
            id: "seq1".to_string(),
            length,
            gc_count: 0,
            at_count: length.saturating_sub(n_count),
            n_count,
            ambiguity_count: 0,
            invalid_count: 0,
            max_gap_run: 0,
            n_fraction: if length == 0 {
                0.0
            } else {
                n_count as f64 / length as f64
            },
            gc_percent: 0.0,
        }
    }

    #[test]
    fn duplicate_ids_can_fail_when_configured() {
        let mut metrics = clean_metrics();
        metrics.duplicate_id_count = 1;

        let analysis = analyze(&metrics, &profile(), &rules(&["duplicate_ids"]));

        assert_eq!(analysis.status, VerdictStatus::Fail);
        assert_eq!(analysis.reasons, ["duplicate_ids"]);
    }

    #[test]
    fn high_n_defaults_to_warning() {
        let mut metrics = clean_metrics();
        metrics.high_n_sequence_count = 1;

        let analysis = analyze(&metrics, &profile(), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Warn);
        assert_eq!(analysis.reasons, ["high_n_rate"]);
    }

    #[test]
    fn clean_metrics_pass_without_reasons() {
        let analysis = analyze(&clean_metrics(), &profile(), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Pass);
        assert!(analysis.reasons.is_empty());
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn fail_on_escalates_non_critical_findings() {
        let mut metrics = clean_metrics();
        metrics.tiny_contig_count = 1;

        let analysis = analyze(&metrics, &profile(), &rules(&["tiny_contigs"]));

        assert_eq!(analysis.status, VerdictStatus::Fail);
        assert_eq!(analysis.reasons, ["tiny_contigs"]);
    }

    #[test]
    fn high_n_uses_exact_global_fraction_below_rounded_boundary() {
        let mut metrics = clean_metrics();
        metrics.sequence_count = 1;
        metrics.total_length = 100_001;
        metrics.n_percent = 5.0;
        metrics.sequences = vec![sequence_summary(100_001, 5_000)];

        let analysis = analyze(&metrics, &profile_with_max_n_rate(0.05), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Pass);
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn clean_metrics_with_zero_max_n_rate_passes() {
        let analysis = analyze(&clean_metrics(), &profile_with_max_n_rate(0.0), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Pass);
        assert!(analysis.reasons.is_empty());
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn global_n_only_high_n_finding_affects_all_sequences() {
        let mut metrics = clean_metrics();
        metrics.n_percent = 6.0;
        metrics.sequences = vec![sequence_summary(500, 30), sequence_summary(500, 30)];

        let analysis = analyze(&metrics, &profile_with_max_n_rate(0.05), &rules(&[]));
        let finding = analysis
            .findings
            .iter()
            .find(|finding| finding.id == "high_n_rate")
            .unwrap();

        assert_eq!(finding.affected_count, metrics.sequence_count);
        assert_eq!(finding.affected_fraction, 1.0);
    }

    #[test]
    fn gap_runs_finding_uses_record_count_semantics() {
        let mut metrics = clean_metrics();
        metrics.max_gap_run = 101;

        let analysis = analyze(&metrics, &profile(), &rules(&[]));
        let finding = analysis
            .findings
            .iter()
            .find(|finding| finding.id == "gap_runs")
            .unwrap();

        assert_eq!(finding.affected_count, 1);
        assert_eq!(
            finding.affected_fraction,
            1.0 / metrics.sequence_count as f64
        );
    }
}
