# FastaGuard v0.4 Preflight Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build v0.4 preflight readiness and compare mode so FastaGuard can rank, gate, and route many assembly FASTA files before downstream QC tools run.

**Architecture:** Extend the existing single-file assembly pipeline first, then build compare mode as a thin orchestration layer over the same analysis/report contract. Keep metrics, findings, readiness, compare aggregation, and report writers in separate modules so each piece can be tested without loading whole FASTA files or parsing HTML.

**Tech Stack:** Rust, clap, serde/serde_json, schemars-style hand-maintained JSON Schema, cargo integration tests, Python unittest adoption checks, inline SVG HTML reports, standard MultiQC custom-content JSON.

---

## Scope Guard

This plan implements the approved spec:

```text
docs/superpowers/specs/2026-06-02-fastaguard-v0.4-preflight-readiness-design.md
```

Keep v0.4 assembly-first and database-free. Do not add transcriptome, protein,
reference-panel, external taxonomy, read mapping, aligners, online APIs, or LLM
summaries.

## File Map

Create:

- `src/readiness.rs`: readiness category/status model and aggregation from findings/gate/report completeness.
- `src/compare.rs`: compare command orchestration and cohort-level summary/finding logic.
- `src/report/compare_html.rs`: self-contained compare HTML writer.
- `src/report/compare_tsv.rs`: compare one-row-per-sample TSV writer.
- `src/report/compare_multiqc.rs`: compare MultiQC custom-content writer.
- `docs/preflight-readiness.md`: product/user documentation for the pre-QC readiness layer.
- `docs/compare-mode.md`: compare command usage and output contract.
- `docs/value-benchmark.md`: measured value and savings scenarios.
- `docs/releases/v0.4.0.md`: release notes draft.
- `testdata/readiness_headers.fa`: fixture for identifier/header readiness findings.
- `testdata/readiness_terminal_ns.fa`: fixture for terminal-N and gap-pattern findings.
- `testdata/compare_pass.fa`: compare PASS fixture.
- `testdata/compare_warn.fa`: compare WARN fixture.
- `testdata/compare_fail.fa`: compare FAIL fixture.
- `tests/golden/compare_mixed_status.json`: compare report golden.
- `tests/golden/compare_all_pass.json`: compare report golden.

Modify:

- `Cargo.toml`: bump crate version only when release preparation begins, not in early feature commits.
- `schema/fastaguard.schema.json`: bump contract to `0.4.0`, add readiness and compare shapes.
- `schema/finding-catalog.json`: add new finding metadata/actions/scope.
- `src/lib.rs`: route contract flags, single-file run, and compare subcommand.
- `src/cli.rs`: introduce clap subcommands, compare config, expected-size parsing.
- `src/gate.rs`: add `duplicate_first_token_ids` to pipeline fail set.
- `src/metrics.rs`: collect header/ID/readiness signals and expected-size context.
- `src/findings.rs`: create new findings from metrics and expected-size config.
- `src/models.rs`: add `readiness`, new summary counters, compare report models.
- `src/report/html.rs`: render single-file readiness section.
- `src/report/tsv.rs`: add readiness rows and new summary counters.
- `src/report/multiqc.rs`: add readiness fields to single-file custom content.
- `src/report/mod.rs`: add compare output validation/write entrypoint.
- `tests/cli.rs`: add CLI integration tests for readiness fields, findings, compare, and exit codes.
- `tests/schema_contract.rs`: assert schema/catalog v0.4 fields.
- `tests/python/test_adoption_assets.py`: docs/example/MultiQC adoption checks.
- `README.md`, `docs/tool-landscape.md`, `docs/benchmarking.md`, `docs/output-contract.md`, `docs/roadmap.md`: document v0.4 positioning and outputs.

---

## Task 1: Add Readiness Models And Aggregation

**Files:**

- Create: `src/readiness.rs`
- Modify: `src/lib.rs`
- Modify: `src/models.rs`
- Test: `src/readiness.rs`

- [ ] **Step 1: Write failing readiness unit tests**

Add `src/readiness.rs` with tests first:

```rust
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
        );

        assert_eq!(readiness.overall.status, ReadinessStatus::Fail);
        assert_eq!(
            readiness.overall.blockers,
            ["index.duplicate_first_token_ids"]
        );
        let index = readiness.category("index").unwrap();
        assert_eq!(index.status, ReadinessStatus::Fail);
        assert_eq!(index.findings, ["duplicate_first_token_ids"]);
    }

    #[test]
    fn terminal_ns_warn_submission_but_do_not_fail_overall_when_gate_passes() {
        let readiness = build_readiness(
            VerdictStatus::Warn,
            &[],
            &[finding("terminal_ns", Severity::Major)],
            ReadinessScope::Single,
        );

        assert_eq!(readiness.overall.status, ReadinessStatus::Warn);
        assert!(readiness.overall.blockers.is_empty());
        assert_eq!(
            readiness.category("submission").unwrap().status,
            ReadinessStatus::Warn
        );
    }

    #[test]
    fn clean_report_has_machine_and_core_categories_pass() {
        let readiness = build_readiness(VerdictStatus::Pass, &[], &[], ReadinessScope::Single);

        assert_eq!(readiness.overall.status, ReadinessStatus::Pass);
        for id in ["file", "structure", "alphabet", "index", "assembly", "submission", "machine"] {
            assert_eq!(readiness.category(id).unwrap().status, ReadinessStatus::Pass);
        }
        assert!(readiness.category("cohort").is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --locked readiness::tests -- --nocapture
```

Expected: FAIL because `src/readiness.rs`, `ReadinessStatus`, `ReadinessScope`, and `build_readiness` do not exist.

- [ ] **Step 3: Implement readiness models**

Create `src/readiness.rs`:

```rust
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessOverall {
    pub status: ReadinessStatus,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessCategory {
    pub id: String,
    pub label: String,
    pub status: ReadinessStatus,
    pub findings: Vec<String>,
}

pub fn build_readiness(
    verdict: VerdictStatus,
    blocking_findings: &[String],
    findings: &[Finding],
    scope: ReadinessScope,
) -> ReadinessReport {
    let mut categories = base_categories(scope);
    for finding in findings {
        for category_id in category_ids_for_finding(&finding.id) {
            if let Some(category) = categories.iter_mut().find(|category| category.id == category_id)
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
            }
        }
    }

    let blockers = categories
        .iter()
        .filter(|category| category.status == ReadinessStatus::Fail)
        .flat_map(|category| {
            category
                .findings
                .iter()
                .map(move |finding| format!("{}.{}", category.id, finding))
        })
        .collect::<Vec<_>>();
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
            status: ReadinessStatus::Pass,
            findings: Vec::new(),
        })
        .collect()
}

fn category_ids_for_finding(id: &str) -> &'static [&'static str] {
    match id {
        "invalid_fasta_structure" => &["file", "structure"],
        "invalid_chars" => &["alphabet"],
        "duplicate_ids" | "duplicate_first_token_ids" => &["index"],
        "unsafe_ids" | "long_headers" | "reserved_header_chars" => &["index", "submission"],
        "high_n_rate" | "gap_runs" | "tiny_contigs" | "gc_outliers" | "length_outliers"
        | "composite_anomalies" | "gap_pattern_warnings" | "expected_size_outlier" => {
            &["assembly"]
        }
        "terminal_ns" => &["assembly", "submission"],
        "cohort_total_length_outliers" | "cohort_gc_outliers" | "cohort_n_percent_outliers"
        | "cohort_sequence_count_outliers" | "cohort_n50_outliers" => &["cohort"],
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
```

Modify `src/lib.rs`:

```rust
pub mod readiness;
```

- [ ] **Step 4: Add readiness to single-file report model**

Modify `src/models.rs`:

```rust
use crate::readiness::{self, ReadinessReport, ReadinessScope};
```

Add field to `FastaguardReport` after `gate`:

```rust
pub readiness: ReadinessReport,
```

In `from_analysis`, after `gate` construction, avoid computing it twice by binding:

```rust
let gate = gate::decision(
    config.gate_mode,
    analysis.status,
    &findings,
    &config.rules.fail_on,
);
let readiness = readiness::build_readiness(
    analysis.status,
    &gate.blocking_findings,
    &findings,
    ReadinessScope::Single,
);
```

