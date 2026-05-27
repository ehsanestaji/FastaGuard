# FastaGuard v0.3 Assembly Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build FastaGuard v0.3 as a pipeline-ready assembly gate with `--gate pipeline`, machine-readable gate decisions, input SHA256 provenance, updated report outputs, and evidence documentation.

**Architecture:** Keep gate policy separate from finding detection. CLI parsing records a gate mode, a new gate module expands that mode into final failure rules, report assembly derives a compact `gate` object from triggered findings, and provenance streams the input file to compute SHA256 without loading the FASTA into memory.

**Tech Stack:** Rust 2021, clap, serde, sha2, hex, JSON Schema, assert_cmd, jsonschema, Python unittest, existing FastaGuard report writers, NCBI Datasets CLI for optional public evidence.

---

## File Structure

- Create `src/gate.rs`: gate mode enum, pipeline preset failure IDs, final failure-set expansion, and gate decision derivation.
- Modify `src/cli.rs`: add `--gate`, store gate mode in `RunConfig`, and union preset rules with explicit `--fail-on`.
- Modify `src/lib.rs`: expose `gate` module and update test config fixtures.
- Modify `src/models.rs`: bump schema version, add `GateDecision`, add `gate` to reports, add `provenance.input_sha256`, and compute streaming checksum.
- Modify `src/report/tsv.rs`, `src/report/multiqc.rs`, and `src/report/html.rs`: surface gate and checksum fields.
- Modify `schema/fastaguard.schema.json`, `schema/finding-catalog.json`, golden JSON files, example reports, and docs to reflect schema `0.3.0`.
- Modify `tests/cli.rs`, `tests/schema_contract.rs`, report writer unit tests, and Python adoption/release tests.
- Add `docs/evidence/fastaguard-v0.3-evidence.md` and, after a successful public run, compact summaries under `docs/evidence/v0.3/`.

## Task 1: Add Gate Policy And CLI Parsing

**Files:**
- Create: `src/gate.rs`
- Modify: `src/cli.rs`
- Modify: `src/lib.rs`
- Test: `src/cli.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Write failing CLI and gate unit tests**

Add these imports in `src/cli.rs` tests:

```rust
use crate::gate::GateMode;
```

Add these tests to `src/cli.rs`:

```rust
#[test]
fn gate_none_preserves_explicit_fail_rules() {
    let cli = Cli::parse_from([
        "fastaguard",
        "input.fa",
        "--gate",
        "none",
        "--fail-on",
        "gc_outliers",
    ]);
    let config = cli.to_run_config().unwrap();

    assert_eq!(config.gate_mode, GateMode::None);
    assert_eq!(
        config.rules.fail_on,
        ["gc_outliers"].into_iter().map(str::to_string).collect()
    );
}

