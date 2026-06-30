# FastaGuard v0.5 Submission Readiness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `--gate submission` and `--submission-target generic|ncbi` so FastaGuard can report FASTA-level submission readiness without claiming to replace official validators.

**Architecture:** Reuse the existing v0.4 analyzer and finding IDs wherever possible. Add one small `submission` module for target parsing and target-specific fail sets, then thread submission target metadata through CLI config, gate decisions, readiness, reports, schema, compare mode, and docs.

**Tech Stack:** Rust 2021, clap, serde/serde_json, assert_cmd integration tests, jsonschema contract tests, existing static HTML/TSV/MultiQC writers.

---

## File Structure

Create:

- `src/submission.rs`: `SubmissionTarget`, display helpers, and submission gate constants.
- `testdata/submission_ids.fa`: FASTA with unsafe IDs, reserved header characters, and duplicate first-token IDs.
- `testdata/submission_warnings.fa`: FASTA with long headers and gap-like N runs.
- `docs/evidence/fastaguard-v0.5-submission-readiness.md`: tiny evidence examples and command transcript.
- `docs/releases/v0.5.0.md`: release notes drafted before tagging.

Modify:

- `src/lib.rs`: export `submission`.
- `src/cli.rs`: parse `--submission-target`, carry it into run and compare configs.
- `src/gate.rs`: add `GateMode::Submission` and target-aware fail rules.
- `src/readiness.rs`: add optional target metadata to readiness categories and map existing findings to submission readiness.
- `src/models.rs`: add submission target fields to gate/provenance/compare summaries and bump schema version.
- `src/findings.rs`: tune text/actions for existing submission-relevant findings; avoid renaming v0.4 IDs.
- `src/contract.rs` and `schema/finding-catalog.json`: keep bundled catalog and runtime actions aligned.
- `schema/fastaguard.schema.json`: update schema version and new fields.
- `src/report/html.rs`, `src/report/tsv.rs`, `src/report/multiqc.rs`: add single-report submission output.
- `src/compare.rs`, `src/report/compare_html.rs`, `src/report/compare_tsv.rs`, `src/report/compare_multiqc.rs`: aggregate and render submission status.
- `tests/cli.rs`, `tests/schema_contract.rs`: add CLI, golden, report, and schema coverage.
- `tests/golden/*.json`, `examples/reports/**`: regenerate committed reports after schema changes.
- `README.md`, `docs/roadmap.md`, `docs/vision-plan.md`, `docs/tool-landscape.md`, `docs/output-contract.md`, `docs/packaging.md`, `examples/nf-core/README.md`, `examples/snakemake/wrapper/README.md`: document v0.5 behavior and boundaries.

Important design choice:

- Keep current finding IDs: `unsafe_ids`, `long_headers`, `reserved_header_chars`, `duplicate_first_token_ids`, `terminal_ns`, `gap_pattern_warnings`, `gap_runs`, `high_n_rate`, and `tiny_contigs`.
- Do not rename them to `unsafe_identifier_chars` or `submission_gap_like_ns` in v0.5. The v0.5 behavior is to promote existing evidence into a stricter submission gate and clearer submission-readiness fields.

## Task 1: CLI Plumbing For Submission Target

**Files:**
- Create: `src/submission.rs`
- Modify: `src/lib.rs`
- Modify: `src/cli.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing CLI tests**

Append these tests to `tests/cli.rs`:

```rust
#[test]
fn submission_gate_defaults_to_generic_target() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_default");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--gate",
        "submission",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(1)
    .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("submission"));
    assert_eq!(report["gate"]["submission_target"], json!("generic"));
    assert_eq!(report["provenance"]["submission_target"], json!("generic"));
}

#[test]
fn submission_target_ncbi_is_serialized_when_requested() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_ncbi");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(1)
    .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["submission_target"], json!("ncbi"));
    assert_eq!(report["provenance"]["submission_target"], json!("ncbi"));
}