Then set:

```rust
gate,
readiness,
```

In `from_invalid_fasta`, build the gate/readiness the same way with `VerdictStatus::Fail`.

Update all test report builders in `src/report/mod.rs`, `src/report/html.rs`, `src/report/tsv.rs`, and `src/report/multiqc.rs` to include:

```rust
readiness: crate::readiness::build_readiness(
    VerdictStatus::Pass,
    &[],
    &[],
    crate::readiness::ReadinessScope::Single,
),
```

- [ ] **Step 5: Run readiness tests**

Run:

```bash
cargo test --locked readiness::tests
cargo test --locked models::tests report:: -- --nocapture
```

Expected: PASS for readiness tests; any compile failures point to test report builders missing the new `readiness` field.

- [ ] **Step 6: Commit**

```bash
git add src/readiness.rs src/lib.rs src/models.rs src/report/mod.rs src/report/html.rs src/report/tsv.rs src/report/multiqc.rs
git commit -m "feat: add readiness report model"
```

---

## Task 2: Extend Metrics For Header, Index, Terminal-N, Gap, And Expected-Size Signals

**Files:**

- Modify: `src/metrics.rs`
- Modify: `src/cli.rs`
- Modify: `src/profile.rs`
- Test: `src/metrics.rs`
- Test: `src/cli.rs`

- [ ] **Step 1: Write failing metrics tests**

Add tests to `src/metrics.rs`:

```rust
#[test]
fn records_header_and_index_readiness_signals() {
    let metrics = AssemblyMetrics::from_records(
        vec![
            FastaRecord {
                id: "contig1".into(),
                header: "contig1 length=1000".into(),
                sequence: b"ACGT".to_vec(),
            },
            FastaRecord {
                id: "contig1".into(),
                header: "contig1 length=2000".into(),
                sequence: b"TGCA".to_vec(),
            },
            FastaRecord {
                id: "unsafe/path".into(),
                header: "unsafe/path with|pipe".into(),
                sequence: b"ACGT".to_vec(),
            },
        ],
        &profile(),
    );

    assert_eq!(metrics.duplicate_first_token_id_count, 1);
    assert_eq!(metrics.unsafe_id_count, 1);
    assert_eq!(metrics.reserved_header_char_count, 1);
    assert!(metrics.sequences[1].duplicate_first_token_id);
    assert!(metrics.sequences[2].unsafe_id);
    assert!(metrics.sequences[2].reserved_header_chars);
}

#[test]
fn detects_terminal_ns_and_gap_pattern_counts() {
    let metrics = AssemblyMetrics::from_records(
        vec![
            FastaRecord {
                id: "terminal".into(),
                header: "terminal".into(),
                sequence: b"NACGTN".to_vec(),
            },
            FastaRecord {
                id: "gap100".into(),
                header: "gap100".into(),
                sequence: format!("AAA{}TTT", "N".repeat(100)).into_bytes(),
            },
        ],
        &profile(),
    );

    assert_eq!(metrics.terminal_n_sequence_count, 1);
    assert_eq!(metrics.repeated_gap_pattern_sequence_count, 1);
    assert_eq!(metrics.sequences[0].terminal_n_prefix, 1);
    assert_eq!(metrics.sequences[0].terminal_n_suffix, 1);
    assert_eq!(metrics.sequences[1].gap_run_100_count, 1);
}
```

Add tests to `src/cli.rs`:

```rust
#[test]
fn expected_size_parses_decimal_units() {
    let cli = Cli::parse_from([
        "fastaguard",
        "input.fa",
        "--expected-size",
        "5mb",
        "--expected-size-tolerance",
        "0.25",
    ]);
    let config = cli.to_run_config().unwrap();

    assert_eq!(config.thresholds.expected_size_bases, Some(5_000_000));
    assert_eq!(config.thresholds.expected_size_tolerance, Some(0.25));
}

#[test]
fn expected_size_rejects_unknown_units() {
    let cli = Cli::parse_from(["fastaguard", "input.fa", "--expected-size", "5mib"]);
    let error = cli.to_run_config().unwrap_err();

    assert!(error.to_string().contains("--expected-size accepts bases, kb, mb, or gb"));
}
```

- [ ] **Step 2: Run targeted tests to verify failure**

Run:

```bash
cargo test --locked metrics::tests::records_header_and_index_readiness_signals -- --nocapture
cargo test --locked metrics::tests::detects_terminal_ns_and_gap_pattern_counts -- --nocapture
cargo test --locked cli::tests::expected_size_parses_decimal_units -- --nocapture
```

Expected: FAIL because metrics fields and expected-size parsing do not exist.

- [ ] **Step 3: Add expected-size threshold fields and parser**

Modify `src/profile.rs` `ThresholdOverrides`:

```rust
pub expected_size_bases: Option<u64>,
pub expected_size_tolerance: Option<f64>,
```

Modify default test builders to set both fields to `None`.

Modify `src/cli.rs`:

```rust
#[arg(long, value_name = "SIZE")]
pub expected_size: Option<String>,

#[arg(long, default_value_t = 0.25)]
pub expected_size_tolerance: f64,
```

Add parser:

```rust
fn parse_expected_size(value: &str) -> Result<u64> {
    let normalized = value.trim().to_ascii_lowercase();
    let (number, multiplier) = if let Some(number) = normalized.strip_suffix("kb") {
        (number, 1_000_u64)
    } else if let Some(number) = normalized.strip_suffix('k') {
        (number, 1_000_u64)
    } else if let Some(number) = normalized.strip_suffix("mb") {
        (number, 1_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix('m') {
        (number, 1_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix("gb") {
        (number, 1_000_000_000_u64)
    } else if let Some(number) = normalized.strip_suffix('g') {
        (number, 1_000_000_000_u64)
    } else if normalized.chars().all(|ch| ch.is_ascii_digit()) {
        (normalized.as_str(), 1_u64)
    } else {
        return Err(anyhow!(
            "--expected-size accepts bases, kb, mb, or gb decimal units"
        ));
    };
    let parsed = number.parse::<u64>().map_err(|_| {
        anyhow!("--expected-size accepts bases, kb, mb, or gb decimal units")
    })?;
    parsed
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow!("--expected-size is too large"))
}
```

In `to_run_config`, validate tolerance:

```rust
if !self.expected_size_tolerance.is_finite() || self.expected_size_tolerance < 0.0 {
    return Err(anyhow!("--expected-size-tolerance must be finite and non-negative"));
}
let expected_size_bases = self
    .expected_size
    .as_deref()
    .map(parse_expected_size)
    .transpose()?;
```

Set thresholds:

```rust
expected_size_bases,
expected_size_tolerance: expected_size_bases.map(|_| self.expected_size_tolerance),
```

- [ ] **Step 4: Extend metrics structs and builders**

Modify `src/metrics.rs` `SequenceSummary`:

```rust
pub header: String,
pub first_token_id: String,
pub duplicate_first_token_id: bool,
pub unsafe_id: bool,
pub long_header: bool,
pub reserved_header_chars: bool,
pub terminal_n_prefix: u64,
pub terminal_n_suffix: u64,
pub gap_run_100_count: u64,
```

Modify `AssemblyMetrics`:

```rust
pub duplicate_first_token_id_count: u64,
pub unsafe_id_count: u64,
pub long_header_count: u64,
pub reserved_header_char_count: u64,
pub terminal_n_sequence_count: u64,
pub repeated_gap_pattern_sequence_count: u64,
pub ungapped_total_length: u64,
```

Modify `MetricsAccumulator`:

```rust
seen_first_token_ids: BTreeSet<String>,
duplicate_first_token_id_count: u64,
unsafe_id_count: u64,
long_header_count: u64,
reserved_header_char_count: u64,
terminal_n_sequence_count: u64,
repeated_gap_pattern_sequence_count: u64,
ungapped_total: u128,
```

Change `start_record` signature:

```rust
fn start_record(&mut self, id: String, header: String)
```

Call it from `from_records`:

```rust
accumulator.start_record(record.id, record.header);
```

Call it from `from_path`:

```rust
FastaEvent::StartRecord { id, header, .. } => accumulator.start_record(id, header),
```

Add helper functions:

```rust
fn unsafe_id(id: &str) -> bool {
    id.trim() != id || id.chars().any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
}

fn reserved_header_chars(header: &str) -> bool {
    header
        .chars()
        .any(|ch| matches!(ch, '|' | ';' | '"' | '\'' | '`' | '<' | '>' | '\t'))
}
```

Track terminal Ns in `SequenceSummaryBuilder` with:

```rust
first_base: Option<u8>,
last_base: Option<u8>,
leading_n_count: u64,
trailing_n_count: u64,
current_gap_run_length: u64,
gap_run_100_count: u64,
```

When adding an uppercase byte:

```rust
if self.first_base.is_none() {
    self.first_base = Some(upper);
}
self.last_base = Some(upper);
if upper == b'N' && self.length == self.leading_n_count {
    self.leading_n_count += 1;
}
if upper == b'N' {
    self.trailing_n_count += 1;
} else {
    self.trailing_n_count = 0;
}
```

When an N run ends or the builder finishes, count exactly-100 runs:

```rust
if self.current_gap_run_length == 100 {
    self.gap_run_100_count += 1;
}
```

- [ ] **Step 5: Run targeted tests**

Run:

```bash
cargo test --locked metrics::tests::records_header_and_index_readiness_signals
cargo test --locked metrics::tests::detects_terminal_ns_and_gap_pattern_counts
cargo test --locked cli::tests::expected_size
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/metrics.rs src/cli.rs src/profile.rs
git commit -m "feat: collect readiness metrics"
```

---

## Task 3: Add New Single-File Findings And Pipeline Gate Behavior

**Files:**

- Modify: `src/findings.rs`
- Modify: `src/models.rs`
- Modify: `src/gate.rs`
- Modify: `schema/finding-catalog.json`
- Test: `src/findings.rs`
- Test: `src/cli.rs`
- Test: `src/contract.rs`

- [ ] **Step 1: Write failing finding tests**

Add tests to `src/findings.rs`:

```rust
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
```

Add helper:

```rust
fn sequence_summary_with_id(id: &str, length: u64, n_count: u64) -> SequenceSummary {
    let mut sequence = sequence_summary(length, n_count);
    sequence.id = id.to_string();
    sequence.header = id.to_string();
    sequence.first_token_id = id.to_string();
    sequence
}
```

- [ ] **Step 2: Write failing gate test**

Update `src/cli.rs` `gate_pipeline_adds_conservative_fail_rules` expected set:

```rust
[
    "duplicate_first_token_ids",
    "duplicate_ids",
    "high_n_rate",
    "invalid_chars",
    "invalid_fasta_structure",
]
```

- [ ] **Step 3: Run tests to verify failure**

Run:

```bash
cargo test --locked findings::tests::duplicate_first_token_ids_are_critical_findings -- --nocapture
cargo test --locked findings::tests::terminal_ns_warn_and_include_prefix_suffix_evidence -- --nocapture
cargo test --locked cli::tests::gate_pipeline_adds_conservative_fail_rules -- --nocapture
```

Expected: FAIL because findings and gate IDs are missing.

- [ ] **Step 4: Implement new findings**

Modify `src/findings.rs` `build_findings` after duplicate IDs:

```rust
if metrics.duplicate_first_token_id_count > 0 {
    findings.push(finding(
        "duplicate_first_token_ids",
        Severity::Critical,
        profile,
        metrics.duplicate_first_token_id_count,
        affected_fraction(metrics.duplicate_first_token_id_count, metrics.sequence_count),
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
```

Add blocks for:

```text
unsafe_ids
long_headers
reserved_header_chars
terminal_ns
gap_pattern_warnings
expected_size_outlier
```

Use these severities:

```text
unsafe_ids = Major
long_headers = Minor
reserved_header_chars = Minor
terminal_ns = Major
gap_pattern_warnings = Minor
expected_size_outlier = Major
```

Extend `EvidenceKind`:

```rust
DuplicateFirstTokenId,
UnsafeId,
HeaderCompatibility,
TerminalN,
GapPattern,
ExpectedSize,
```

For `ExpectedSize`, use `empty_evidence()` plus message values because it is a whole-assembly signal. The message must include expected size, tolerance, and ungapped total length.

- [ ] **Step 5: Update metadata, actions, and routing**

Modify `finding_metadata`:

```rust
"duplicate_first_token_ids" => (Duplication, High),
"unsafe_ids" | "long_headers" | "reserved_header_chars" => (Validity, Moderate),
"terminal_ns" | "gap_pattern_warnings" => (Structure, Moderate),
"expected_size_outlier" => (Structure, Moderate),
```

Modify `finding_actions` in `src/models.rs` with explicit actions:

```rust
"duplicate_first_token_ids" => vec![action(
    "rename_records",
    "first-token FASTA identifiers",
    "Tools that index by first token can retrieve or annotate the wrong record when first-token IDs collide.",
    "seqkit",
    false,
)]
```

Add actions for header compatibility, terminal Ns, gap patterns, and expected size. For expected size, recommend official validator/deeper QC:

```rust
"expected_size_outlier" => vec![action(
    "review_expected_size",
    "assembly ungapped total length",
    "Unexpected assembly size can indicate missing sequence, extra sequence, contamination, or incorrect expected-size metadata.",
    "NCBI expected genome size check",
    true,
)]
```

Modify `routing_hints` for the new IDs:

```rust
"duplicate_first_token_ids" => push_routing_hint(&mut hints, "index_readiness_failure", "rename_records_before_indexing", false),
"unsafe_ids" | "long_headers" | "reserved_header_chars" => push_routing_hint(&mut hints, "header_compatibility_warning", "review_headers_before_database_or_submission", false),
"terminal_ns" | "gap_pattern_warnings" => push_routing_hint(&mut hints, "submission_readiness_warning", "review_gap_and_terminal_n_patterns", false),
"expected_size_outlier" => push_routing_hint(&mut hints, "expected_size_warning", "run_submission_or_contamination_followup", true),
```

- [ ] **Step 6: Update pipeline gate**

Modify `src/gate.rs` default pipeline set:

```rust
const PIPELINE_FAIL_ON: &[&str] = &[
    "duplicate_first_token_ids",
    "duplicate_ids",
    "high_n_rate",
    "invalid_chars",
    "invalid_fasta_structure",
];
```

- [ ] **Step 7: Update finding catalog**

Modify `schema/finding-catalog.json` and add entries for:

```text
duplicate_first_token_ids
unsafe_ids
long_headers
reserved_header_chars
terminal_ns
gap_pattern_warnings
expected_size_outlier
```

Each entry must include:

```json
"id": "duplicate_first_token_ids",
"category": "duplication",
"severity": "critical",
"confidence": "high",
"requires_followup_tool": false,
"description": "First whitespace-delimited FASTA identifiers collide.",
"why_it_matters": "Many downstream tools index records by the first header token.",
"suggested_actions": [
  {
    "action_type": "rename_records",
    "target": "first-token FASTA identifiers",
    "reason": "First-token collisions can make indexes, annotation joins, and BLAST databases ambiguous.",
    "recommended_tool": "seqkit",
    "requires_external_database": false
  }
]
```

- [ ] **Step 8: Run tests**

Run:

```bash
cargo test --locked findings::tests
cargo test --locked cli::tests::gate_pipeline_adds_conservative_fail_rules
cargo test --locked contract::tests
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src/findings.rs src/models.rs src/gate.rs src/cli.rs schema/finding-catalog.json
git commit -m "feat: add preflight readiness findings"
```

---

## Task 4: Render Readiness In Single-File JSON, TSV, HTML, MultiQC, Schema, And Goldens

**Files:**

- Modify: `src/report/html.rs`
- Modify: `src/report/tsv.rs`
- Modify: `src/report/multiqc.rs`
- Modify: `schema/fastaguard.schema.json`
- Modify: `tests/schema_contract.rs`
- Modify: `tests/golden/*.json`
- Test: `tests/cli.rs`
- Test: `tests/schema_contract.rs`

- [ ] **Step 1: Write failing CLI/schema tests**

Add to `tests/cli.rs`:

```rust
#[test]
fn report_includes_readiness_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let json = temp.path().join("report.json");
    let html = temp.path().join("report.html");
    let tsv = temp.path().join("report.tsv");
    let multiqc = temp.path().join("report_mqc.json");

    Command::cargo_bin("fastaguard")
        .unwrap()
        .args([
            "testdata/problem_assembly.fa",
            "--gate",
            "pipeline",
            "--json",
            json.to_str().unwrap(),
            "--out",
            html.to_str().unwrap(),
            "--tsv",
            tsv.to_str().unwrap(),
            "--multiqc",
            multiqc.to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(report["readiness"]["overall"]["status"], "FAIL");
    assert!(report["readiness"]["categories"].as_array().unwrap().iter().any(|category| {
        category["id"] == "index" && category["status"] == "FAIL"
    }));
    assert!(std::fs::read_to_string(html).unwrap().contains("Readiness"));
    assert!(std::fs::read_to_string(tsv).unwrap().contains("readiness_status\tFAIL"));
}
```

Add to `tests/schema_contract.rs`:

```rust
#[test]
fn schema_requires_readiness_for_single_reports() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();

    assert!(schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "readiness"));
    assert_eq!(schema["properties"]["schema_version"]["const"], "0.4.0");
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --locked --test cli report_includes_readiness_matrix -- --nocapture
cargo test --locked --test schema_contract schema_requires_readiness_for_single_reports -- --nocapture
```

Expected: FAIL until writers/schema are updated.

- [ ] **Step 3: Update TSV**

Modify `src/report/tsv.rs` after gate rows:

```rust
write_metric(
    &mut writer,
    "readiness_status",
    readiness_status(report.readiness.overall.status),
)?;
write_metric(
    &mut writer,
    "readiness_blockers",
    report.readiness.overall.blockers.join(","),
)?;
for category in &report.readiness.categories {
    write_metric(
        &mut writer,
        &format!("readiness_{}_status", category.id),
        readiness_status(category.status),
    )?;
}
```

Add helper:

```rust
fn readiness_status(status: crate::readiness::ReadinessStatus) -> &'static str {
    match status {
        crate::readiness::ReadinessStatus::Pass => "PASS",
        crate::readiness::ReadinessStatus::Warn => "WARN",
        crate::readiness::ReadinessStatus::Fail => "FAIL",
    }
}
```

- [ ] **Step 4: Update HTML**

Modify `src/report/html.rs`:

Add CSS:

```css
.readiness-table td.status-pass { color: #1f7a3f; font-weight: 700; }
.readiness-table td.status-warn { color: #9a6a00; font-weight: 700; }
.readiness-table td.status-fail { color: #a32020; font-weight: 700; }
```

Insert after gate:

```rust
let readiness = render_readiness(report);
```

Template:

```html
<section>
<h2>Readiness</h2>
{readiness}
</section>
```

Helper:

```rust
fn render_readiness(report: &FastaguardReport) -> String {
    let rows = report
        .readiness
        .categories
        .iter()
        .map(|category| {
            let status = readiness_status(category.status);
            format!(
                r#"<tr><td>{label}</td><td class="status-{class}">{status}</td><td>{findings}</td></tr>"#,
                label = escape_html(&category.label),
                class = status.to_ascii_lowercase(),
                status = status,
                findings = render_string_list_or_none(&category.findings),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<p><span class="label">Overall:</span> {overall}</p>
<table class="readiness-table"><thead><tr><th>Category</th><th>Status</th><th>Findings</th></tr></thead><tbody>{rows}</tbody></table>"#,
        overall = readiness_status(report.readiness.overall.status),
        rows = rows,
    )
}
```

- [ ] **Step 5: Update MultiQC**

Modify `src/report/multiqc.rs` `MultiqcSummaryRow`:

```rust
readiness_status: String,
readiness_blockers: String,
```

Set:

```rust
readiness_status: readiness_status(report.readiness.overall.status).to_string(),
readiness_blockers: report.readiness.overall.blockers.join(","),
```

Add headers to pconfig:

```rust
("readiness_status", "Readiness"),
("readiness_blockers", "Readiness blockers"),
```

- [ ] **Step 6: Update schema version and readiness schema**

Modify `src/models.rs`:

```rust
pub const SCHEMA_VERSION: &str = "0.4.0";
```

Modify `schema/fastaguard.schema.json`:

- Set `schema_version.const` to `0.4.0`.
- Add `"readiness"` to top-level `required`.
- Add top-level property:

```json
"readiness": {
  "$ref": "#/$defs/readiness_report"
}
```

Add `$defs`:

```json
"readiness_report": {
  "type": "object",
  "required": ["overall", "categories"],
  "properties": {
    "overall": { "$ref": "#/$defs/readiness_overall" },
    "categories": {
      "type": "array",
      "items": { "$ref": "#/$defs/readiness_category" }
    }
  },
  "additionalProperties": false
}
```

Define `readiness_status` enum as `PASS`, `WARN`, `FAIL`.

- [ ] **Step 7: Regenerate goldens**

Run:

```bash
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-05-23T00:00:00Z cargo test --locked --test cli golden_reports_match -- --nocapture
```

If the test writes runtime files only, regenerate using the repo's existing golden workflow from `tests/cli.rs`. Then copy updated JSON into:

```text
tests/golden/valid_assembly.json
tests/golden/problem_assembly.json
tests/golden/invalid_empty_record.json
examples/reports/assembly_pass/fastaguard.json
examples/reports/assembly_fail/fastaguard.json
```

Do not hand-edit generated JSON except for deterministic path/timestamp conventions already used by the test suite.

- [ ] **Step 8: Run tests**

Run:

```bash
cargo test --locked --test cli report_includes_readiness_matrix
cargo test --locked --test schema_contract
cargo test --locked report::tsv report::html report::multiqc
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src/models.rs src/report/html.rs src/report/tsv.rs src/report/multiqc.rs schema/fastaguard.schema.json tests/schema_contract.rs tests/cli.rs tests/golden examples/reports
git commit -m "feat: expose readiness in reports"
```

---

## Task 5: Add Compare CLI Configuration

**Files:**

- Modify: `src/cli.rs`
- Modify: `src/lib.rs`
- Create: `src/compare.rs`
- Test: `src/cli.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Add to `src/cli.rs` tests:

```rust
#[test]
fn compare_defaults_to_compare_output_names() {
    let cli = Cli::parse_from(["fastaguard", "compare", "a.fa", "b.fa"]);
    let command = cli.to_command_config().unwrap();

    let CommandConfig::Compare(config) = command else {
        panic!("expected compare config");
    };
    assert_eq!(config.inputs, vec![PathBuf::from("a.fa"), PathBuf::from("b.fa")]);
    assert_eq!(config.outputs.html, PathBuf::from("cohort_report.html"));
    assert_eq!(config.outputs.json, PathBuf::from("cohort.json"));
    assert_eq!(config.outputs.tsv, PathBuf::from("cohort.tsv"));
    assert_eq!(
        config.outputs.multiqc,
        PathBuf::from("fastaguard_compare_mqc.json")
    );
}

#[test]
fn compare_rejects_single_input() {
    let cli = Cli::parse_from(["fastaguard", "compare", "a.fa"]);
    let error = cli.to_command_config().unwrap_err();

    assert!(error.to_string().contains("compare requires at least two FASTA inputs"));
}
```

Add to `tests/cli.rs`:

```rust
#[test]
fn compare_requires_at_least_two_inputs() {
    Command::cargo_bin("fastaguard")
        .unwrap()
        .args(["compare", "testdata/valid_assembly.fa"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("compare requires at least two FASTA inputs"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --locked cli::tests::compare_defaults_to_compare_output_names -- --nocapture
cargo test --locked --test cli compare_requires_at_least_two_inputs -- --nocapture
```

Expected: FAIL because clap subcommands and compare config do not exist.

- [ ] **Step 3: Refactor CLI into subcommands without breaking current command**

Modify `src/cli.rs`:

```rust
use clap::{ArgGroup, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "fastaguard")]
#[command(version)]
#[command(about = "FASTA preflight QC for assembly pipelines")]
pub struct Cli {
    #[command(flatten)]
    pub contract: ContractFlags,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Debug, Clone, Args)]
#[command(group(
    ArgGroup::new("contract")
        .args(["schema", "finding_catalog", "explain_finding"])
        .multiple(false)
))]
pub struct ContractFlags {
    #[arg(long)]
    pub schema: bool,
    #[arg(long)]
    pub finding_catalog: bool,
    #[arg(long, value_name = "ID")]
    pub explain_finding: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Compare(CompareArgs),
}
```

Move existing single-file fields into `RunArgs`, keeping `input: Option<PathBuf>`.

Add:

```rust
#[derive(Debug, Clone, Args)]
pub struct CompareArgs {
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    #[command(flatten)]
    pub common: CommonAnalysisArgs,

    #[arg(long, default_value = "cohort_report.html")]
    pub out: PathBuf,
    #[arg(long, default_value = "cohort.json")]
    pub json: PathBuf,
    #[arg(long, default_value = "cohort.tsv")]
    pub tsv: PathBuf,
    #[arg(long, default_value = "fastaguard_compare_mqc.json")]
    pub multiqc: PathBuf,
}
```

Create:

```rust
#[derive(Debug, Clone)]
pub enum CommandConfig {
    Run(RunConfig),
    Compare(CompareConfig),
    Contract,
}

#[derive(Debug, Clone)]
pub struct CompareConfig {
    pub inputs: Vec<PathBuf>,
    pub profile: String,
    pub gate_mode: GateMode,
    pub outputs: OutputPaths,
    pub rules: RuleConfig,
    pub thresholds: ThresholdOverrides,
    pub threads: usize,
    pub command: String,
    pub started_at: String,
    pub provenance_timestamp_override: Option<String>,
}
```

Add:

```rust
pub fn to_command_config(&self) -> Result<CommandConfig>
```

Keep `to_run_config()` as a compatibility wrapper:

```rust
pub fn to_run_config(&self) -> Result<RunConfig> {
    match self.to_command_config()? {
        CommandConfig::Run(config) => Ok(config),
        CommandConfig::Compare(_) => Err(anyhow!("compare command cannot be converted to RunConfig")),
        CommandConfig::Contract => Err(anyhow!("contract command does not have run config")),
    }
}
```

- [ ] **Step 4: Update `src/lib.rs` routing**

Modify `run`:

```rust
if cli.contract.schema {
    println!("{}", contract::schema_json().trim_end());
    return Ok(0);
}
if cli.contract.finding_catalog {
    println!("{}", contract::finding_catalog_json().trim_end());
    return Ok(0);
}
if let Some(finding_id) = &cli.contract.explain_finding {
    println!("{}", contract::explain_finding_json(finding_id)?);
    return Ok(0);
}

match cli.to_command_config()? {
    cli::CommandConfig::Run(config) => run_single(config),
    cli::CommandConfig::Compare(config) => compare::run_compare(config),
    cli::CommandConfig::Contract => Ok(0),
}
```

Move current single-file body into:

```rust
fn run_single(config: cli::RunConfig) -> Result<i32>
```

Add to `src/compare.rs` a compile bridge. It exists only to let the CLI
refactor compile before Task 6 adds real compare behavior:

```rust
use anyhow::Result;

use crate::cli::CompareConfig;

pub fn run_compare(_config: CompareConfig) -> Result<i32> {
    Ok(0)
}
```

Add to `src/lib.rs`:

```rust
pub mod compare;
```

- [ ] **Step 5: Run CLI tests**

Run:

```bash
cargo test --locked cli::tests
cargo test --locked --test cli compare_requires_at_least_two_inputs
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/lib.rs src/compare.rs tests/cli.rs
git commit -m "feat: add compare CLI shape"
```

---

## Task 6: Implement Compare Analysis And JSON Model

**Files:**

- Modify: `src/compare.rs`
- Modify: `src/models.rs`
- Modify: `src/report/mod.rs`
- Test: `src/compare.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Write failing compare model/unit tests**

Add to `src/compare.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::VerdictStatus;

    #[test]
    fn worst_status_prefers_fail_over_warn_over_pass() {
        assert_eq!(
            worst_status([VerdictStatus::Pass, VerdictStatus::Warn, VerdictStatus::Fail]),
            VerdictStatus::Fail
        );
        assert_eq!(
            worst_status([VerdictStatus::Pass, VerdictStatus::Warn]),
            VerdictStatus::Warn
        );
        assert_eq!(worst_status([VerdictStatus::Pass]), VerdictStatus::Pass);
    }

    #[test]
    fn sample_id_uses_file_stem_without_compression_suffix() {
        assert_eq!(sample_id(Path::new("assemblies/ecoli.fa")), "ecoli");
        assert_eq!(sample_id(Path::new("assemblies/ecoli.fasta.gz")), "ecoli");
    }
}
```

Add to `tests/cli.rs`:

```rust
#[test]
fn compare_writes_json_with_mixed_status_samples() {
    let temp = tempfile::tempdir().unwrap();
    let json = temp.path().join("cohort.json");
    let html = temp.path().join("cohort.html");
    let tsv = temp.path().join("cohort.tsv");
    let multiqc = temp.path().join("cohort_mqc.json");

    Command::cargo_bin("fastaguard")
        .unwrap()
        .args([
            "compare",
            "testdata/valid_assembly.fa",
            "testdata/problem_assembly.fa",
            "--gate",
            "pipeline",
            "--json",
            json.to_str().unwrap(),
            "--out",
            html.to_str().unwrap(),
            "--tsv",
            tsv.to_str().unwrap(),
            "--multiqc",
            multiqc.to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(report["report_type"], "compare");
    assert_eq!(report["schema_version"], "0.4.0");
    assert_eq!(report["summary"]["sample_count"], 2);
    assert_eq!(report["summary"]["fail_count"], 1);
    assert_eq!(report["samples"].as_array().unwrap().len(), 2);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --locked compare::tests -- --nocapture
cargo test --locked --test cli compare_writes_json_with_mixed_status_samples -- --nocapture
```

Expected: unit tests fail until helpers exist; CLI test fails until compare writes reports.

- [ ] **Step 3: Add compare models**

Modify `src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareReport {
    pub schema_version: String,
    pub report_type: String,
    pub tool: ToolInfo,
    pub input: CompareInputInfo,
    pub summary: CompareSummary,
    pub samples: Vec<CompareSample>,
    pub cohort_findings: Vec<CohortFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareInputInfo {
    pub profile: String,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareSummary {
    pub sample_count: u64,
    pub pass_count: u64,
    pub warn_count: u64,
    pub fail_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareSample {
    pub sample_id: String,
    pub input_path: String,
    pub verdict: VerdictStatus,
    pub gate_status: VerdictStatus,
    pub readiness_status: crate::readiness::ReadinessStatus,
    pub sequence_count: u64,
    pub total_length: u64,
    pub n50: u64,
    pub n90: u64,
    pub gc_percent: f64,
    pub n_percent: f64,
    pub duplicate_id_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
    pub gc_outlier_count: u64,
    pub length_outlier_count: u64,
    pub finding_count: u64,
    pub finding_ids: Vec<String>,
    pub readiness_blockers: Vec<String>,
    pub recommended_next_tools: Vec<String>,
    pub input_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohortFinding {
    pub id: String,
    pub severity: Severity,
    pub affected_count: u64,
    pub evidence: serde_json::Value,
}
```

- [ ] **Step 4: Implement compare runner**

Modify `src/compare.rs`:

```rust
use anyhow::{Context, Result};
use std::path::Path;
use std::time::Instant;

use crate::cli::{CompareConfig, RunConfig};
use crate::models::{
    CompareInputInfo, CompareReport, CompareSample, CompareSummary, FastaguardReport,
    SCHEMA_VERSION, TOOL_NAME, TOOL_VERSION, ToolInfo, VerdictStatus,
};

pub fn run_compare(config: CompareConfig) -> Result<i32> {
    let mut samples = Vec::new();
    for input in &config.inputs {
        let sample_report = run_one_sample(&config, input)?;
        samples.push(sample_from_report(sample_id(input), &sample_report));
    }
    let summary = summarize(&samples);
    let report = CompareReport {
        schema_version: SCHEMA_VERSION.to_string(),
        report_type: "compare".to_string(),
        tool: ToolInfo {
            name: TOOL_NAME.to_string(),
            version: TOOL_VERSION.to_string(),
        },
        input: CompareInputInfo {
            profile: config.profile.clone(),
            sample_count: samples.len() as u64,
        },
        summary,
        samples,
        cohort_findings: Vec::new(),
    };
    crate::report::write_compare_all(&report, &config.outputs)?;
    Ok(compare_exit_code(&report))
}
```

Implement `run_one_sample` by constructing a `RunConfig` with in-memory artifact
names derived from compare output names, then building a report without writing
per-sample files. To avoid duplicating single-run logic, add a helper in
`src/lib.rs`:

```rust
pub(crate) fn build_single_report(config: cli::RunConfig, started: Instant) -> Result<models::FastaguardReport>
```

Use that helper from both `run_single` and `compare::run_one_sample`.

Implement helpers:

```rust
pub(crate) fn sample_id(path: &Path) -> String
pub(crate) fn worst_status<I>(statuses: I) -> VerdictStatus
where
    I: IntoIterator<Item = VerdictStatus>
```

For `.fasta.gz`, strip `.gz` first, then `.fasta` or `.fa`.

- [ ] **Step 5: Add bootstrap JSON-only compare writer**

Modify `src/report/mod.rs`:

```rust
use crate::models::CompareReport;

pub fn write_compare_all(report: &CompareReport, outputs: &OutputPaths) -> Result<()> {
    validate_output_paths(outputs)?;
    std::fs::write(&outputs.json, serde_json::to_string_pretty(report)? + "\n")?;
    std::fs::write(&outputs.tsv, "sample_id\tverdict\n")?;
    std::fs::write(&outputs.multiqc, "{}\n")?;
    std::fs::write(&outputs.html, "<!doctype html><title>FastaGuard Compare</title>\n")?;
    Ok(())
}
```

This bootstrap writer is intentionally complete enough for Task 6 JSON tests.
Task 7 replaces the bootstrap TSV, MultiQC, and HTML bodies with complete report
writers.

- [ ] **Step 6: Run compare tests**

Run:

```bash
cargo test --locked compare::tests
cargo test --locked --test cli compare_writes_json_with_mixed_status_samples
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/compare.rs src/models.rs src/lib.rs src/report/mod.rs tests/cli.rs
git commit -m "feat: build compare JSON report"
```

---

## Task 7: Add Cohort Findings And Compare Report Writers

**Files:**

- Modify: `src/compare.rs`
- Create: `src/report/compare_tsv.rs`
- Create: `src/report/compare_multiqc.rs`
- Create: `src/report/compare_html.rs`
- Modify: `src/report/mod.rs`
- Test: `src/compare.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Write failing cohort finding test**

Add to `src/compare.rs`:

```rust
#[test]
fn cohort_total_length_outliers_rank_unusual_samples() {
    let samples = vec![
        sample_for_cohort("a", 5_000_000, 50.0, 0.1, 100, 200_000),
        sample_for_cohort("b", 5_100_000, 50.2, 0.1, 101, 210_000),
        sample_for_cohort("c", 9_000_000, 50.1, 0.1, 99, 205_000),
    ];

    let findings = cohort_findings(&samples);

    assert!(findings
        .iter()
        .any(|finding| finding.id == "cohort_total_length_outliers"));
}
```

Add helper:

```rust
fn sample_for_cohort(
    sample_id: &str,
    total_length: u64,
    gc_percent: f64,
    n_percent: f64,
    sequence_count: u64,
    n50: u64,
) -> CompareSample {
    CompareSample {
        sample_id: sample_id.to_string(),
        input_path: format!("{sample_id}.fa"),
        verdict: VerdictStatus::Pass,
        gate_status: VerdictStatus::Pass,
        readiness_status: crate::readiness::ReadinessStatus::Pass,
        sequence_count,
        total_length,
        n50,
        n90: n50,
        gc_percent,
        n_percent,
        duplicate_id_count: 0,
        invalid_sequence_count: 0,
        high_n_sequence_count: 0,
        tiny_contig_count: 0,
        max_gap_run: 0,
        gc_outlier_count: 0,
        length_outlier_count: 0,
        finding_count: 0,
        finding_ids: Vec::new(),
        readiness_blockers: Vec::new(),
        recommended_next_tools: Vec::new(),
        input_sha256: "0".repeat(64),
    }
}
```

- [ ] **Step 2: Write failing writer CLI assertions**

Extend `compare_writes_json_with_mixed_status_samples`:

```rust
assert!(std::fs::read_to_string(&tsv).unwrap().contains("sample_id\tinput_path\tverdict"));
assert!(std::fs::read_to_string(&html).unwrap().contains("Readiness Matrix"));
let mqc: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(&multiqc).unwrap()).unwrap();
assert_eq!(mqc["plot_type"], "table");
assert!(mqc["data"].as_object().unwrap().contains_key("valid_assembly"));
```

- [ ] **Step 3: Run tests to verify failure**

Run:

```bash
cargo test --locked compare::tests::cohort_total_length_outliers_rank_unusual_samples -- --nocapture
cargo test --locked --test cli compare_writes_json_with_mixed_status_samples -- --nocapture
```

Expected: FAIL because cohort findings and real writers do not exist.

- [ ] **Step 4: Implement cohort findings**

In `src/compare.rs`, add:

```rust
pub(crate) fn cohort_findings(samples: &[CompareSample]) -> Vec<CohortFinding> {
    let mut findings = Vec::new();
    push_numeric_outliers(
        &mut findings,
        "cohort_total_length_outliers",
        samples,
        |sample| sample.total_length as f64,
        |sample| serde_json::json!({
            "sample_id": sample.sample_id,
            "total_length": sample.total_length,
            "reason": "total length is unusual relative to the cohort"
        }),
    );
    push_numeric_outliers(
        &mut findings,
        "cohort_gc_outliers",
        samples,
        |sample| sample.gc_percent,
        |sample| serde_json::json!({
            "sample_id": sample.sample_id,
            "gc_percent": sample.gc_percent,
            "reason": "GC percent is unusual relative to the cohort"
        }),
    );
    findings
}
```

Use simple deterministic IQR or z-score logic already present in `src/stats/outliers.rs`. If using existing IQR functions for `u64`, use it for total length, sequence count, and N50. For floating values, add a small local z-score helper requiring at least three finite values.

Set `report.cohort_findings = cohort_findings(&samples)`.

- [ ] **Step 5: Implement compare TSV writer**

Create `src/report/compare_tsv.rs`:

```rust
use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::models::{CompareReport, VerdictStatus};

pub fn write(report: &CompareReport, path: &Path) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "sample_id\tinput_path\tverdict\tgate_status\treadiness_status\tsequence_count\ttotal_length\tn50\tn90\tgc_percent\tn_percent\tduplicate_id_count\tinvalid_sequence_count\thigh_n_sequence_count\ttiny_contig_count\tmax_gap_run\tgc_outlier_count\tlength_outlier_count\tfinding_count\treadiness_blockers\trecommended_next_tools\tinput_sha256"
    )?;
    for sample in &report.samples {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            sample.sample_id,
            sample.input_path,
            status(sample.verdict),
            status(sample.gate_status),
            readiness_status(sample.readiness_status),
            sample.sequence_count,
            sample.total_length,
            sample.n50,
            sample.n90,
            sample.gc_percent,
            sample.n_percent,
            sample.duplicate_id_count,
            sample.invalid_sequence_count,
            sample.high_n_sequence_count,
            sample.tiny_contig_count,
            sample.max_gap_run,
            sample.gc_outlier_count,
            sample.length_outlier_count,
            sample.finding_count,
            sample.readiness_blockers.join(","),
            sample.recommended_next_tools.join(","),
            sample.input_sha256
        )?;
    }
    writer.flush()?;
    Ok(())
}

fn status(status: VerdictStatus) -> &'static str {
    match status {
        VerdictStatus::Pass => "PASS",
        VerdictStatus::Warn => "WARN",
        VerdictStatus::Fail => "FAIL",
    }
}

fn readiness_status(status: crate::readiness::ReadinessStatus) -> &'static str {
    match status {
        crate::readiness::ReadinessStatus::Pass => "PASS",
        crate::readiness::ReadinessStatus::Warn => "WARN",
        crate::readiness::ReadinessStatus::Fail => "FAIL",
    }
}
```

This writer deliberately mirrors the existing single-file TSV style and does not
add a TSV/CSV dependency.

- [ ] **Step 6: Implement compare MultiQC writer**

Create `src/report/compare_multiqc.rs` with standard custom content:

```json
{
  "id": "fastaguard_compare_summary",
  "section_name": "FastaGuard Compare",
  "description": "FASTA preflight readiness summary across multiple inputs",
  "plot_type": "table",
  "pconfig": {
    "id": "fastaguard_compare_summary",
    "title": "FastaGuard Compare"
  },
  "data": {
    "sample": {
      "verdict": "PASS",
      "gate_status": "PASS",
      "readiness_status": "PASS"
    }
  }
}
```

Write with `serde_json::to_writer_pretty`.

- [ ] **Step 7: Implement compare HTML writer**

Create `src/report/compare_html.rs`.

Required strings for tests:

```text
FastaGuard Compare Report
Readiness Matrix
Cohort Findings
Suggested Next Tools
```

Use inline SVG bar charts for total length, N50, GC%, N%, and sequence count. Keep helpers private:

```rust
fn render_bar_chart(title: &str, samples: &[CompareSample], value: fn(&CompareSample) -> f64) -> String
```

Escape all sample IDs and paths with the same style as `html.rs`.

- [ ] **Step 8: Wire real writers**

Modify `src/report/mod.rs`:

```rust
pub mod compare_html;
pub mod compare_multiqc;
pub mod compare_tsv;

pub fn write_compare_all(report: &CompareReport, outputs: &OutputPaths) -> Result<()> {
    validate_output_paths(outputs)?;
    json::write_compare(report, &outputs.json)?;
    compare_tsv::write(report, &outputs.tsv)?;
    compare_multiqc::write(report, &outputs.multiqc)?;
    compare_html::write(report, &outputs.html)?;
    Ok(())
}
```

Modify `src/report/json.rs`:

```rust
pub fn write_compare(report: &crate::models::CompareReport, path: &Path) -> Result<()> {
    let file = File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("failed to write {}", path.display()))?;
    std::fs::OpenOptions::new().append(true).open(path)?.write_all(b"\n")?;
    Ok(())
}
```

- [ ] **Step 9: Run tests**

Run:

```bash
cargo test --locked compare::tests
cargo test --locked --test cli compare_writes_json_with_mixed_status_samples
cargo test --locked report::compare
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add src/compare.rs src/models.rs src/report/mod.rs src/report/json.rs src/report/compare_tsv.rs src/report/compare_multiqc.rs src/report/compare_html.rs tests/cli.rs
git commit -m "feat: write compare reports"
```

---

## Task 8: Schema, Golden Compare Reports, And Contract Discovery

**Files:**

- Modify: `schema/fastaguard.schema.json`
- Modify: `tests/schema_contract.rs`
- Create: `tests/golden/compare_mixed_status.json`
- Create: `tests/golden/compare_all_pass.json`
- Test: `tests/schema_contract.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Write failing schema tests for compare**

Add to `tests/schema_contract.rs`:

```rust
#[test]
fn schema_supports_compare_reports() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();

    assert!(schema["$defs"].get("compare_report").is_some());
    assert!(schema["$defs"]["compare_report"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "samples"));
}
```

Add CLI golden test:

```rust
#[test]
fn compare_golden_mixed_status_matches() {
    let temp = tempfile::tempdir().unwrap();
    let json = temp.path().join("compare.json");
    let html = temp.path().join("compare.html");
    let tsv = temp.path().join("compare.tsv");
    let multiqc = temp.path().join("compare_mqc.json");

    Command::cargo_bin("fastaguard")
        .unwrap()
        .env("FASTAGUARD_PROVENANCE_TIMESTAMP", "2026-06-02T00:00:00Z")
        .args([
            "compare",
            "testdata/valid_assembly.fa",
            "testdata/problem_assembly.fa",
            "--gate",
            "pipeline",
            "--json",
            json.to_str().unwrap(),
            "--out",
            html.to_str().unwrap(),
            "--tsv",
            tsv.to_str().unwrap(),
            "--multiqc",
            multiqc.to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let actual: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("golden/compare_mixed_status.json")).unwrap();
    assert_eq!(actual, expected);
}
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --locked --test schema_contract schema_supports_compare_reports -- --nocapture
cargo test --locked --test cli compare_golden_mixed_status_matches -- --nocapture
```

Expected: FAIL because schema and golden files are missing.

- [ ] **Step 3: Update schema**

Modify `schema/fastaguard.schema.json` so top-level accepts either single or compare:

```json
"oneOf": [
  { "$ref": "#/$defs/single_report" },
  { "$ref": "#/$defs/compare_report" }
]
```

Move current top-level report fields under `$defs.single_report`. Add `$defs.compare_report`, `$defs.compare_sample`, `$defs.compare_summary`, and `$defs.cohort_finding`.

Keep `additionalProperties: false` for both report types.

- [ ] **Step 4: Generate compare goldens**

Run the compare command with deterministic provenance:

```bash
mkdir -p target/fastaguard-golden-runtime
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-06-02T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='fastaguard compare testdata/valid_assembly.fa testdata/problem_assembly.fa --gate pipeline --json target/fastaguard-golden-runtime/compare_mixed_status.json --out target/fastaguard-golden-runtime/compare_mixed_status.html --tsv target/fastaguard-golden-runtime/compare_mixed_status.tsv --multiqc target/fastaguard-golden-runtime/compare_mixed_status_mqc.json' \
cargo run --locked -- compare \
  testdata/valid_assembly.fa \
  testdata/problem_assembly.fa \
  --gate pipeline \
  --json target/fastaguard-golden-runtime/compare_mixed_status.json \
  --out target/fastaguard-golden-runtime/compare_mixed_status.html \
  --tsv target/fastaguard-golden-runtime/compare_mixed_status.tsv \
  --multiqc target/fastaguard-golden-runtime/compare_mixed_status_mqc.json || test "$?" = "2"
cp target/fastaguard-golden-runtime/compare_mixed_status.json tests/golden/compare_mixed_status.json
```

Generate all-pass:

```bash
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-06-02T00:00:00Z \
cargo run --locked -- compare \
  testdata/valid_assembly.fa \
  testdata/valid_assembly.fa \
  --json target/fastaguard-golden-runtime/compare_all_pass.json \
  --out target/fastaguard-golden-runtime/compare_all_pass.html \
  --tsv target/fastaguard-golden-runtime/compare_all_pass.tsv \
  --multiqc target/fastaguard-golden-runtime/compare_all_pass_mqc.json
cp target/fastaguard-golden-runtime/compare_all_pass.json tests/golden/compare_all_pass.json
```

- [ ] **Step 5: Run schema/golden tests**

Run:

```bash
cargo test --locked --test schema_contract
cargo test --locked --test cli compare_golden
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add schema/fastaguard.schema.json tests/schema_contract.rs tests/cli.rs tests/golden/compare_mixed_status.json tests/golden/compare_all_pass.json
git commit -m "test: add compare schema and goldens"
```

---

## Task 9: Documentation, Examples, Value Benchmark, And Release Notes

**Files:**

- Create: `docs/preflight-readiness.md`
- Create: `docs/compare-mode.md`
- Create: `docs/value-benchmark.md`
- Create: `docs/releases/v0.4.0.md`
- Modify: `README.md`
- Modify: `docs/tool-landscape.md`
- Modify: `docs/benchmarking.md`
- Modify: `docs/output-contract.md`
- Modify: `docs/roadmap.md`
- Modify: `examples/nextflow/main.nf`
- Modify: `examples/snakemake/Snakefile`
- Modify: `examples/nf-core/README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Write failing Python adoption tests**

Add to `tests/python/test_adoption_assets.py`:

```python
def test_v0_4_docs_explain_preflight_readiness_and_compare_mode(self):
    readiness = ROOT / "docs" / "preflight-readiness.md"
    compare = ROOT / "docs" / "compare-mode.md"
    value = ROOT / "docs" / "value-benchmark.md"

    for path in (readiness, compare, value):
        self.assertTrue(path.exists(), path)

    self.assertIn("before interpretive QC tools", readiness.read_text())
    self.assertIn("Index readiness", readiness.read_text())
    self.assertIn("fastaguard compare", compare.read_text())
    self.assertIn("fastaguard_compare_mqc.json", compare.read_text())
    self.assertIn("0.98 seconds", value.read_text())
    self.assertIn("50 MB", value.read_text())

def test_v0_4_examples_mention_compare_as_starter_pattern(self):
    text = "\n".join(
        [
            (ROOT / "examples" / "nf-core" / "README.md").read_text(),
            (ROOT / "examples" / "snakemake" / "Snakefile").read_text(),
            (ROOT / "examples" / "nextflow" / "main.nf").read_text(),
        ]
    )
    self.assertIn("fastaguard compare", text)
    self.assertIn("starter", text.lower())
```

- [ ] **Step 2: Run Python tests to verify failure**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_v0_4_docs_explain_preflight_readiness_and_compare_mode -v
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_v0_4_examples_mention_compare_as_starter_pattern -v
```

Expected: FAIL because docs/examples are missing.

- [ ] **Step 3: Create `docs/preflight-readiness.md`**

Include these sections exactly:

```markdown
# Preflight Readiness

FastaGuard runs before interpretive QC tools. It checks whether a FASTA file is
safe enough for downstream tools to consume.

## Readiness Categories

- File readiness
- Structure readiness
- Alphabet readiness
- Index readiness
- Assembly readiness
- Submission readiness
- Machine readiness

## Boundary

FastaGuard does not prove biological completeness, assembly correctness, or
taxonomic contamination. It routes users to QUAST, BUSCO, BlobToolKit, CheckM,
samtools, BLAST, official submission validators, or annotation tools when those
questions matter.
```

- [ ] **Step 4: Create `docs/compare-mode.md`**

Include:

```markdown
# Compare Mode

```bash
fastaguard compare assemblies/*.fa \
  --profile assembly \
  --gate pipeline \
  --out cohort_report.html \
  --json cohort.json \
  --tsv cohort.tsv \
  --multiqc fastaguard_compare_mqc.json
```

Compare mode is a starter cohort triage layer. It ranks many FASTA files by
preflight status, readiness status, structural metrics, composition metrics, and
cohort outliers.
```

- [ ] **Step 5: Create `docs/value-benchmark.md`**

Include measured local numbers:

```markdown
# Value Benchmark

Measured locally with `fastaguard 0.3.0`, commit `1873216`, macOS ARM64:

| Input | Result | Time | Memory |
| --- | --- | ---: | ---: |
| 10 Mbp synthetic FASTA, 10k records | PASS | 0.51 seconds | about 17 MB RSS |
| 100 Mbp synthetic FASTA, 100k records | WARN for GC outliers | 0.98 seconds | about 50 MB RSS |

FastaGuard costs seconds. It can save minutes, CPU-hours, or days when it blocks
a bad FASTA before heavier QC starts.
```

- [ ] **Step 6: Update README/docs/examples**

Add README quick example:

```bash
fastaguard compare assemblies/*.fa --profile assembly --gate pipeline
```

Add links:

```markdown
- [Preflight readiness](docs/preflight-readiness.md)
- [Compare mode](docs/compare-mode.md)
- [Value benchmark](docs/value-benchmark.md)
```

Update examples with comments that compare mode is a starter pattern, not an upstream nf-core/Snakemake submission yet.

- [ ] **Step 7: Add v0.4 release notes draft**

Create `docs/releases/v0.4.0.md` with:

```markdown
# FastaGuard v0.4.0

Theme: Preflight Readiness + Compare Mode.

This release adds `fastaguard compare`, readiness categories, index-readiness
checks, submission-readiness advisories, and cohort triage outputs. FastaGuard
remains FASTA preflight QC and does not replace QUAST, BUSCO, BlobToolKit,
CheckM, official submission validators, or annotation workflows.
```

- [ ] **Step 8: Run Python tests**

Run:

```bash
python3 -m unittest discover tests/python -v
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add README.md docs/preflight-readiness.md docs/compare-mode.md docs/value-benchmark.md docs/releases/v0.4.0.md docs/tool-landscape.md docs/benchmarking.md docs/output-contract.md docs/roadmap.md examples/nextflow/main.nf examples/snakemake/Snakefile examples/nf-core/README.md tests/python/test_adoption_assets.py
git commit -m "docs: document v0.4 readiness and compare mode"
```

---

## Task 10: Final Integration, Version Bump, Examples, And Verification

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `README.md`
- Modify: `docs/releases/v0.4.0.md`
- Modify: `examples/reports/**`
- Test: all test suites

- [ ] **Step 1: Bump package version**

Modify `Cargo.toml`:

```toml
version = "0.4.0"
```

Run:

```bash
cargo check --locked
```

If `Cargo.lock` needs the package version update, run:

```bash
cargo check
```

Then inspect that only the FastaGuard package version changed in `Cargo.lock`.

- [ ] **Step 2: Regenerate example reports**

Run:

```bash
cargo build --release --locked
rm -rf examples/reports/assembly_pass examples/reports/assembly_fail
mkdir -p examples/reports/assembly_pass examples/reports/assembly_fail
target/release/fastaguard testdata/valid_assembly.fa \
  --min-contig-length 1 \
  --out examples/reports/assembly_pass/fastaguard_report.html \
  --json examples/reports/assembly_pass/fastaguard.json \
  --tsv examples/reports/assembly_pass/fastaguard.tsv \
  --multiqc examples/reports/assembly_pass/fastaguard_mqc.json
target/release/fastaguard testdata/problem_assembly.fa \
  --out examples/reports/assembly_fail/fastaguard_report.html \
  --json examples/reports/assembly_fail/fastaguard.json \
  --tsv examples/reports/assembly_fail/fastaguard.tsv \
  --multiqc examples/reports/assembly_fail/fastaguard_mqc.json || test "$?" = "2"
```

- [ ] **Step 3: Run local compare smoke**

Run:

```bash
mkdir -p target/v0.4-smoke
target/release/fastaguard compare \
  testdata/valid_assembly.fa \
  testdata/problem_assembly.fa \
  --gate pipeline \
  --out target/v0.4-smoke/cohort_report.html \
  --json target/v0.4-smoke/cohort.json \
  --tsv target/v0.4-smoke/cohort.tsv \
  --multiqc target/v0.4-smoke/fastaguard_compare_mqc.json || test "$?" = "2"
jq '.report_type, .summary, .samples[].readiness_status' target/v0.4-smoke/cohort.json
```

Expected:

```text
"compare"
summary object with sample_count 2
one PASS or WARN/FAIL readiness status per sample
```

- [ ] **Step 4: Run full verification gates**

Run:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

Expected: all commands pass with no trailing whitespace output.

- [ ] **Step 5: Review changed files**

Run:

```bash
git status --short
git diff --stat
git diff -- README.md docs src schema tests examples Cargo.toml Cargo.lock | sed -n '1,260p'
```

Expected: only v0.4 readiness, compare, docs, schema, tests, examples, and version files changed.

- [ ] **Step 6: Commit final release prep**

```bash
git add Cargo.toml Cargo.lock README.md docs/releases/v0.4.0.md examples/reports
git commit -m "chore: prepare v0.4 release metadata"
```

---

## Self-Review Checklist

- Spec coverage:
  - Compare command: Tasks 5-8.
  - Readiness matrix: Tasks 1 and 4.
  - New preflight findings: Tasks 2 and 3.
  - Gate behavior: Task 3.
  - JSON/TSV/HTML/MultiQC outputs: Tasks 4, 7, and 8.
  - Value benchmark docs: Task 9.
  - Release criteria and verification: Task 10.

- Product boundaries:
  - No external database calls are introduced.
  - Expected size is user-provided only.
  - Submission readiness is advisory, not a new gate.
  - Compare mode is cohort triage, not biological interpretation.

- Implementation boundaries:
  - `metrics` collects signals.
  - `findings` interprets signals into stable finding IDs.
  - `readiness` maps findings to tool-readiness categories.
  - `compare` orchestrates many single-file analyses.
  - `report/*` writes views without changing analysis behavior.

## Execution Handoff

Plan complete when this file is committed. Recommended execution mode:

```text
Subagent-Driven
```

Use one focused implementation agent per task, review each commit, then continue.