#[test]
fn gate_pipeline_adds_conservative_fail_rules() {
    let cli = Cli::parse_from(["fastaguard", "input.fa", "--gate", "pipeline"]);
    let config = cli.to_run_config().unwrap();

    assert_eq!(config.gate_mode, GateMode::Pipeline);
    assert_eq!(
        config.rules.fail_on,
        [
            "duplicate_ids",
            "high_n_rate",
            "invalid_chars",
            "invalid_fasta_structure",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    );
}

#[test]
fn gate_pipeline_unions_explicit_fail_rules() {
    let cli = Cli::parse_from([
        "fastaguard",
        "input.fa",
        "--gate",
        "pipeline",
        "--fail-on",
        "gc_outliers",
    ]);
    let config = cli.to_run_config().unwrap();

    assert!(config.rules.fail_on.contains("duplicate_ids"));
    assert!(config.rules.fail_on.contains("invalid_chars"));
    assert!(config.rules.fail_on.contains("invalid_fasta_structure"));
    assert!(config.rules.fail_on.contains("high_n_rate"));
    assert!(config.rules.fail_on.contains("gc_outliers"));
}
```

Add this CLI integration test to `tests/cli.rs`:

```rust
#[test]
fn unknown_gate_value_is_cli_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/valid_assembly.fa", "--gate", "strict"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'strict'"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --locked gate_none_preserves_explicit_fail_rules gate_pipeline_adds_conservative_fail_rules gate_pipeline_unions_explicit_fail_rules unknown_gate_value_is_cli_error
```

Expected: compile failure because `crate::gate`, `GateMode`, `Cli.gate`, and `RunConfig.gate_mode` do not exist.

- [ ] **Step 3: Add the gate module**

Create `src/gate.rs`:

```rust
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
```

- [ ] **Step 4: Wire the gate module into lib and CLI**

In `src/lib.rs`, add:

```rust
pub mod gate;
```

In `src/cli.rs`, add:

```rust
use crate::gate::{self, GateMode};
```

Add this field to `Cli` after `profile`:

```rust
/// Gate preset for pipeline-friendly failure behavior.
#[arg(long, value_enum, default_value_t = GateMode::None)]
pub gate: GateMode,
```

Add this field to `RunConfig`:

```rust
pub gate_mode: GateMode,
```

In `Cli::to_run_config`, set:

```rust
gate_mode: self.gate,
rules: RuleConfig {
    fail_on: gate::final_fail_on(self.gate, &self.fail_on),
},
```

Remove the old call to `normalize_rules(&self.fail_on)`. Keep `normalize_rules` only if another test or helper still uses it; otherwise delete it.

Update `cli_with_max_n_rate` test helper:

```rust
gate: GateMode::None,
```

Update `src/lib.rs` test config:

```rust
gate_mode: crate::gate::GateMode::None,
```

- [ ] **Step 5: Run tests and commit**

Run:

```bash
cargo test --locked gate_none_preserves_explicit_fail_rules gate_pipeline_adds_conservative_fail_rules gate_pipeline_unions_explicit_fail_rules unknown_gate_value_is_cli_error
```

Expected: all four tests pass.

Commit:

```bash
git add src/gate.rs src/cli.rs src/lib.rs tests/cli.rs
git commit -m "feat: add assembly gate preset"
```

## Task 2: Add JSON Gate Decision And Input SHA256 Provenance

**Files:**
- Modify: `src/gate.rs`
- Modify: `src/models.rs`
- Modify: `schema/fastaguard.schema.json`
- Test: `tests/cli.rs`
- Test: `tests/schema_contract.rs`

- [ ] **Step 1: Write failing report contract tests**

Add this test to `tests/cli.rs`:

```rust
#[test]
fn pipeline_gate_report_lists_blocking_and_advisory_findings() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "pipeline_gate");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--gate", "pipeline", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2);

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.3.0"));
    assert_eq!(report["gate"]["mode"], json!("pipeline"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(&report["gate"]["blocking_findings"], "duplicate_ids"));
    assert!(array_contains_string(&report["gate"]["blocking_findings"], "invalid_chars"));
    assert!(array_contains_string(&report["gate"]["blocking_findings"], "high_n_rate"));
    assert!(array_contains_string(&report["gate"]["advisory_findings"], "gap_runs"));
    assert!(array_contains_string(&report["gate"]["fail_on"], "invalid_fasta_structure"));
    assert_eq!(
        report["provenance"]["input_sha256"],
        json!(sha256_file(Path::new("testdata/problem_assembly.fa")))
    );
}
```

Add this test to `tests/cli.rs`:

```rust
#[test]
fn gate_none_report_preserves_warning_behavior_and_checksum() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "gate_none");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2);

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("none"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(&report["gate"]["blocking_findings"], "duplicate_ids"));
    assert!(array_contains_string(&report["gate"]["blocking_findings"], "invalid_chars"));
    assert!(array_contains_string(&report["gate"]["advisory_findings"], "high_n_rate"));
    assert_eq!(
        report["provenance"]["input_sha256"],
        json!(sha256_file(Path::new("testdata/problem_assembly.fa")))
    );
}
```

Add this helper to `tests/cli.rs` near the other helpers:

```rust
fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    let bytes = std::fs::read(path).unwrap();
    hex::encode(Sha256::digest(bytes))
}
```

Add this schema test to `tests/schema_contract.rs`:

```rust
#[test]
fn schema_requires_gate_and_input_sha256() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let report_required = schema["required"].as_array().unwrap();
    let provenance_required = schema["properties"]["provenance"]["required"]
        .as_array()
        .unwrap();

    assert!(report_required.contains(&serde_json::json!("gate")));
    assert!(provenance_required.contains(&serde_json::json!("input_sha256")));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --locked pipeline_gate_report_lists_blocking_and_advisory_findings gate_none_report_preserves_warning_behavior_and_checksum schema_requires_gate_and_input_sha256
```

Expected: compile or assertion failures because report `gate`, schema `gate`, schema `0.3.0`, and `provenance.input_sha256` do not exist yet.

- [ ] **Step 3: Add gate decision types and derivation**

In `src/models.rs`, change:

```rust
pub const SCHEMA_VERSION: &str = "0.3.0";
```

Add `gate` to `FastaguardReport` after `verdict`:

```rust
pub gate: GateDecision,
```

Add this struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecision {
    pub mode: String,
    pub status: VerdictStatus,
    pub blocking_findings: Vec<String>,
    pub advisory_findings: Vec<String>,
    pub fail_on: Vec<String>,
}
```