#[test]
fn unknown_submission_target_is_cli_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ena",
    ])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("invalid value 'ena'"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --locked --test cli submission_
```

Expected: the first two tests fail because `--gate submission`, `--submission-target`, and serialized fields do not exist. The unknown-target test may fail with a different clap error until the flag exists.

- [ ] **Step 3: Create `src/submission.rs`**

Create:

```rust
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
```

- [ ] **Step 4: Export the module**

Modify `src/lib.rs` and add:

```rust
pub mod submission;
```

- [ ] **Step 5: Add CLI fields and config plumbing**

In `src/cli.rs`, import:

```rust
use crate::submission::SubmissionTarget;
```

Add to `AnalysisArgs`:

```rust
    /// Submission-readiness target used by --gate submission.
    #[arg(long, value_enum)]
    pub submission_target: Option<SubmissionTarget>,
```

Add to `RunConfig`, `CompareConfig`, and `ValidatedAnalysis`:

```rust
    pub submission_target: Option<SubmissionTarget>,
```

In the analysis validation function that constructs `ValidatedAnalysis`, set:

```rust
let submission_target = match (analysis.gate, analysis.submission_target) {
    (GateMode::Submission, None) => Some(SubmissionTarget::Generic),
    (_, target) => target,
};
```

Carry `submission_target` into both `RunConfig` and `CompareConfig`.

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test --locked --test cli submission_
```

Expected: tests still fail until gate/model serialization exists, but clap should now accept `--submission-target generic|ncbi` and reject `ena`.

- [ ] **Step 7: Commit**

```bash
git add src/submission.rs src/lib.rs src/cli.rs tests/cli.rs
git commit -m "feat: add submission target CLI"
```

## Task 2: Submission Gate Semantics

**Files:**
- Modify: `src/gate.rs`
- Modify: `src/models.rs`
- Modify: `src/compare.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing tests for blocking behavior**

Add fixtures:

`testdata/submission_ids.fa`

```text
>seq/one
ACGTACGT
>seq two
ACGTACGT
>seq two duplicate-description
ACGTACGA
>pipe|id
ACGTACGT
```

Add to `tests/cli.rs`:

```rust
#[test]
fn submission_gate_fails_identifier_hazards() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_ids");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("submission"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "duplicate_first_token_ids"
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "unsafe_ids"
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "reserved_header_chars"
    ));
}
```

- [ ] **Step 2: Run failing test**

Run:

```bash
cargo test --locked --test cli submission_gate_fails_identifier_hazards
```

Expected: FAIL because `GateMode::Submission` and the submission fail set do not exist.

- [ ] **Step 3: Add `Submission` gate mode and fail set**

Modify `src/gate.rs`:

```rust
use crate::submission::SubmissionTarget;

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
```

Extend `GateMode`:

```rust
pub enum GateMode {
    None,
    Pipeline,
    Submission,
}
```

Extend `as_str`:

```rust
GateMode::Submission => "submission",
```

Change `final_fail_on` signature:

```rust
pub fn final_fail_on(
    mode: GateMode,
    submission_target: Option<SubmissionTarget>,
    explicit_rules: &[String],
) -> BTreeSet<String>
```

Inside it:

```rust
match mode {
    GateMode::Pipeline => {
        fail_on.extend(PIPELINE_FAIL_ON.iter().map(|id| (*id).to_string()));
    }
    GateMode::Submission => {
        let rules = match submission_target.unwrap_or(SubmissionTarget::Generic) {
            SubmissionTarget::Generic => SUBMISSION_FAIL_ON_GENERIC,
            SubmissionTarget::Ncbi => SUBMISSION_FAIL_ON_NCBI,
        };
        fail_on.extend(rules.iter().map(|id| (*id).to_string()));
    }
    GateMode::None => {}
}
```

- [ ] **Step 4: Add target to gate decision**

Modify `GateDecision` in `src/models.rs`:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_target: Option<String>,
```

Change `gate::decision` signature:

```rust
pub fn decision(
    mode: GateMode,
    submission_target: Option<SubmissionTarget>,
    status: VerdictStatus,
    findings: &[Finding],
    fail_on: &BTreeSet<String>,
) -> GateDecision
```

Set:

```rust
submission_target: submission_target.map(|target| target.as_str().to_string()),
```

Update all call sites in `src/models.rs` to pass `config.submission_target`.

- [ ] **Step 5: Pass target through compare sample runs**

In `src/compare.rs`, add:

```rust
submission_target: config.submission_target,
```

to the `RunConfig` built inside `run_one_sample`.

- [ ] **Step 6: Run focused gate tests**

Run:

```bash
cargo test --locked --test cli submission_
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/gate.rs src/models.rs src/compare.rs tests/cli.rs testdata/submission_ids.fa
git commit -m "feat: add submission gate semantics"
```

## Task 3: Submission Readiness Metadata

**Files:**
- Modify: `src/readiness.rs`
- Modify: `src/models.rs`
- Test: `src/readiness.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing readiness tests**

Add to `src/readiness.rs` tests:

```rust
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
```

- [ ] **Step 2: Run failing readiness tests**

Run:

```bash
cargo test --locked readiness::tests::submission_target_is_attached_to_submission_category readiness::tests::submission_findings_warn_when_not_blocking
```

Expected: FAIL because `ReadinessCategory.target` and the new function signature do not exist.

- [ ] **Step 3: Extend readiness category**

Modify `ReadinessCategory`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub target: Option<String>,
```

