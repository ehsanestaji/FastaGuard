use crate::cli::RuleConfig;
use crate::metrics::AssemblyMetrics;
use crate::metrics::SequenceSummary;
use crate::models::{
    empty_evidence, finding_actions, EvidenceRecord, Finding, FindingCategory, FindingConfidence,
    FindingEvidence, Severity, VerdictStatus,
};
use crate::profile::ProfileConfig;
use crate::stats::composition::round2;

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
            evidence_for_sequences(
                metrics.duplicate_id_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.duplicate_id),
                "duplicate FASTA identifier",
                EvidenceKind::DuplicateId,
            ),
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

    if metrics.duplicate_first_token_id_count > 0 {
        findings.push(finding(
            "duplicate_first_token_ids",
            Severity::Critical,
            profile,
            metrics.duplicate_first_token_id_count,
            affected_fraction(
                metrics.duplicate_first_token_id_count,
                metrics.sequence_count,
            ),
            evidence_for_sequences(
                metrics.duplicate_first_token_id_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.duplicate_first_token_id),
                "duplicate first whitespace-delimited FASTA identifier",
                EvidenceKind::DuplicateFirstTokenId,
            ),
            FindingText {
                message: format!(
                    "{} duplicate first-token FASTA IDs were found.",
                    metrics.duplicate_first_token_id_count
                ),
                why_it_matters:
                    "Many indexing, mapping, BLAST, and annotation tools treat the first header token as the record name.",
                suggested_next_step:
                    "Rename records so every first-token FASTA identifier is unique before running downstream tools.",
            },
        ));
    }

    if metrics.unsafe_id_count > 0 {
        findings.push(finding(
            "unsafe_ids",
            Severity::Major,
            profile,
            metrics.unsafe_id_count,
            affected_fraction(metrics.unsafe_id_count, metrics.sequence_count),
            evidence_for_sequences(
                metrics.unsafe_id_count,
                metrics.sequences.iter().filter(|sequence| sequence.unsafe_id),
                "FASTA identifier contains characters that may be unsafe in indexes, paths, or tabular joins",
                EvidenceKind::UnsafeId,
            ),
            FindingText {
                message: format!(
                    "{} FASTA IDs contain characters that may be unsafe for downstream tools.",
                    metrics.unsafe_id_count
                ),
                why_it_matters:
                    "Some indexing, database, submission, and workflow tools assume simple portable record identifiers.",
                suggested_next_step:
                    "Review and normalize identifiers before building databases, indexes, or submission packages.",
            },
        ));
    }

    if metrics.long_header_count > 0 {
        findings.push(finding(
            "long_headers",
            Severity::Minor,
            profile,
            metrics.long_header_count,
            affected_fraction(metrics.long_header_count, metrics.sequence_count),
            evidence_for_sequences(
                metrics.long_header_count,
                metrics.sequences.iter().filter(|sequence| sequence.long_header),
                "FASTA header is longer than the compatibility limit",
                EvidenceKind::HeaderCompatibility,
            ),
            FindingText {
                message: format!(
                    "{} FASTA headers are longer than the compatibility limit.",
                    metrics.long_header_count
                ),
                why_it_matters:
                    "Long headers can be truncated or rejected by some database, indexing, and submission workflows.",
                suggested_next_step:
                    "Shorten headers or move descriptive metadata into a sidecar table before downstream use.",
            },
        ));
    }

    if metrics.reserved_header_char_count > 0 {
        findings.push(finding(
            "reserved_header_chars",
            Severity::Minor,
            profile,
            metrics.reserved_header_char_count,
            affected_fraction(metrics.reserved_header_char_count, metrics.sequence_count),
            evidence_for_sequences(
                metrics.reserved_header_char_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.reserved_header_chars),
                "FASTA header contains reserved compatibility characters",
                EvidenceKind::HeaderCompatibility,
            ),
            FindingText {
                message: format!(
                    "{} FASTA headers contain reserved compatibility characters.",
                    metrics.reserved_header_char_count
                ),
                why_it_matters:
                    "Reserved header characters can confuse parsers, database builders, or submission validators.",
                suggested_next_step:
                    "Normalize headers to portable characters before database construction or submission.",
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
            evidence_for_sequences(
                metrics.invalid_sequence_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.invalid_count > 0),
                "invalid sequence symbols",
                EvidenceKind::InvalidChars,
            ),
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
            high_n_evidence(metrics, profile, affected_count),
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
            evidence_for_sequences(
                metrics.tiny_contig_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.length < profile.min_contig_length),
                "shorter than profile minimum contig length",
                EvidenceKind::TinyContig,
            ),
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
        let gap_run_count = metrics
            .sequences
            .iter()
            .filter(|sequence| sequence.max_gap_run > profile.max_gap_run)
            .count() as u64;
        let affected_count = gap_run_count.max(1);

        findings.push(finding(
            "gap_runs",
            Severity::Major,
            profile,
            affected_count,
            affected_fraction(affected_count, metrics.sequence_count),
            evidence_for_sequences(
                gap_run_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.max_gap_run > profile.max_gap_run),
                "long N gap run",
                EvidenceKind::GapRun,
            ),
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

    if metrics.terminal_n_sequence_count > 0 {
        findings.push(finding(
            "terminal_ns",
            Severity::Major,
            profile,
            metrics.terminal_n_sequence_count,
            affected_fraction(metrics.terminal_n_sequence_count, metrics.sequence_count),
            evidence_for_sequences(
                metrics.terminal_n_sequence_count,
                metrics.sequences.iter().filter(|sequence| {
                    sequence.terminal_n_prefix > 0 || sequence.terminal_n_suffix > 0
                }),
                "record has leading or trailing N bases",
                EvidenceKind::TerminalN,
            ),
            FindingText {
                message: format!(
                    "{} records have leading or trailing N bases.",
                    metrics.terminal_n_sequence_count
                ),
                why_it_matters:
                    "Terminal Ns can trigger submission warnings and may indicate records that need trimming or scaffold-boundary review.",
                suggested_next_step:
                    "Review terminal N runs and trim or justify them before submission-oriented workflows.",
            },
        ));
    }

    if metrics.repeated_gap_pattern_sequence_count > 0 {
        findings.push(finding(
            "gap_pattern_warnings",
            Severity::Minor,
            profile,
            metrics.repeated_gap_pattern_sequence_count,
            affected_fraction(
                metrics.repeated_gap_pattern_sequence_count,
                metrics.sequence_count,
            ),
            evidence_for_sequences(
                metrics.repeated_gap_pattern_sequence_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.gap_run_100_count > 0),
                "record contains repeated 100 bp N gap patterns",
                EvidenceKind::GapPattern,
            ),
            FindingText {
                message: format!(
                    "{} records contain repeated 100 bp N gap patterns.",
                    metrics.repeated_gap_pattern_sequence_count
                ),
                why_it_matters:
                    "Repeated gap patterns can be legitimate scaffold placeholders, but they are often worth checking before submission or annotation.",
                suggested_next_step:
                    "Review scaffold gap patterns and confirm they match the intended assembly representation.",
            },
        ));
    }

    if expected_size_outlier(metrics.ungapped_total_length, profile) {
        let expected_size = profile.expected_size_bases.unwrap();
        let tolerance = profile.expected_size_tolerance.unwrap_or(0.0);
        findings.push(finding(
            "expected_size_outlier",
            Severity::Major,
            profile,
            1,
            1.0,
            empty_evidence(),
            FindingText {
                message: format!(
                    "Assembly ungapped total length is {} bp, outside expected size {} bp with {:.2}% tolerance.",
                    metrics.ungapped_total_length,
                    expected_size,
                    tolerance * 100.0
                ),
                why_it_matters:
                    "Unexpected assembly size can indicate missing sequence, extra sequence, contamination, or incorrect expected-size metadata, but FASTA-only checks cannot determine the cause.",
                suggested_next_step:
                    "Review the expected-size input and run submission validation or deeper assembly QC before drawing biological conclusions.",
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
            evidence_for_sequences(
                metrics.duplicate_sequence_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.duplicate_sequence),
                "duplicate sequence content",
                EvidenceKind::DuplicateSequence,
            ),
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

    let gc_outlier_count = metrics
        .sequences
        .iter()
        .filter(|sequence| sequence.gc_outlier)
        .count() as u64;
    if gc_outlier_count > 0 {
        findings.push(finding(
            "gc_outliers",
            Severity::Major,
            profile,
            gc_outlier_count,
            affected_fraction(gc_outlier_count, metrics.sequence_count),
            evidence_for_sequences(
                gc_outlier_count,
                metrics.sequences.iter().filter(|sequence| sequence.gc_outlier),
                "GC composition far from assembly background",
                EvidenceKind::CompositionOutlier,
            ),
            FindingText {
                message: format!(
                    "{} records have GC composition far from the assembly background.",
                    gc_outlier_count
                ),
                why_it_matters:
                    "GC outliers can reflect contamination, cobionts, plasmids, artifacts, or genuine biological variation and need context before interpretation.",
                suggested_next_step:
                    "Inspect the affected records; if the pattern is strong, compare coverage and taxonomy signals with BlobToolKit, sourmash, or Kraken.",
            },
        ));
    }

    let length_outlier_count = metrics
        .sequences
        .iter()
        .filter(|sequence| sequence.length_outlier)
        .count() as u64;
    if length_outlier_count > 0 {
        findings.push(finding(
            "length_outliers",
            Severity::Minor,
            profile,
            length_outlier_count,
            affected_fraction(length_outlier_count, metrics.sequence_count),
            evidence_for_sequences(
                length_outlier_count,
                metrics
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.length_outlier),
                "record length outside the assembly length distribution",
                EvidenceKind::AssemblyOutlier,
            ),
            FindingText {
                message: format!(
                    "{} records have lengths outside the assembly length distribution.",
                    length_outlier_count
                ),
                why_it_matters:
                    "Length outliers can be expected in assemblies, but extreme records deserve inspection before downstream filtering or annotation.",
                suggested_next_step:
                    "Inspect the affected records and confirm whether their size is expected for this assembly.",
            },
        ));
    }

    let composite_anomaly_count = metrics
        .sequences
        .iter()
        .filter(|sequence| sequence.composite_anomaly)
        .count() as u64;
    if composite_anomaly_count > 0 {
        findings.push(finding(
            "composite_anomalies",
            Severity::Major,
            profile,
            composite_anomaly_count,
            affected_fraction(composite_anomaly_count, metrics.sequence_count),
            composite_anomaly_evidence(metrics, profile, composite_anomaly_count),
            FindingText {
                message: format!(
                    "{} records have multiple assembly anomaly signals.",
                    composite_anomaly_count
                ),
                why_it_matters:
                    "Multiple independent FASTA-level signals make a record more likely to need manual review before expensive downstream QC.",
                suggested_next_step:
                    "Prioritize these records for review and compare composition, coverage, and taxonomy context with BlobToolKit when available.",
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
    evidence: FindingEvidence,
    text: FindingText<'_>,
) -> Finding {
    let metadata = finding_metadata(id);
    Finding {
        id: id.to_string(),
        category: metadata.category,
        severity,
        confidence: metadata.confidence,
        requires_followup_tool: metadata.requires_followup_tool,
        profile: profile.name.clone(),
        affected_count,
        affected_fraction,
        message: text.message,
        why_it_matters: text.why_it_matters.to_string(),
        suggested_next_step: text.suggested_next_step.to_string(),
        evidence,
        actions: finding_actions(id),
    }
}

#[derive(Debug, Clone, Copy)]
struct FindingMetadata {
    category: FindingCategory,
    confidence: FindingConfidence,
    requires_followup_tool: bool,
}

fn finding_metadata(id: &str) -> FindingMetadata {
    use FindingCategory::{Composition, Duplication, Structure, Validity};
    use FindingConfidence::{High, Moderate};

    let (category, confidence) = match id {
        "duplicate_ids" | "duplicate_sequences" | "duplicate_first_token_ids" => {
            (Duplication, High)
        }
        "invalid_chars" | "invalid_fasta_structure" => (Validity, High),
        "unsafe_ids" | "long_headers" | "reserved_header_chars" => (Validity, Moderate),
        "high_n_rate" => (Composition, High),
        "tiny_contigs" | "terminal_ns" | "gap_pattern_warnings" | "expected_size_outlier" => {
            (Structure, Moderate)
        }
        "gap_runs" => (Structure, High),
        "gc_outliers" => (Composition, Moderate),
        "length_outliers" => (Structure, Moderate),
        "composite_anomalies" => (Composition, Moderate),
        _ => unreachable!("unknown finding id: {id}"),
    };

    FindingMetadata {
        category,
        confidence,
        requires_followup_tool: matches!(
            id,
            "gc_outliers" | "composite_anomalies" | "expected_size_outlier"
        ),
    }
}

#[cfg(test)]
mod taxonomy_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "unknown finding id: future_finding")]
    fn finding_metadata_panics_on_unknown_ids() {
        finding_metadata("future_finding");
    }

    #[test]
    fn finding_metadata_classifies_current_ids() {
        assert_eq!(
            finding_metadata("duplicate_ids").category,
            FindingCategory::Duplication
        );
        assert_eq!(
            finding_metadata("high_n_rate").category,
            FindingCategory::Composition
        );
        assert_eq!(
            finding_metadata("tiny_contigs").confidence,
            FindingConfidence::Moderate
        );
        assert_eq!(
            finding_metadata("gc_outliers").category,
            FindingCategory::Composition
        );
        assert!(finding_metadata("gc_outliers").requires_followup_tool);
        assert_eq!(
            finding_metadata("length_outliers").category,
            FindingCategory::Structure
        );
        assert_eq!(
            finding_metadata("duplicate_first_token_ids").category,
            FindingCategory::Duplication
        );
        assert_eq!(
            finding_metadata("unsafe_ids").category,
            FindingCategory::Validity
        );
        assert_eq!(
            finding_metadata("terminal_ns").category,
            FindingCategory::Structure
        );
        assert!(finding_metadata("expected_size_outlier").requires_followup_tool);
        assert!(finding_metadata("composite_anomalies").requires_followup_tool);
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

const MAX_EVIDENCE_RECORDS: usize = 20;

#[derive(Debug, Clone, Copy)]
enum EvidenceKind {
    DuplicateId,
    DuplicateFirstTokenId,
    DuplicateSequence,
    InvalidChars,
    HighN,
    TinyContig,
    GapRun,
    UnsafeId,
    HeaderCompatibility,
    TerminalN,
    GapPattern,
    #[allow(dead_code)]
    ExpectedSize,
    CompositionOutlier,
    AssemblyOutlier,
}

fn high_n_evidence(
    metrics: &AssemblyMetrics,
    profile: &ProfileConfig,
    affected_count: u64,
) -> FindingEvidence {
    let high_n_sequences: Vec<&SequenceSummary> = metrics
        .sequences
        .iter()
        .filter(|sequence| sequence.n_fraction >= profile.high_n_sequence_fraction)
        .collect();

    if high_n_sequences.is_empty() {
        return evidence_for_sequences(
            affected_count,
            metrics.sequences.iter(),
            "included because global N rate exceeded threshold",
            EvidenceKind::HighN,
        );
    }

    evidence_for_sequences(
        affected_count,
        high_n_sequences.into_iter(),
        "per-sequence N fraction exceeded threshold",
        EvidenceKind::HighN,
    )
}

fn evidence_for_sequences<'a>(
    total_records: u64,
    sequences: impl Iterator<Item = &'a SequenceSummary>,
    reason: &str,
    kind: EvidenceKind,
) -> FindingEvidence {
    let records: Vec<EvidenceRecord> = sequences
        .take(MAX_EVIDENCE_RECORDS)
        .map(|sequence| evidence_record(sequence, reason, kind))
        .collect();

    FindingEvidence {
        total_records,
        truncated: total_records > records.len() as u64,
        records,
    }
}

fn composite_anomaly_evidence(
    metrics: &AssemblyMetrics,
    profile: &ProfileConfig,
    total_records: u64,
) -> FindingEvidence {
    let records: Vec<EvidenceRecord> = metrics
        .sequences
        .iter()
        .filter(|sequence| sequence.composite_anomaly)
        .take(MAX_EVIDENCE_RECORDS)
        .map(|sequence| {
            let mut record = evidence_record(
                sequence,
                "record has multiple assembly anomaly signals",
                EvidenceKind::AssemblyOutlier,
            );
            record.signals = composite_signals(sequence, profile);
            record
        })
        .collect();

    FindingEvidence {
        total_records,
        truncated: total_records > records.len() as u64,
        records,
    }
}

fn evidence_record(sequence: &SequenceSummary, reason: &str, kind: EvidenceKind) -> EvidenceRecord {
    let mut record = EvidenceRecord {
        id: sequence.id.clone(),
        length: sequence.length,
        reason: reason.to_string(),
        invalid_count: None,
        n_fraction: None,
        n_percent: None,
        max_gap_run: None,
        gc_percent: None,
        gc_zscore: None,
        signals: Vec::new(),
    };

    match kind {
        EvidenceKind::InvalidChars => {
            record.invalid_count = Some(sequence.invalid_count);
        }
        EvidenceKind::HighN => {
            record.n_fraction = Some(round2(sequence.n_fraction));
            record.n_percent = Some(round2(sequence.n_fraction * 100.0));
        }
        EvidenceKind::GapRun => {
            record.max_gap_run = Some(sequence.max_gap_run);
        }
        EvidenceKind::TinyContig => {
            record.gc_percent = Some(sequence.gc_percent);
        }
        EvidenceKind::TerminalN => {
            record.n_fraction = Some(round2(sequence.n_fraction));
            record.n_percent = Some(round2(sequence.n_fraction * 100.0));
            record
                .signals
                .push(format!("terminal_n_prefix={}", sequence.terminal_n_prefix));
            record
                .signals
                .push(format!("terminal_n_suffix={}", sequence.terminal_n_suffix));
        }
        EvidenceKind::GapPattern => {
            record.max_gap_run = Some(sequence.max_gap_run);
            record
                .signals
                .push(format!("gap_run_100_count={}", sequence.gap_run_100_count));
        }
        EvidenceKind::UnsafeId => {
            record.signals.push("unsafe_id".to_string());
        }
        EvidenceKind::HeaderCompatibility => {
            if sequence.long_header {
                record.signals.push("long_header".to_string());
            }
            if sequence.reserved_header_chars {
                record.signals.push("reserved_header_chars".to_string());
            }
        }
        EvidenceKind::CompositionOutlier | EvidenceKind::AssemblyOutlier => {
            record.gc_percent = Some(sequence.gc_percent);
            record.gc_zscore = sequence.gc_zscore;
            record.n_fraction = Some(round2(sequence.n_fraction));
            record.n_percent = Some(round2(sequence.n_fraction * 100.0));
        }
        EvidenceKind::DuplicateId
        | EvidenceKind::DuplicateFirstTokenId
        | EvidenceKind::DuplicateSequence
        | EvidenceKind::ExpectedSize => {}
    }

    record
}

fn expected_size_outlier(ungapped_total_length: u64, profile: &ProfileConfig) -> bool {
    let Some(expected_size) = profile.expected_size_bases else {
        return false;
    };
    let tolerance = profile.expected_size_tolerance.unwrap_or(0.0);
    let lower_bound = expected_size as f64 * (1.0 - tolerance);
    let upper_bound = expected_size as f64 * (1.0 + tolerance);
    let observed = ungapped_total_length as f64;

    observed < lower_bound || observed > upper_bound
}

fn composite_signals(sequence: &SequenceSummary, profile: &ProfileConfig) -> Vec<String> {
    let mut signals = Vec::new();
    if sequence.gc_outlier {
        signals.push("gc_outlier".to_string());
    }
    if sequence.n_fraction >= profile.high_n_sequence_fraction {
        signals.push("high_n".to_string());
    }
    if sequence.length_outlier {
        signals.push("length_outlier".to_string());
    }
    if sequence.duplicate_sequence {
        signals.push("duplicate_sequence".to_string());
    }
    if sequence.invalid_count > 0 {
        signals.push("invalid_chars".to_string());
    }
    if sequence.max_gap_run > profile.max_gap_run {
        signals.push("gap_run".to_string());
    }
    signals
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
            duplicate_first_token_id_count: 0,
            duplicate_sequence_count: 0,
            unsafe_id_count: 0,
            long_header_count: 0,
            reserved_header_char_count: 0,
            invalid_sequence_count: 0,
            high_n_sequence_count: 0,
            tiny_contig_count: 0,
            terminal_n_sequence_count: 0,
            repeated_gap_pattern_sequence_count: 0,
            max_gap_run: 0,
            ungapped_total_length: 1_000,
            sequences: Vec::new(),
        }
    }

    fn profile() -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: None,
            expected_size_bases: None,
            expected_size_tolerance: None,
        })
    }

    fn profile_with_max_n_rate(max_n_rate: f64) -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: Some(max_n_rate),
            min_contig_length: None,
            expected_size_bases: None,
            expected_size_tolerance: None,
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
            header: "seq1".to_string(),
            first_token_id: "seq1".to_string(),
            duplicate_id: false,
            duplicate_first_token_id: false,
            duplicate_sequence: false,
            unsafe_id: false,
            long_header: false,
            reserved_header_chars: false,
            length,
            gc_count: 0,
            at_count: length.saturating_sub(n_count),
            n_count,
            ambiguity_count: 0,
            invalid_count: 0,
            max_gap_run: 0,
            terminal_n_prefix: 0,
            terminal_n_suffix: 0,
            gap_run_100_count: 0,
            n_fraction: if length == 0 {
                0.0
            } else {
                n_count as f64 / length as f64
            },
            gc_percent: 0.0,
            gc_outlier: false,
            length_outlier: false,
            composite_anomaly: false,
            gc_zscore: None,
        }
    }

    fn sequence_summary_with_id(id: &str, length: u64, n_count: u64) -> SequenceSummary {
        let mut sequence = sequence_summary(length, n_count);
        sequence.id = id.to_string();
        sequence.header = id.to_string();
        sequence.first_token_id = id.to_string();
        sequence
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
    fn duplicate_first_token_ids_are_critical_findings() {
        let mut metrics = clean_metrics();
        metrics.sequence_count = 2;
        metrics.duplicate_first_token_id_count = 1;
        metrics.sequences = vec![
            sequence_summary_with_id("contig1", 100, 0),
            sequence_summary_with_id("contig1", 100, 0),
        ];
        metrics.sequences[1].duplicate_first_token_id = true;

        let analysis = analyze(&metrics, &profile(), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Fail);
        assert_eq!(analysis.reasons, ["duplicate_first_token_ids"]);
        assert_eq!(analysis.findings[0].id, "duplicate_first_token_ids");
        assert_eq!(analysis.findings[0].severity, Severity::Critical);
    }

    #[test]
    fn terminal_ns_warn_and_include_prefix_suffix_evidence() {
        let mut metrics = clean_metrics();
        metrics.sequence_count = 1;
        metrics.terminal_n_sequence_count = 1;
        metrics.sequences = vec![sequence_summary_with_id("edge_n", 10, 2)];
        metrics.sequences[0].terminal_n_prefix = 1;
        metrics.sequences[0].terminal_n_suffix = 1;

        let analysis = analyze(&metrics, &profile(), &rules(&[]));

        assert_eq!(analysis.status, VerdictStatus::Warn);
        let finding = analysis
            .findings
            .iter()
            .find(|finding| finding.id == "terminal_ns")
            .unwrap();
        assert_eq!(finding.affected_count, 1);
        assert!(finding.why_it_matters.contains("submission"));
    }

    #[test]
    fn expected_size_outlier_uses_ungapped_length() {
        let mut metrics = clean_metrics();
        metrics.total_length = 1_100_000;
        metrics.ungapped_total_length = 1_000_000;
        let profile = ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: None,
            expected_size_bases: Some(500_000),
            expected_size_tolerance: Some(0.10),
        });

        let analysis = analyze(&metrics, &profile, &rules(&[]));

        assert!(analysis
            .findings
            .iter()
            .any(|finding| finding.id == "expected_size_outlier"));
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