In `src/gate.rs`, add:

```rust
use crate::models::{Finding, GateDecision, VerdictStatus};

pub fn decision(
    mode: GateMode,
    status: VerdictStatus,
    findings: &[Finding],
    fail_on: &BTreeSet<String>,
) -> GateDecision {
    let mut blocking_findings = Vec::new();
    let mut advisory_findings = Vec::new();

    for finding in findings {
        if fail_on.contains(&finding.id) || finding.severity == crate::models::Severity::Critical {
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
```

In `src/models.rs`, import `crate::gate` and set `gate` in both constructors:

```rust
gate: gate::decision(
    config.gate_mode,
    analysis.status,
    &findings,
    &config.rules.fail_on,
),
```

For invalid FASTA:

```rust
gate: gate::decision(
    config.gate_mode,
    VerdictStatus::Fail,
    &findings,
    &config.rules.fail_on,
),
```

- [ ] **Step 4: Add streaming input SHA256**

In `src/models.rs`, add imports:

```rust
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
```

Add `input_sha256` to `Provenance` after `input_size_bytes`:

```rust
pub input_sha256: String,
```

Add helper:

```rust
fn input_sha256(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("failed to open {} for SHA256", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .with_context(|| format!("failed to read {} for SHA256", path.display()))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}
```

Change `build_provenance` to compute:

```rust
let input_sha256 = input_sha256(&config.input).unwrap_or_else(|_| String::new());
```

Set:

```rust
input_sha256,
```

The empty-string fallback should only affect report-only test fixtures that build synthetic reports without readable input paths. CLI runs should already fail before report creation when input is unreadable.

- [ ] **Step 5: Update schema**

In `schema/fastaguard.schema.json`:

- Change `schema_version.const` from `0.2.0` to `0.3.0`.
- Add `"gate"` to top-level `required` after `"verdict"`.
- Add `gate` under top-level `properties`:

```json
"gate": {
  "type": "object",
  "required": ["mode", "status", "blocking_findings", "advisory_findings", "fail_on"],
  "properties": {
    "mode": {
      "type": "string",
      "enum": ["none", "pipeline"]
    },
    "status": {
      "type": "string",
      "enum": ["PASS", "WARN", "FAIL"]
    },
    "blocking_findings": {
      "type": "array",
      "items": { "type": "string" },
      "uniqueItems": true
    },
    "advisory_findings": {
      "type": "array",
      "items": { "type": "string" },
      "uniqueItems": true
    },
    "fail_on": {
      "type": "array",
      "items": { "type": "string" },
      "uniqueItems": true
    }
  }
}
```

- Add `"input_sha256"` to `provenance.required`.
- Add `input_sha256` to `provenance.properties`:

```json
"input_sha256": {
  "type": "string",
  "pattern": "^[a-f0-9]{64}$"
}
```

- [ ] **Step 6: Update test fixture builders**

Update every manual `FastaguardReport` literal in:

- `src/report/tsv.rs`
- `src/report/multiqc.rs`
- `src/report/html.rs`
- `src/report/mod.rs`

Add:

```rust
gate: GateDecision {
    mode: "none".to_string(),
    status,
    blocking_findings: Vec::new(),
    advisory_findings: Vec::new(),
    fail_on: Vec::new(),
},
```

For literals with a fixed `VerdictStatus::Pass`, use `status: VerdictStatus::Pass`.

Add provenance:

```rust
input_sha256: "0".repeat(64),
```

- [ ] **Step 7: Run tests and commit**

Run:

```bash
cargo test --locked pipeline_gate_report_lists_blocking_and_advisory_findings gate_none_report_preserves_warning_behavior_and_checksum schema_requires_gate_and_input_sha256
cargo test --locked
```

Expected: all tests pass except golden/schema tests may still fail until Task 4 regenerates fixtures. If only golden/schema fixture mismatches remain, continue to Task 4 before committing. If unit or CLI behavior tests fail, fix before continuing.

Commit after behavior tests and schema update are passing or after Task 4 if golden fixtures are part of the same change:

```bash
git add src/gate.rs src/models.rs schema/fastaguard.schema.json tests/cli.rs tests/schema_contract.rs src/report/tsv.rs src/report/multiqc.rs src/report/html.rs src/report/mod.rs
git commit -m "feat: add gate report contract"
```