In `base_categories`, set `target: None`.

Change `build_readiness` signature:

```rust
pub fn build_readiness(
    verdict: VerdictStatus,
    blocking_findings: &[String],
    findings: &[Finding],
    scope: ReadinessScope,
    submission_target: Option<crate::submission::SubmissionTarget>,
) -> ReadinessReport
```

After `base_categories(scope)`, attach the target:

```rust
if let Some(target) = submission_target {
    if let Some(category) = categories
        .iter_mut()
        .find(|category| category.id == "submission")
    {
        category.target = Some(target.as_str().to_string());
    }
}
```

Update all call sites. Use `None` in tests that do not care about target.

- [ ] **Step 4: Promote existing findings into submission readiness**

Update `category_ids_for_finding`:

```rust
"duplicate_ids" | "duplicate_first_token_ids" => &["index", "submission"],
"unsafe_ids" | "long_headers" | "reserved_header_chars" => &["index", "submission"],
"terminal_ns" | "gap_pattern_warnings" | "gap_runs" => &["assembly", "submission"],
"high_n_rate" | "tiny_contigs" => &["assembly", "submission"],
```

This makes the submission category show every identifier issue that can block
the submission gate, while preserving index readiness for parser/index users.

- [ ] **Step 5: Update model call sites**

In `FastaguardReport::from_analysis` and `FastaguardReport::from_invalid_fasta`, pass `config.submission_target` to `build_readiness`.

In tests that construct readiness manually, pass `None` unless target behavior is under test.

- [ ] **Step 6: Run readiness tests**

Run:

```bash
cargo test --locked readiness
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/readiness.rs src/models.rs
git commit -m "feat: add submission readiness metadata"
```

## Task 4: Provenance, Scope, And Machine Routing

**Files:**
- Modify: `src/models.rs`
- Modify: `src/findings.rs`
- Modify: `schema/finding-catalog.json`
- Test: `src/contract.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing routing/scope assertions**

In `tests/cli.rs`, add to `submission_target_ncbi_is_serialized_when_requested`:

```rust
assert!(array_contains_string(
    &report["scope"]["can_conclude"],
    "FASTA-level submission readiness"
));
assert!(array_contains_string(
    &report["scope"]["cannot_conclude"],
    "repository acceptance"
));
```

Add a separate test:

```rust
#[test]
fn submission_hazards_route_to_official_validators_and_fcs_without_claiming_results() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_routes");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(2);

    let report = read_json(&outputs.json);
    assert_routing_hint(
        &report,
        "submission_readiness_failure",
        "fix_fasta_before_official_validation",
        false,
    );
    assert!(array_contains_tool(
        &report["machine_summary"]["recommended_next_tools"],
        "official submission validator"
    ));
}
```

- [ ] **Step 2: Run failing test**

Run:

```bash
cargo test --locked --test cli submission_hazards_route_to_official_validators_and_fcs_without_claiming_results
```

Expected: FAIL because routing and scope text have not been extended.

- [ ] **Step 3: Extend scope**

In `fasta_preflight_scope()` in `src/models.rs`, include:

```rust
"FASTA-level submission readiness".to_string(),
```

in `can_conclude`, and include:

```rust
"repository acceptance".to_string(),
"official validator acceptance".to_string(),
"annotation correctness".to_string(),
```

in `cannot_conclude`.

- [ ] **Step 4: Extend recommended tools**

In `recommended_next_tools`, route submission-relevant findings:

```rust
if has_any_finding(
    findings,
    &[
        "unsafe_ids",
        "long_headers",
        "reserved_header_chars",
        "duplicate_first_token_ids",
        "terminal_ns",
        "gap_pattern_warnings",
    ],
) {
    tools.push(recommended_tool(
        "official submission validator",
        "Use the target repository validator after FASTA-level issues are fixed; FastaGuard is not an official validator.",
    ));
}

if has_any_finding(findings, &["high_n_rate", "gap_runs"]) {
    tools.push(recommended_tool(
        "NCBI FCS",
        "Run database-backed contamination/adaptor screening when submission-oriented ambiguity or gap signals need follow-up.",
    ));
}
```

Add these helper functions near the existing recommendation helpers:

```rust
fn has_any_finding(findings: &[Finding], ids: &[&str]) -> bool {
    findings
        .iter()
        .any(|finding| ids.iter().any(|id| *id == finding.id))
}

fn recommended_tool(tool: &str, reason: &str) -> RecommendedTool {
    RecommendedTool {
        tool: tool.to_string(),
        reason: reason.to_string(),
    }
}
```

- [ ] **Step 5: Extend routing hints**

In `routing_hints`, add:

```rust
"unsafe_ids" | "long_headers" | "reserved_header_chars" | "duplicate_first_token_ids" => {
    push_routing_hint(
        &mut hints,
        "submission_readiness_failure",
        "fix_fasta_before_official_validation",
        false,
    )
}
```

Keep existing `index_readiness_failure` if present by using two calls for `duplicate_first_token_ids`.

- [ ] **Step 6: Update catalog text and runtime action alignment**

In `schema/finding-catalog.json`, keep IDs unchanged and add wording that these findings affect submission readiness. Do not add suggested actions that are missing from `finding_actions`; `src/contract.rs::bundled_catalog_actions_match_runtime_actions` must remain green.

- [ ] **Step 7: Run tests**

Run:

```bash
cargo test --locked --test cli submission_hazards_route_to_official_validators_and_fcs_without_claiming_results
cargo test --locked contract
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/models.rs src/findings.rs schema/finding-catalog.json tests/cli.rs
git commit -m "feat: route submission readiness findings"
```

## Task 5: Single-Report Output Fields

**Files:**
- Modify: `src/report/tsv.rs`
- Modify: `src/report/multiqc.rs`
- Modify: `src/report/html.rs`
- Test: `src/report/tsv.rs`
- Test: `src/report/multiqc.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing output assertions**

In `tests/cli.rs`, add:

```rust
#[test]
fn submission_gate_outputs_tsv_multiqc_and_html_fields() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_outputs");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(2);

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    assert!(tsv.contains("submission_target\tncbi\n"), "{tsv}");
    assert!(tsv.contains("submission_status\tFAIL\n"), "{tsv}");
    assert!(tsv.contains("unsafe_identifier_count\t"), "{tsv}");

    let multiqc = read_json(&outputs.multiqc);
    assert_eq!(multiqc["data"]["submission_ids"]["submission_target"], json!("ncbi"));
    assert_eq!(multiqc["data"]["submission_ids"]["submission_status"], json!("FAIL"));

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Submission Readiness"), "{html}");
    assert!(html.contains("Official validators are still required"), "{html}");
}
```

- [ ] **Step 2: Run failing output test**

Run:

```bash
cargo test --locked --test cli submission_gate_outputs_tsv_multiqc_and_html_fields
```

Expected: FAIL because the output fields and HTML section are missing.

- [ ] **Step 3: Add TSV metrics**

In `src/report/tsv.rs`, after readiness rows, write:

```rust
write_metric(
    &mut writer,
    "submission_target",
    report
        .gate
        .submission_target
        .as_deref()
        .unwrap_or("."),
)?;
write_metric(
    &mut writer,
    "submission_status",
    submission_status(report),
)?;
write_metric(
    &mut writer,
    "submission_blocking_findings",
    report.gate.blocking_findings.join(","),
)?;
write_metric(
    &mut writer,
    "submission_advisory_findings",
    report.gate.advisory_findings.join(","),
)?;
write_metric(
    &mut writer,
    "unsafe_identifier_count",
    report.summary.unsafe_id_count,
)?;
write_metric(
    &mut writer,
    "long_identifier_count",
    report.summary.long_header_count,
)?;
write_metric(
    &mut writer,
    "duplicate_first_token_id_count",
    report.summary.duplicate_first_token_id_count,
)?;
write_metric(
    &mut writer,
    "gap_like_n_run_count",
    report.summary.repeated_gap_pattern_sequence_count,
)?;
```

Add:

```rust
fn submission_status(report: &FastaguardReport) -> &'static str {
    report
        .readiness
        .category("submission")
        .map(|category| readiness_status(category.status))
        .unwrap_or("PASS")
}
```

- [ ] **Step 4: Add MultiQC fields**

Add to `MultiqcSummaryRow`:

```rust
submission_target: String,
submission_status: String,
unsafe_identifier_count: u64,
long_identifier_count: u64,
duplicate_first_token_id_count: u64,
gap_like_n_run_count: u64,
```

Populate in `summary_row` using the same fields as TSV.

Add headers:

```rust
("submission_target", "Submission Target"),
("submission_status", "Submission Status"),
("unsafe_identifier_count", "Unsafe IDs"),
("long_identifier_count", "Long Headers"),
("duplicate_first_token_id_count", "Duplicate First-Token IDs"),
("gap_like_n_run_count", "Gap-Like N Runs"),
```