## Task 3: Surface Gate Fields In TSV, MultiQC, And HTML

**Files:**
- Modify: `src/report/tsv.rs`
- Modify: `src/report/multiqc.rs`
- Modify: `src/report/html.rs`
- Test: report writer unit tests and `tests/cli.rs`

- [ ] **Step 1: Write failing output tests**

In `src/report/tsv.rs`, add:

```rust
#[test]
fn writes_gate_and_checksum_rows() {
    let mut report = test_report(VerdictStatus::Fail);
    report.gate.mode = "pipeline".to_string();
    report.gate.status = VerdictStatus::Fail;
    report.gate.blocking_findings = vec!["duplicate_ids".to_string()];
    report.gate.advisory_findings = vec!["gc_outliers".to_string()];
    report.provenance.input_sha256 = "a".repeat(64);
    let file = NamedTempFile::new().unwrap();

    write(&report, file.path()).unwrap();

    let output = fs::read_to_string(file.path()).unwrap();
    assert!(output.contains("gate_mode\tpipeline\n"), "{output}");
    assert!(output.contains("gate_status\tFAIL\n"), "{output}");
    assert!(output.contains("gate_blocking_findings\tduplicate_ids\n"), "{output}");
    assert!(output.contains("gate_advisory_findings\tgc_outliers\n"), "{output}");
    assert!(output.contains(&format!("input_sha256\t{}\n", "a".repeat(64))), "{output}");
}
```

In `src/report/multiqc.rs`, extend `writes_multiqc_custom_content_table`:

```rust
assert_eq!(output["data"]["sample"]["gate_mode"], "none");
assert_eq!(output["data"]["sample"]["gate_status"], "PASS");
assert_eq!(output["data"]["sample"]["gate_blocking_findings"], "");
```

In `tests/cli.rs`, add:

```rust
#[test]
fn html_report_shows_gate_decision() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "html_gate");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--gate", "pipeline", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2);

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Gate Decision"), "{html}");
    assert!(html.contains("Blocking"), "{html}");
    assert!(html.contains("Advisory"), "{html}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --locked writes_gate_and_checksum_rows writes_multiqc_custom_content_table html_report_shows_gate_decision
```

Expected: failures because the output writers do not emit gate fields yet.

- [ ] **Step 3: Add TSV rows**

In `src/report/tsv.rs`, after verdict:

```rust
write_metric(&mut writer, "gate_mode", &report.gate.mode)?;
write_metric(
    &mut writer,
    "gate_status",
    verdict_status(report.gate.status),
)?;
write_metric(
    &mut writer,
    "gate_blocking_findings",
    report.gate.blocking_findings.join(","),
)?;
write_metric(
    &mut writer,
    "gate_advisory_findings",
    report.gate.advisory_findings.join(","),
)?;
write_metric(
    &mut writer,
    "input_sha256",
    &report.provenance.input_sha256,
)?;
```

- [ ] **Step 4: Add MultiQC fields**

In `src/report/multiqc.rs`, add fields to `MultiqcSummaryRow`:

```rust
gate_mode: String,
gate_status: &'static str,
gate_blocking_findings: String,
```

Set them in `summary_row`:

```rust
gate_mode: report.gate.mode.clone(),
gate_status: verdict_status(report.gate.status),
gate_blocking_findings: report.gate.blocking_findings.join(","),
```

- [ ] **Step 5: Add HTML gate panel**

In `src/report/html.rs`, add a `let gate = render_gate(report);` line in `render`.

Place this block after the positioning paragraph and before Machine Summary:

```html
<h2>Gate Decision</h2>
{gate}
```

Add helper:

```rust
fn render_gate(report: &FastaguardReport) -> String {
    format!(
        r#"<div class="grid">
<section class="panel">
<h3>Gate</h3>
<p><span class="label">Mode:</span> {mode}</p>
<p><span class="label">Status:</span> {status}</p>
</section>
<section class="panel">
<h3>Blocking</h3>
{blocking}
</section>
<section class="panel">
<h3>Advisory</h3>
{advisory}
</section>
</div>"#,
        mode = escape_html(&report.gate.mode),
        status = escape_html(verdict_status(report.gate.status)),
        blocking = render_string_list_or_none(&report.gate.blocking_findings),
        advisory = render_string_list_or_none(&report.gate.advisory_findings),
    )
}

fn render_string_list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "<p>None</p>".to_string()
    } else {
        render_string_list(values)
    }
}
```