- [ ] **Step 5: Add HTML section**

In `src/report/html.rs`, add `let submission = render_submission_readiness(report);` and place this after the Gate Decision section:

```html
<h2>Submission Readiness</h2>
{submission}
```

Add:

```rust
fn render_submission_readiness(report: &FastaguardReport) -> String {
    let target = report
        .gate
        .submission_target
        .as_deref()
        .unwrap_or("generic");
    let category = report.readiness.category("submission");
    let status = category
        .map(|category| readiness_status(category.status))
        .unwrap_or("PASS");
    let findings = category
        .map(|category| render_string_list_or_none(&category.findings))
        .unwrap_or_else(|| "None".to_string());

    format!(
        r#"<div class="grid">
<section class="panel">
<h3>Target</h3>
<p>{target}</p>
</section>
<section class="panel">
<h3>Status</h3>
<p>{status}</p>
</section>
<section class="panel">
<h3>Findings</h3>
{findings}
</section>
</div>
<p class="muted">Official validators are still required. FastaGuard reports FASTA-level preflight risks only.</p>"#,
        target = escape_html(target),
        status = escape_html(status),
        findings = findings,
    )
}
```

- [ ] **Step 6: Run focused output tests**

Run:

```bash
cargo test --locked --test cli submission_gate_outputs_tsv_multiqc_and_html_fields
cargo test --locked report::tsv report::multiqc
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/report/tsv.rs src/report/multiqc.rs src/report/html.rs tests/cli.rs
git commit -m "feat: render submission readiness outputs"
```

## Task 6: Compare Mode Submission Aggregation

**Files:**
- Modify: `src/models.rs`
- Modify: `src/compare.rs`
- Modify: `src/report/compare_tsv.rs`
- Modify: `src/report/compare_multiqc.rs`
- Modify: `src/report/compare_html.rs`
- Test: `tests/cli.rs`
- Test: `src/report/compare_tsv.rs`
- Test: `src/report/compare_multiqc.rs`

- [ ] **Step 1: Add failing compare test**

Add to `tests/cli.rs`:

```rust
#[test]
fn compare_submission_gate_aggregates_submission_status() {
    let temp_dir = TempDir::new().unwrap();
    let clean = temp_dir.path().join("clean.fa");
    std::fs::write(&clean, ">clean\nACGTACGT\n").unwrap();
    let outputs = output_paths(&temp_dir, "submission_compare");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("compare")
        .arg(&clean)
        .arg("testdata/submission_ids.fa")
        .args([
            "--gate",
            "submission",
            "--submission-target",
            "ncbi",
            "--json",
        ])
        .arg(&outputs.json)
        .arg("--out")
        .arg(&outputs.html)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2);

    let report = read_json(&outputs.json);
    assert_eq!(report["summary"]["submission_fail_count"], json!(1));
    assert_eq!(report["summary"]["submission_ready_count"], json!(1));
    let failing = report["samples"]
        .as_array()
        .unwrap()
        .iter()
        .find(|sample| sample["sample_id"] == "submission_ids")
        .unwrap();
    assert_eq!(failing["submission_target"], json!("ncbi"));
    assert_eq!(failing["submission_status"], json!("FAIL"));

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    assert!(tsv.lines().next().unwrap().contains("submission_status"), "{tsv}");

    let multiqc = read_json(&outputs.multiqc);
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_status"],
        json!("FAIL")
    );
}
```

- [ ] **Step 2: Run failing compare test**

Run:

```bash
cargo test --locked --test cli compare_submission_gate_aggregates_submission_status
```

Expected: FAIL because compare summary/sample fields do not exist.

- [ ] **Step 3: Extend compare models**

In `CompareSummary`, add:

```rust
pub submission_ready_count: u64,
pub submission_warn_count: u64,
pub submission_fail_count: u64,
```

In `CompareSample`, add:

```rust
pub submission_target: Option<String>,
pub submission_status: crate::readiness::ReadinessStatus,
```

- [ ] **Step 4: Populate compare fields**

In `compare_sample`:

```rust
let submission_status = report
    .readiness
    .category("submission")
    .map(|category| category.status)
    .unwrap_or(crate::readiness::ReadinessStatus::Pass);
```

Set:

```rust
submission_target: report.gate.submission_target.clone(),
submission_status,
```

In `compare_summary`, count submission statuses:

```rust
submission_ready_count: count_readiness_status(samples, crate::readiness::ReadinessStatus::Pass),
submission_warn_count: count_readiness_status(samples, crate::readiness::ReadinessStatus::Warn),
submission_fail_count: count_readiness_status(samples, crate::readiness::ReadinessStatus::Fail),
```

Add:

```rust
fn count_readiness_status(
    samples: &[CompareSample],
    status: crate::readiness::ReadinessStatus,
) -> u64 {
    usize_to_u64(
        samples
            .iter()
            .filter(|sample| sample.submission_status == status)
            .count(),
    )
}
```

- [ ] **Step 5: Extend compare TSV and MultiQC**

In `src/report/compare_tsv.rs`, add `submission_target` and `submission_status` columns after `readiness_status`.

In `src/report/compare_multiqc.rs`, add the same fields and headers:

```rust
submission_target: String,
submission_status: &'static str,
```

- [ ] **Step 6: Extend compare HTML**

In `src/report/compare_html.rs`, add submission status to the summary cards or table near readiness. Use existing HTML escaping helpers and the same uppercase status labels.

- [ ] **Step 7: Run compare tests**

Run:

```bash
cargo test --locked --test cli compare_submission_gate_aggregates_submission_status
cargo test --locked compare
cargo test --locked report::compare_tsv report::compare_multiqc
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/models.rs src/compare.rs src/report/compare_tsv.rs src/report/compare_multiqc.rs src/report/compare_html.rs tests/cli.rs
git commit -m "feat: aggregate submission readiness in compare mode"
```