- [ ] **Step 6: Run tests and commit**

Run:

```bash
cargo test --locked writes_gate_and_checksum_rows writes_multiqc_custom_content_table html_report_shows_gate_decision
```

Expected: all pass.

Commit:

```bash
git add src/report/tsv.rs src/report/multiqc.rs src/report/html.rs tests/cli.rs
git commit -m "feat: surface assembly gate outputs"
```

## Task 4: Bump Version, Schema, Golden Reports, And Examples To v0.3.0

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/models.rs`
- Modify: `schema/finding-catalog.json`
- Modify: `tests/golden/*.json`
- Modify: `examples/reports/**`
- Modify: `tests/python/test_release_metadata.py`
- Modify: `tests/schema_contract.rs`

- [ ] **Step 1: Update metadata tests first**

In `tests/python/test_release_metadata.py`, change version expectations from `0.2.0` to `0.3.0` for Cargo and release notes existence. Add assertions that v0.3 release notes mention:

```python
self.assertIn("FastaGuard v0.3.0", text)
self.assertIn("Evidence And Assembly Gate", text)
self.assertIn("--gate pipeline", text)
self.assertIn("input_sha256", text)
```

Keep Bioconda source SHA checks scoped to the current recipe until the v0.3 GitHub source archive exists; do not require `packaging/bioconda/meta.yaml` to be v0.3 during feature implementation.

- [ ] **Step 2: Run Python metadata tests to verify failure**

Run:

```bash
python3 -m unittest tests.python.test_release_metadata -v
```

Expected: failure because Cargo is still `0.2.0` and `docs/releases/v0.3.0.md` does not exist.

- [ ] **Step 3: Bump Cargo package version**

In `Cargo.toml`:

```toml
version = "0.3.0"
```

Run:

```bash
cargo update -p fastaguard --precise 0.3.0
```

Expected: `Cargo.lock` updates the local package version to `0.3.0`.

- [ ] **Step 4: Update schema and catalog versions**

In `schema/finding-catalog.json`, change:

```json
"schema_version": "0.3.0",
"catalog_version": "0.3.0"
```

In tests that assert catalog version, update expected strings to `0.3.0`.

- [ ] **Step 5: Add v0.3 release notes**

Create `docs/releases/v0.3.0.md`:

````markdown
# FastaGuard v0.3.0

FastaGuard v0.3.0 is the Evidence And Assembly Gate release.

## Highlights

- Adds `--gate pipeline` for conservative assembly preflight gating.
- Adds a machine-readable `gate` object to JSON reports.
- Adds `provenance.input_sha256` so reports identify the exact input bytes.
- Surfaces gate mode, status, blocking findings, advisory findings, and input
  checksum in pipeline-friendly outputs.
- Documents the v0.3 evidence workflow for local and public assembly runs.

## Install

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

Until the v0.3 Bioconda update is merged, Bioconda may still serve the previous
published release. GitHub release binaries and source archives should be used
for immediate v0.3 testing after the tag is published.

## Pipeline Gate

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

The pipeline gate fails on duplicate IDs, invalid characters, structurally
invalid FASTA, and high-N content. GC and length outliers remain advisory unless
explicitly added with `--fail-on`.

## Known Limits

- FastaGuard remains assembly-focused.
- Gate decisions are FASTA preflight decisions, not biological completeness,
  contamination, or assembly correctness claims.
- External taxonomy, coverage, k-mer, and database-backed checks remain
  follow-up steps.
````

- [ ] **Step 6: Regenerate golden and example reports**

Run the existing golden update workflow manually by using the commands encoded in `tests/cli.rs`. If no helper exists, regenerate with:

```bash
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-05-23T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='fastaguard testdata/valid_assembly.fa --min-contig-length 1 --out target/fastaguard-golden-runtime/valid_assembly.html --json target/fastaguard-golden-runtime/valid_assembly.json --tsv target/fastaguard-golden-runtime/valid_assembly.tsv --multiqc target/fastaguard-golden-runtime/valid_assembly_multiqc.json' \
cargo run -- testdata/valid_assembly.fa --min-contig-length 1 \
  --out target/fastaguard-golden-runtime/valid_assembly.html \
  --json tests/golden/valid_assembly.json \
  --tsv target/fastaguard-golden-runtime/valid_assembly.tsv \
  --multiqc target/fastaguard-golden-runtime/valid_assembly_multiqc.json

FASTAGUARD_PROVENANCE_TIMESTAMP=2026-05-23T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='fastaguard testdata/problem_assembly.fa --out target/fastaguard-golden-runtime/problem_assembly.html --json target/fastaguard-golden-runtime/problem_assembly.json --tsv target/fastaguard-golden-runtime/problem_assembly.tsv --multiqc target/fastaguard-golden-runtime/problem_assembly_multiqc.json' \
cargo run -- testdata/problem_assembly.fa \
  --out target/fastaguard-golden-runtime/problem_assembly.html \
  --json tests/golden/problem_assembly.json \
  --tsv target/fastaguard-golden-runtime/problem_assembly.tsv \
  --multiqc target/fastaguard-golden-runtime/problem_assembly_multiqc.json || test "$?" = "2"

FASTAGUARD_PROVENANCE_TIMESTAMP=2026-05-23T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='fastaguard testdata/invalid_empty_record.fa --out target/fastaguard-golden-runtime/invalid_empty_record.html --json target/fastaguard-golden-runtime/invalid_empty_record.json --tsv target/fastaguard-golden-runtime/invalid_empty_record.tsv --multiqc target/fastaguard-golden-runtime/invalid_empty_record_multiqc.json' \
cargo run -- testdata/invalid_empty_record.fa \
  --out target/fastaguard-golden-runtime/invalid_empty_record.html \
  --json tests/golden/invalid_empty_record.json \
  --tsv target/fastaguard-golden-runtime/invalid_empty_record.tsv \
  --multiqc target/fastaguard-golden-runtime/invalid_empty_record_multiqc.json || test "$?" = "2"
```

Regenerate committed examples:

```bash
cargo run -- testdata/valid_assembly.fa \
  --min-contig-length 1 \
  --out examples/reports/assembly_pass/fastaguard_report.html \
  --json examples/reports/assembly_pass/fastaguard.json \
  --tsv examples/reports/assembly_pass/fastaguard.tsv \
  --multiqc examples/reports/assembly_pass/fastaguard_mqc.json

cargo run -- testdata/problem_assembly.fa \
  --out examples/reports/assembly_fail/fastaguard_report.html \
  --json examples/reports/assembly_fail/fastaguard.json \
  --tsv examples/reports/assembly_fail/fastaguard.tsv \
  --multiqc examples/reports/assembly_fail/fastaguard_mqc.json || test "$?" = "2"
```

- [ ] **Step 7: Run tests and commit**

Run:

```bash
python3 -m unittest tests.python.test_release_metadata -v
cargo test --locked
```

Expected: all pass.

Commit:

```bash
git add Cargo.toml Cargo.lock src/models.rs schema/finding-catalog.json schema/fastaguard.schema.json tests/golden examples/reports docs/releases/v0.3.0.md tests/python/test_release_metadata.py tests/schema_contract.rs tests/cli.rs
git commit -m "chore: prepare v0.3 report contract"
```

## Task 5: Update User Docs And Workflow Examples

**Files:**
- Modify: `README.md`
- Modify: `docs/output-contract.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/benchmarking.md`
- Modify: `docs/tool-landscape.md`
- Modify: `examples/nextflow/main.nf`
- Modify: `examples/nf-core/modules/local/fastaguard/main.nf`
- Modify: `examples/nf-core/README.md`
- Modify: `examples/snakemake/Snakefile`
- Modify: `examples/snakemake/wrapper/README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Write failing adoption tests**

In `tests/python/test_adoption_assets.py`, add:

```python
def test_v0_3_gate_docs_and_examples_are_present(self):
    readme = (ROOT / "README.md").read_text()
    output_contract = (ROOT / "docs" / "output-contract.md").read_text()
    nfcore_module = (
        ROOT / "examples" / "nf-core" / "modules" / "local" / "fastaguard" / "main.nf"
    ).read_text()
    snakemake = (ROOT / "examples" / "snakemake" / "Snakefile").read_text()

    self.assertIn("--gate pipeline", readme)
    self.assertIn("The assembly FASTA gate before expensive QC.", readme)
    self.assertIn('"gate"', output_contract)
    self.assertIn("provenance.input_sha256", output_contract)
    self.assertIn("--gate pipeline", nfcore_module)
    self.assertIn("--gate pipeline", snakemake)
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_v0_3_gate_docs_and_examples_are_present -v
```

Expected: failure because docs and examples do not mention v0.3 gate yet.

- [ ] **Step 3: Update README**

Add this quickstart example near the current pipeline gate example:

````markdown
Pipeline gate preset:

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

```text
The assembly FASTA gate before expensive QC.
```
````

Mention that `--gate pipeline` fails on duplicate IDs, invalid characters, invalid FASTA structure, and high-N content while keeping GC and length outliers advisory.

- [ ] **Step 4: Update output contract docs**

In `docs/output-contract.md`, add a `Gate Contract` section:

````markdown
## Gate Contract

The `gate` object is the machine-readable assembly gate decision.

```json
"gate": {
  "mode": "pipeline",
  "status": "FAIL",
  "blocking_findings": ["duplicate_ids", "invalid_chars"],
  "advisory_findings": ["gc_outliers"],
  "fail_on": ["duplicate_ids", "high_n_rate", "invalid_chars", "invalid_fasta_structure"]
}
```

Machines should use `gate.blocking_findings` for workflow stop/go decisions.
Humans should use the HTML report to inspect the evidence behind each finding.
`provenance.input_sha256` identifies the exact input bytes used for the report.
````

- [ ] **Step 5: Update workflow examples**

Add `--gate pipeline` to the FastaGuard command blocks in:

- `examples/nextflow/main.nf`
- `examples/nf-core/modules/local/fastaguard/main.nf`
- `examples/snakemake/Snakefile`
- `examples/snakemake/wrapper/wrapper/fastaguard/wrapper.py`

Keep outputs unchanged.

- [ ] **Step 6: Run tests and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
```

Expected: all Python adoption tests pass.

Commit:

```bash
git add README.md docs/output-contract.md docs/roadmap.md docs/benchmarking.md docs/tool-landscape.md examples/nextflow/main.nf examples/nf-core/README.md examples/nf-core/modules/local/fastaguard/main.nf examples/snakemake/Snakefile examples/snakemake/wrapper/README.md examples/snakemake/wrapper/wrapper/fastaguard/wrapper.py tests/python/test_adoption_assets.py
git commit -m "docs: document v0.3 assembly gate"
```

## Task 6: Evidence Pack Updates

**Files:**
- Modify: `scripts/collect_evidence.py`
- Create: `docs/evidence/fastaguard-v0.3-evidence.md`
- Create when public run succeeds: `docs/evidence/v0.3/evidence_summary.json`
- Create when public run succeeds: `docs/evidence/v0.3/evidence_summary.tsv`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Write failing evidence documentation tests**

Add to `tests/python/test_adoption_assets.py`:

```python
def test_v0_3_evidence_docs_reference_gate_and_checksum(self):
    evidence = ROOT / "docs" / "evidence" / "fastaguard-v0.3-evidence.md"

    self.assertTrue(evidence.exists())
    text = evidence.read_text()
    self.assertIn("--gate pipeline", text)
    self.assertIn("input_sha256", text)
    self.assertIn("not biological completeness", text)
    self.assertIn("not contamination confirmation", text)
    self.assertIn("python3 scripts/collect_evidence.py", text)
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_v0_3_evidence_docs_reference_gate_and_checksum -v
```

Expected: failure because v0.3 evidence page does not exist.

- [ ] **Step 3: Update evidence script to run gate mode**

In `scripts/collect_evidence.py`, add `--gate pipeline` to the command list in `run_case` immediately after `--profile assembly`.

Expected command shape:

```python
command = [
    str(binary),
    str(case["input_path"]),
    "--profile",
    "assembly",
    "--gate",
    "pipeline",
    "--out",
    str(html_path),
    "--json",
    str(json_path),
    "--tsv",
    str(tsv_path),
    "--multiqc",
    str(multiqc_path),
]
```

Add summary fields from the parsed report:

```python
"gate_mode": report.get("gate", {}).get("mode"),
"gate_status": report.get("gate", {}).get("status"),
"gate_blocking_findings": ",".join(report.get("gate", {}).get("blocking_findings", [])),
"input_sha256": report.get("provenance", {}).get("input_sha256"),
```

Add these names to `SUMMARY_COLUMNS`:

```python
"gate_mode",
"gate_status",
"gate_blocking_findings",
"input_sha256",
```

- [ ] **Step 4: Add v0.3 evidence page**

Create `docs/evidence/fastaguard-v0.3-evidence.md`:

````markdown
# FastaGuard v0.3 Evidence Pack

This page records the evidence workflow for the v0.3 assembly gate release.

FastaGuard is FASTA preflight QC. It is not biological completeness analysis,
not assembly correctness analysis, and not contamination confirmation.

## Local Gate Evidence

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3-local \
  --local-only
```

The evidence command runs FastaGuard with `--gate pipeline`. Summaries include
the verdict, gate status, blocking findings, top findings, runtime, input size,
and `input_sha256`.

## Public NCBI Evidence

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3
```

The public workflow uses the assembly manifest in
`docs/evidence/public_assemblies.json` and requires NCBI Datasets CLI plus
network access.

## Interpretation

Use this evidence to decide whether FastaGuard is useful as the first assembly
gate before QUAST, BUSCO, BlobToolKit, CheckM, annotation, or submission.
Passing the gate means the FASTA-level contract is sane enough to continue; it
does not prove biological completeness or rule out contamination.
````

- [ ] **Step 5: Run local-only evidence smoke**

Run:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3-local \
  --local-only
```

Expected: command exits `0`, prints JSON summary, and all cases include `gate_mode`, `gate_status`, and `input_sha256`.

- [ ] **Step 6: Run public evidence only if NCBI Datasets CLI is available**

Check:

```bash
command -v datasets
```

If present, run:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3
```

If the public run succeeds, copy only compact summary files into docs:

```bash
mkdir -p docs/evidence/v0.3
cp target/evidence/v0.3/evidence_summary.json docs/evidence/v0.3/evidence_summary.json
cp target/evidence/v0.3/evidence_summary.tsv docs/evidence/v0.3/evidence_summary.tsv
```

Do not commit downloaded FASTA files, NCBI zip archives, or generated per-case reports.

- [ ] **Step 7: Run tests and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
```

Expected: all Python adoption tests pass.

Commit:

```bash
git add scripts/collect_evidence.py docs/evidence/fastaguard-v0.3-evidence.md docs/evidence/v0.3 tests/python/test_adoption_assets.py
git commit -m "docs: add v0.3 gate evidence workflow"
```

If no public evidence run was possible, omit `docs/evidence/v0.3` from `git add` and note the reason in the final summary.

## Task 7: Full Verification And Release Readiness

**Files:**
- Modify only files needed to fix verification failures from previous tasks.

- [ ] **Step 1: Run full local gates**

Run:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

Expected: all commands exit `0` and whitespace scan prints no output.

- [ ] **Step 2: Run CLI smoke for pipeline gate**

Run:

```bash
cargo run -- testdata/problem_assembly.fa \
  --gate pipeline \
  --out target/v0.3-smoke/fastaguard_report.html \
  --json target/v0.3-smoke/fastaguard.json \
  --tsv target/v0.3-smoke/fastaguard.tsv \
  --multiqc target/v0.3-smoke/fastaguard_mqc.json || test "$?" = "2"
```

Inspect:

```bash
jq '.schema_version, .gate, .provenance.input_sha256' target/v0.3-smoke/fastaguard.json
```

Expected:

```text
"0.3.0"
```

Gate mode is `pipeline`, status is `FAIL`, and `input_sha256` is a 64-character lowercase hex string.

- [ ] **Step 3: Review final diff**

Run:

```bash
git status --short --branch
git diff --stat
```

Expected: only v0.3 assembly gate, evidence, docs, tests, schema, and generated example/golden files are changed.

- [ ] **Step 4: Commit final verification fixes**

If Step 1 or Step 2 required changes, run `git status --short` and stage the
specific files shown there that belong to v0.3 assembly gate work. Do not stage
unrelated local files. Use this commit message:

```bash
git commit -m "chore: finalize v0.3 assembly gate"
```

If Step 1 and Step 2 required no changes, skip this commit step.

- [ ] **Step 5: Prepare PR**

Push the branch and open a draft PR:

```bash
git push -u origin codex/v0.3-evidence-assembly-gate
gh pr create \
  --repo ehsanestaji/FastaGuard \
  --base main \
  --head codex/v0.3-evidence-assembly-gate \
  --draft \
  --title "[codex] Add v0.3 assembly gate" \
  --body-file /tmp/fastaguard-v0.3-pr.md
```

Use this PR body:

````markdown
## Summary

- Add `--gate pipeline` for conservative assembly FASTA preflight gating.
- Add machine-readable `gate` JSON plus TSV, MultiQC, and HTML gate outputs.
- Add `provenance.input_sha256` for exact input-file identity.
- Bump the report contract and package metadata to v0.3.0.
- Update docs, examples, release notes, and evidence workflow.

## Validation

- `python3 -m unittest discover tests/python -v`
- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `git diff --check`
- trailing whitespace scan
- local evidence smoke with `scripts/collect_evidence.py --local-only`

## Notes

FastaGuard remains FASTA preflight QC. The gate does not replace QUAST, BUSCO,
BlobToolKit, CheckM, annotation, or contamination workflows.
````