## Task 7: Schema Version, Goldens, And Contract Fixtures

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/models.rs`
- Modify: `schema/fastaguard.schema.json`
- Modify: `schema/finding-catalog.json`
- Modify: `tests/schema_contract.rs`
- Modify: `tests/golden/*.json`
- Modify: `examples/reports/**`
- Test: `tests/schema_contract.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing schema assertions**

In `tests/schema_contract.rs`, update schema version assertions:

```rust
assert_eq!(
    single_report["properties"]["schema_version"]["const"],
    "0.5.0"
);
```

Add:

```rust
#[test]
fn schema_supports_submission_gate_fields() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();
    let gate = &schema["$defs"]["single_report"]["properties"]["gate"];
    let provenance = &schema["$defs"]["single_report"]["properties"]["provenance"];
    let compare_summary = &schema["$defs"]["compare_summary"];
    let compare_sample = &schema["$defs"]["compare_sample"];

    assert!(gate["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "submission_target"));
    assert_eq!(
        gate["properties"]["mode"]["enum"],
        serde_json::json!(["none", "pipeline", "submission"])
    );
    assert_eq!(
        gate["properties"]["submission_target"]["enum"],
        serde_json::json!(["generic", "ncbi"])
    );
    assert!(provenance["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "submission_target"));
    assert!(compare_summary["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "submission_fail_count"));
    assert!(compare_sample["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "submission_status"));
}
```

- [ ] **Step 2: Run failing schema tests**

Run:

```bash
cargo test --locked --test schema_contract schema_supports_submission_gate_fields
```

Expected: FAIL because schema is still v0.4.0 and fields are missing.

- [ ] **Step 3: Bump crate and schema versions**

In `Cargo.toml`:

```toml
version = "0.5.0"
```

In `src/models.rs`:

```rust
pub const SCHEMA_VERSION: &str = "0.5.0";
```

Run:

```bash
cargo check --locked
```

If `Cargo.lock` still records `fastaguard 0.4.0`, run:

```bash
cargo check
```

and verify the lockfile updates only the local package version.

- [ ] **Step 4: Update schema**

Update `schema/fastaguard.schema.json`:

- `schema_version.const` from `0.4.0` to `0.5.0` for single and compare reports.
- `gate.mode.enum` to include `"submission"`.
- `gate.required` to include `"submission_target"`.
- `gate.properties.submission_target`:

```json
{
  "type": "string",
  "enum": ["generic", "ncbi"]
}
```

- `provenance.required` to include `"submission_target"`.
- `provenance.properties.submission_target` with the same enum.
- `readiness_category.properties.target` with the same enum.
- `compare_summary.required` and properties for `submission_ready_count`, `submission_warn_count`, `submission_fail_count`.
- `compare_sample.required` and properties for `submission_target` and `submission_status`.

- [ ] **Step 5: Update finding catalog version**

In `schema/finding-catalog.json`:

```json
"schema_version": "0.5.0",
"catalog_version": "0.5.0"
```

Ensure every catalog `suggested_actions` still equals `finding_actions(id)`.

- [ ] **Step 6: Regenerate goldens**

Run the existing golden tests once to produce current temp outputs if helpers write to `target`; if they do not overwrite goldens, generate with the same commands used in `tests/cli.rs` and copy the JSON into:

```text
tests/golden/valid_assembly.json
tests/golden/problem_assembly.json
tests/golden/invalid_empty_record.json
tests/golden/compare_mixed_status.json
tests/golden/compare_all_pass.json
examples/reports/assembly_pass/fastaguard.json
examples/reports/assembly_fail/fastaguard.json
examples/reports/assembly_pass/fastaguard.tsv
examples/reports/assembly_fail/fastaguard.tsv
examples/reports/assembly_pass/fastaguard_mqc.json
examples/reports/assembly_fail/fastaguard_mqc.json
examples/reports/assembly_pass/fastaguard_report.html
examples/reports/assembly_fail/fastaguard_report.html
```

Use deterministic provenance environment variables already present in `tests/cli.rs` when regenerating golden JSON.

- [ ] **Step 7: Run contract tests**

Run:

```bash
cargo test --locked --test schema_contract
cargo test --locked --test cli golden
cargo test --locked contract
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock src/models.rs schema/fastaguard.schema.json schema/finding-catalog.json tests/schema_contract.rs tests/golden examples/reports
git commit -m "chore: update v0.5 output contract"
```

## Task 8: Documentation, Evidence, And Release Notes

**Files:**
- Create: `docs/evidence/fastaguard-v0.5-submission-readiness.md`
- Create: `docs/releases/v0.5.0.md`
- Modify: `README.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/vision-plan.md`
- Modify: `docs/tool-landscape.md`
- Modify: `docs/output-contract.md`
- Modify: `docs/packaging.md`
- Modify: `examples/nf-core/README.md`
- Modify: `examples/snakemake/wrapper/README.md`
- Test: `tests/python/test_adoption_assets.py`
- Test: `tests/python/test_release_metadata.py`

- [ ] **Step 1: Add failing Python docs tests**

In `tests/python/test_adoption_assets.py`, add:

```python
    def test_v0_5_submission_readiness_docs_are_present(self):
        readme = self.read("README.md")
        roadmap = self.read("docs/roadmap.md")
        evidence = self.read("docs/evidence/fastaguard-v0.5-submission-readiness.md")
        release = self.read("docs/releases/v0.5.0.md")

        for text in [readme, roadmap, evidence, release]:
            self.assertIn("--gate submission", text)
            self.assertIn("--submission-target", text)
            self.assertIn("official validators", text)

        self.assertIn("FastaGuard does not replace NCBI, ENA, DDBJ", roadmap)
        self.assertIn("repository acceptance", evidence)
```

In `tests/python/test_release_metadata.py`, update `test_package_targets_v0_4_0` to `test_package_targets_v0_5_0` and assert:

```python
self.assertIn('version = "0.5.0"', cargo_toml)
```

- [ ] **Step 2: Run failing docs tests**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets tests.python.test_release_metadata -v
```

Expected: FAIL because v0.5 docs and release notes are missing or stale.

- [ ] **Step 3: Update README**

Add a quickstart block:

```markdown
Submission-readiness preflight:

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate submission \
  --submission-target ncbi \
  --json fastaguard.json \
  --out fastaguard_report.html
```

FastaGuard reports FASTA-level risks before official validators. It does not
guarantee NCBI, ENA, or DDBJ acceptance and does not replace NCBI FCS,
annotation validation, QUAST, BUSCO, BlobToolKit, or CheckM.
```

- [ ] **Step 4: Add evidence page**

Create `docs/evidence/fastaguard-v0.5-submission-readiness.md`:

```markdown
# FastaGuard v0.5 Submission Readiness Evidence

This page records tiny local evidence cases for the v0.5 submission-readiness
gate. The goal is to show FASTA-level hazards before official validators and
expensive QC.

## Commands

```bash
fastaguard testdata/submission_ids.fa \
  --gate submission \
  --submission-target ncbi \
  --json target/evidence/v0.5/submission_ids.json

fastaguard testdata/submission_warnings.fa \
  --gate submission \
  --submission-target generic \
  --json target/evidence/v0.5/submission_warnings.json
```

## Scope

FastaGuard can report parse validity, identifier safety, duplicate first-token
IDs, invalid sequence symbols, gap-like N runs, high ambiguity, and tiny-record
advisories. It cannot guarantee repository acceptance, biological completeness,
annotation correctness, or contamination status.

## Expected Follow-Up

After FASTA-level blockers are fixed, users should continue to official
validators, NCBI FCS, QUAST, BUSCO, BlobToolKit, CheckM, annotation, or other
the next workflow step named in the report.
```

- [ ] **Step 5: Add release notes**

Create `docs/releases/v0.5.0.md` with:

```markdown
# FastaGuard v0.5.0

FastaGuard v0.5.0 is the Submission Readiness Gate release.

## Highlights

- Adds `--gate submission`.
- Adds `--submission-target generic|ncbi`.
- Adds submission-readiness fields to JSON, TSV, HTML, MultiQC, and compare outputs.
- Promotes existing identifier, header, gap, ambiguity, and tiny-record findings into a clearer submission-readiness view.

## Boundary

FastaGuard is a FASTA-level preflight tool. It does not replace NCBI, ENA, DDBJ,
NCBI FCS, QUAST, BUSCO, BlobToolKit, CheckM, annotation validation, or official
repository acceptance checks.
```

- [ ] **Step 6: Update workflow docs**

In nf-core and Snakemake docs, add the command pattern:

```bash
fastaguard {input.fasta} --gate submission --submission-target ncbi
```

State that pipeline authors should route on:

```text
gate.mode
gate.status
gate.blocking_findings
readiness.categories[id=submission]
```

- [ ] **Step 7: Run docs tests**

Run:

```bash
python3 -m unittest discover tests/python -v
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add README.md docs/roadmap.md docs/vision-plan.md docs/tool-landscape.md docs/output-contract.md docs/packaging.md docs/evidence/fastaguard-v0.5-submission-readiness.md docs/releases/v0.5.0.md examples/nf-core/README.md examples/snakemake/wrapper/README.md tests/python
git commit -m "docs: document v0.5 submission readiness"
```

## Task 9: Full Verification And Release Preparation

**Files:**
- Modify only files needed for failures found by verification.

- [ ] **Step 1: Run full Rust and Python gates**

Run:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

Expected: all commands exit 0 and trailing whitespace scan prints nothing.

- [ ] **Step 2: Run smoke commands**

Run:

```bash
cargo run --locked -- --schema >/tmp/fastaguard-v0.5-schema.json
cargo run --locked -- testdata/submission_ids.fa --gate submission --submission-target ncbi --json /tmp/submission.json --out /tmp/submission.html --tsv /tmp/submission.tsv --multiqc /tmp/submission_mqc.json
cargo run --locked -- compare testdata/valid_assembly.fa testdata/submission_ids.fa --gate submission --submission-target ncbi --json /tmp/submission_compare.json --out /tmp/submission_compare.html --tsv /tmp/submission_compare.tsv --multiqc /tmp/submission_compare_mqc.json
```

Expected:

- `--schema` exits 0.
- single submission run exits 2 because `testdata/submission_ids.fa` contains blocking identifier hazards.
- compare submission run exits 2 because one sample fails.

- [ ] **Step 3: Inspect final diff**

Run:

```bash
git status --short
git diff --stat origin/main..HEAD
git log --oneline --decorate --max-count=12
```

Expected: the branch contains the v0.5 submission-readiness commits and no unrelated file changes.

- [ ] **Step 4: Commit verification fixes if needed**

If verification required edits:

```bash
git status --short
git add src tests schema docs examples Cargo.toml Cargo.lock testdata
git commit -m "fix: stabilize v0.5 submission readiness"
```

If verification required no edits, do not create an empty commit.

## Self-Review Checklist

- Spec coverage: tasks cover CLI, gate behavior, readiness, JSON/provenance/scope, HTML, TSV, MultiQC, compare mode, schema, goldens, docs, evidence, and verification.
- Scope boundary: the plan stays assembly-first and database-free; it routes to official validators and NCBI FCS without claiming to run or replace them.
- Finding IDs: the plan preserves existing v0.4 IDs for identifier/header/gap findings to avoid breaking report consumers.
- Type consistency: `SubmissionTarget` flows from CLI config into gate/provenance/readiness/compare reports as `Option<SubmissionTarget>` or serialized strings.
- Test strategy: every behavior change starts with a failing test, then implementation, then focused verification and commit.

## Execution Choice

Plan complete and saved to `docs/superpowers/plans/2026-06-11-fastaguard-v0.5-submission-readiness.md`. Two execution options:

1. **Subagent-Driven (recommended)** - dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** - execute tasks in this session using executing-plans, batch execution with checkpoints.

Recommended choice: **Subagent-Driven**, because the feature touches independent surfaces: CLI/gate, readiness/schema, report writers, compare mode, and docs.
