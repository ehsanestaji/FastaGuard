# FastaGuard v0.2 Adoption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build FastaGuard v0.2 as an assembly-trust and pipeline-adoption release with stronger integrations, explainable outlier findings, and richer machine-readable provenance.

**Architecture:** Keep the current Rust boundaries: parser and metrics stay streaming-focused, statistical helpers live under `src/stats/`, finding promotion stays in `src/findings.rs`, report contract changes stay in `src/models.rs`, and rendered outputs remain derived views. Integrations and documentation are separate tasks so the core biological behavior can be tested independently.

**Tech Stack:** Rust, clap, serde, JSON Schema, assert_cmd, jsonschema, Python unittest, MultiQC plugin APIs, Nextflow/nf-core starter files, Snakemake wrapper files, Docker, Bioconda.

---

## File Structure

The implementation should touch these files and keep each responsibility clear:

- `README.md`: install and status docs; already has local Bioconda-live edits.
- `docs/adoption-plan.md`: adoption priorities and status; already has local Bioconda-live edits.
- `docs/packaging.md`: official install, platform support, container status.
- `docs/tool-landscape.md`: evidence state and remaining proof.
- `docs/releases/v0.1.1.md`: historical release note corrected for Bioconda status.
- `packaging/bioconda/README.md`: mirror status for the upstream recipe.
- `examples/nf-core/README.md`: local module usage and container status.
- `examples/nf-core/modules/local/fastaguard/main.nf`: add container directive only if a BioContainers image is confirmed.
- `examples/snakemake/wrapper/README.md`: wrapper usage with Bioconda.
- `examples/snakemake/wrapper/environment.yaml`: new Conda environment for wrapper use.
- `integrations/multiqc/src/fastaguard_multiqc/parser.py`: parse expanded `_mqc.json` rows.
- `integrations/multiqc/src/fastaguard_multiqc/multiqc_module.py`: add richer MultiQC general stats, color rules, and summary columns.
- `integrations/multiqc/README.md`: document strict-mode verification and local install.
- `tests/python/test_adoption_assets.py`: test MultiQC parser, wrapper environment, and docs.
- `tests/python/test_release_metadata.py`: keep Bioconda install doc guard.
- `src/stats/outliers.rs`: add robust percentile/IQR helpers for length outliers.
- `src/metrics.rs`: add per-record outlier signal fields after metrics are computed.
- `src/findings.rs`: promote GC outliers, length outliers, and composite anomalies to findings.
- `src/models.rs`: add schema version `0.2.0`, finding taxonomy fields, routing hints, and richer provenance.
- `src/report/html.rs`: show provenance, routing hints, outlier findings, and evidence.
- `src/report/tsv.rs`: add outlier count metrics.
- `src/report/multiqc.rs`: add outlier count fields to custom content.
- `schema/fastaguard.schema.json`: update schema to validate the v0.2 contract.
- `schema/finding-catalog.json`: add v0.2 finding entries and taxonomy fields.
- `tests/cli.rs`: add CLI/contract tests for new fields and findings.
- `tests/schema_contract.rs`: continue validating all golden reports.
- `tests/golden/*.json`: regenerate after contract changes.
- `examples/reports/**`: regenerate example reports.
- `docs/benchmarking.md`: add evidence and runtime/memory table.
- `docs/releases/v0.2.0.md`: new release notes after implementation.

## Task 1: Commit Current Bioconda-Live Documentation Update

**Files:**
- Modify already-local: `README.md`
- Modify already-local: `docs/adoption-plan.md`
- Modify already-local: `docs/packaging.md`
- Modify already-local: `docs/releases/v0.1.1.md`
- Modify already-local: `docs/tool-landscape.md`
- Modify already-local: `examples/nf-core/README.md`
- Modify already-local: `examples/snakemake/wrapper/README.md`
- Modify already-local: `packaging/bioconda/README.md`
- Modify already-local: `tests/python/test_release_metadata.py`

- [ ] **Step 1: Inspect the existing local documentation changes**

Run:

```bash
git diff -- README.md docs/adoption-plan.md docs/packaging.md docs/releases/v0.1.1.md docs/tool-landscape.md examples/nf-core/README.md examples/snakemake/wrapper/README.md packaging/bioconda/README.md tests/python/test_release_metadata.py
```

Expected: only Bioconda-live wording, BioContainers pending wording, and the release metadata guard test are shown.

- [ ] **Step 2: Run the targeted Python tests**

Run:

```bash
python3 -m unittest tests.python.test_release_metadata -v
python3 -m unittest discover tests/python -v
git diff --check
```

Expected: release metadata tests pass, Python discovery passes, and `git diff --check` exits 0.

- [ ] **Step 3: Commit only the current documentation update**

Run:

```bash
git add README.md docs/adoption-plan.md docs/packaging.md docs/releases/v0.1.1.md docs/tool-landscape.md examples/nf-core/README.md examples/snakemake/wrapper/README.md packaging/bioconda/README.md tests/python/test_release_metadata.py
git commit -m "docs: mark Bioconda package live"
```

Expected: one docs commit is created. `git status --short` still shows this plan file if it has not been committed separately.

## Task 2: Confirm BioContainers Status And Update Workflow Docs

**Files:**
- Modify: `docs/packaging.md`
- Modify: `docs/adoption-plan.md`
- Modify: `examples/nf-core/README.md`
- Modify only if confirmed: `examples/nf-core/modules/local/fastaguard/main.nf`
- Modify: `examples/snakemake/wrapper/README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Check BioContainers registry status**

Run:

```bash
python3 - <<'PY'
import json
import subprocess
import sys

tags = [
    "0.1.1--hfa8f182_0",
    "0.1.1--hfc06a8d_0",
    "0.1.1--h87b00fb_0",
    "0.1.1--hb05d258_0",
]
for tag in tags:
    image = f"quay.io/biocontainers/fastaguard:{tag}"
    result = subprocess.run(
        ["docker", "manifest", "inspect", image],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    print(f"{image}\t{result.returncode}")
    if result.returncode == 0:
        print(json.loads(result.stdout).get("schemaVersion", "manifest-ok"))
PY
```

Expected: one or more tags return `0` if BioContainers is visible. If all return nonzero, keep BioContainers documented as pending.

- [ ] **Step 2: If BioContainers is not confirmed, keep docs honest**

Ensure `docs/packaging.md` contains this exact sentence:

```markdown
BioContainers image availability is still pending confirmation.
```

Ensure `examples/nf-core/README.md` contains:

```markdown
Once a BioContainers image is confirmed, the module can add a pinned container directive.
```

- [ ] **Step 3: If BioContainers is confirmed, add a container directive**

Modify `examples/nf-core/modules/local/fastaguard/main.nf` by adding this line after `label 'process_low'`, using the confirmed tag:

```nextflow
    container 'quay.io/biocontainers/fastaguard:0.1.1--hfa8f182_0'
```

If the confirmed tag differs, use the exact tag returned by the manifest check. Also update `examples/nf-core/README.md` with the same image string.

- [ ] **Step 4: Add adoption asset tests for current container wording**

Modify `tests/python/test_adoption_assets.py` with:

```python
    def test_workflow_docs_reference_bioconda_and_container_status(self):
        nfcore_readme = (ROOT / "examples" / "nf-core" / "README.md").read_text()
        snakemake_readme = (
            ROOT / "examples" / "snakemake" / "wrapper" / "README.md"
        ).read_text()

        install = "mamba install -c conda-forge -c bioconda fastaguard"
        self.assertIn(install, nfcore_readme)
        self.assertIn(install, snakemake_readme)
        self.assertTrue(
            "BioContainers image is confirmed" in nfcore_readme
            or "Once a BioContainers image is confirmed" in nfcore_readme
        )
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
git diff --check
git status --short
```

Expected: tests pass and only files from this task are changed.

Commit:

```bash
git add docs/packaging.md docs/adoption-plan.md examples/nf-core/README.md examples/nf-core/modules/local/fastaguard/main.nf examples/snakemake/wrapper/README.md tests/python/test_adoption_assets.py
git commit -m "docs: confirm container adoption status"
```

If `examples/nf-core/modules/local/fastaguard/main.nf` was not changed, omit it from `git add`.

## Task 3: Add Snakemake Bioconda Environment

**Files:**
- Create: `examples/snakemake/wrapper/environment.yaml`
- Modify: `examples/snakemake/wrapper/README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Add a failing test for the wrapper environment**

Modify `tests/python/test_adoption_assets.py` with:

```python
    def test_snakemake_wrapper_declares_bioconda_environment(self):
        environment = (
            ROOT / "examples" / "snakemake" / "wrapper" / "environment.yaml"
        )

        self.assertTrue(environment.exists())
        text = environment.read_text()
        self.assertIn("bioconda", text)
        self.assertIn("conda-forge", text)
        self.assertIn("fastaguard=0.1.1", text)
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_snakemake_wrapper_declares_bioconda_environment -v
```

Expected: FAIL because `environment.yaml` does not exist.

- [ ] **Step 3: Create the Snakemake environment file**

Create `examples/snakemake/wrapper/environment.yaml`:

```yaml
channels:
  - conda-forge
  - bioconda
dependencies:
  - fastaguard=0.1.1
```

- [ ] **Step 4: Update wrapper README usage**

Ensure `examples/snakemake/wrapper/README.md` includes:

````markdown
The wrapper also includes a Conda environment:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```
````

- [ ] **Step 5: Verify and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
git diff --check
```

Expected: tests pass.

Commit:

```bash
git add examples/snakemake/wrapper/environment.yaml examples/snakemake/wrapper/README.md tests/python/test_adoption_assets.py
git commit -m "chore: add Snakemake Bioconda environment"
```

## Task 4: Harden MultiQC Parser And Module Summary

**Files:**
- Modify: `integrations/multiqc/src/fastaguard_multiqc/parser.py`
- Modify: `integrations/multiqc/src/fastaguard_multiqc/multiqc_module.py`
- Modify: `integrations/multiqc/README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Add parser test for expanded MultiQC fields**

Modify `tests/python/test_adoption_assets.py`:

```python
    def test_multiqc_parser_reads_expanded_summary_fields(self):
        with TemporaryDirectory() as temp_dir:
            fixture = Path(temp_dir) / "fastaguard_mqc.json"
            fixture.write_text(
                json.dumps(
                    {
                        "id": "fastaguard",
                        "section_name": "FastaGuard",
                        "description": "FASTA preflight QC summary",
                        "plot_type": "table",
                        "pconfig": {"id": "fastaguard_summary", "title": "FastaGuard"},
                        "data": {
                            "sample": {
                                "verdict": "WARN",
                                "sequence_count": 8,
                                "total_length": 2000,
                                "n50": 500,
                                "n90": 100,
                                "gc_percent": 50.0,
                                "n_percent": 2.5,
                                "duplicate_id_count": 1,
                                "invalid_sequence_count": 0,
                                "high_n_sequence_count": 2,
                                "tiny_contig_count": 1,
                                "max_gap_run": 120,
                                "gc_outlier_count": 1,
                                "length_outlier_count": 1,
                                "composite_anomaly_count": 1,
                                "finding_count": 4,
                            }
                        },
                    }
                )
            )

            summary = load_custom_content_summary(fixture)
            row = summary["sample"]

            for field in (
                "verdict",
                "sequence_count",
                "total_length",
                "n50",
                "n90",
                "gc_percent",
                "n_percent",
                "duplicate_id_count",
                "invalid_sequence_count",
                "high_n_sequence_count",
                "tiny_contig_count",
                "max_gap_run",
                "gc_outlier_count",
                "length_outlier_count",
                "composite_anomaly_count",
                "finding_count",
            ):
                self.assertIn(field, row)
```

- [ ] **Step 2: Run the focused test to observe current failure**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_multiqc_parser_reads_expanded_summary_fields -v
```

Expected: FAIL until the parser supports the expanded fields.

- [ ] **Step 3: Expand parser fields**

Modify `SUMMARY_FIELDS` in `integrations/multiqc/src/fastaguard_multiqc/parser.py`:

```python
SUMMARY_FIELDS = (
    "verdict",
    "sequence_count",
    "total_length",
    "n50",
    "n90",
    "gc_percent",
    "n_percent",
    "duplicate_id_count",
    "invalid_sequence_count",
    "high_n_sequence_count",
    "tiny_contig_count",
    "max_gap_run",
    "gc_outlier_count",
    "length_outlier_count",
    "composite_anomaly_count",
    "finding_count",
)
```

- [ ] **Step 4: Expand MultiQC general stats and table headers**

Modify `integrations/multiqc/src/fastaguard_multiqc/multiqc_module.py` so `_general_stats_data` uses:

```python
visible_fields = (
    "finding_count",
    "gc_outlier_count",
    "length_outlier_count",
    "composite_anomaly_count",
    "n50",
    "n_percent",
)
```

Replace `_general_stats_headers` with:

```python
    @staticmethod
    def _general_stats_headers() -> dict:
        return {
            "finding_count": {
                "title": "FG findings",
                "description": "Number of FastaGuard findings",
                "min": 0,
                "scale": "OrRd",
            },
            "gc_outlier_count": {
                "title": "FG GC outliers",
                "description": "Number of FastaGuard GC outlier records",
                "min": 0,
                "scale": "OrRd",
            },
            "length_outlier_count": {
                "title": "FG length outliers",
                "description": "Number of FastaGuard length outlier records",
                "min": 0,
                "scale": "YlOrBr",
            },
            "composite_anomaly_count": {
                "title": "FG composite",
                "description": "Number of records with multiple FastaGuard anomaly signals",
                "min": 0,
                "scale": "Reds",
            },
            "n50": {
                "title": "FG N50",
                "description": "FastaGuard assembly N50",
                "hidden": True,
                "min": 0,
                "scale": "Blues",
            },
            "n_percent": {
                "title": "FG N%",
                "description": "FastaGuard global N percentage",
                "hidden": True,
                "min": 0,
                "max": 100,
                "suffix": "%",
                "scale": "OrRd",
            },
        }
```

- [ ] **Step 5: Document strict-mode verification**

Add to `integrations/multiqc/README.md`:

````markdown
## Verification

Run the plugin against example reports in strict mode:

```bash
cd integrations/multiqc
python -m pip install -e .
cd ../..
multiqc --strict examples/reports
```
````

- [ ] **Step 6: Verify parser tests and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
git diff --check
```

Expected: Python tests pass.

```bash
git add integrations/multiqc/src/fastaguard_multiqc/parser.py integrations/multiqc/src/fastaguard_multiqc/multiqc_module.py integrations/multiqc/README.md tests/python/test_adoption_assets.py
git commit -m "feat: expand MultiQC FastaGuard summary"
```

## Task 5: Add v0.2 Contract Fields To Models

**Files:**
- Modify: `src/models.rs`
- Modify: `src/cli.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing contract test for provenance and routing hints**

Modify `tests/cli.rs` with:

```rust
#[test]
fn report_includes_v0_2_provenance_and_routing_hints() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "v02_contract");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--out",
    ])
    .arg(&outputs.html)
    .arg("--json")
    .arg(&outputs.json)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .success();

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.2.0"));
    assert!(report["provenance"]["command"].as_str().unwrap().contains("fastaguard"));
    assert!(report["provenance"]["started_at"].as_str().unwrap().ends_with('Z'));
    assert!(report["provenance"]["completed_at"].as_str().unwrap().ends_with('Z'));
    assert!(report["provenance"]["duration_ms"].as_u64().is_some());
    assert!(report["provenance"]["input_size_bytes"].as_u64().unwrap() > 0);
    assert!(report["machine_summary"]["routing_hints"].as_array().unwrap().is_empty());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
cargo test --locked report_includes_v0_2_provenance_and_routing_hints
```

Expected: FAIL because schema version and provenance fields are not present.

- [ ] **Step 3: Add runtime context to `RunConfig`**

Modify `src/cli.rs`:

```rust
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub input: PathBuf,
    pub profile: String,
    pub outputs: OutputPaths,
    pub rules: RuleConfig,
    pub thresholds: ThresholdOverrides,
    pub threads: usize,
    pub command: String,
    pub started_at: String,
}
```

In `to_run_config`, set:

```rust
            command: std::env::args().collect::<Vec<_>>().join(" "),
            started_at: current_utc_timestamp(),
```

Add helper:

```rust
fn current_utc_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
```

Add `chrono` to `Cargo.toml`:

```toml
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
```

- [ ] **Step 4: Extend model structs**

Modify `src/models.rs`:

```rust
pub const SCHEMA_VERSION: &str = "0.2.0";
```

Extend `MachineSummary`:

```rust
pub struct MachineSummary {
    pub verdict: VerdictStatus,
    pub safe_for_downstream: bool,
    pub top_findings: Vec<String>,
    pub recommended_next_tools: Vec<RecommendedTool>,
    pub routing_hints: Vec<RoutingHint>,
}
```

Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingHint {
    pub condition: String,
    pub suggested_route: String,
    pub requires_external_database: bool,
}
```

Extend `Provenance`:

```rust
pub struct Provenance {
    pub profile: String,
    pub threads: usize,
    pub fail_on: Vec<String>,
    pub thresholds: ProvenanceThresholds,
    pub command: String,
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
    pub input_size_bytes: u64,
}
```

Extend `Finding`:

```rust
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub category: FindingCategory,
    pub confidence: FindingConfidence,
    pub requires_followup_tool: bool,
    pub profile: String,
    pub affected_count: u64,
    pub affected_fraction: f64,
    pub message: String,
    pub why_it_matters: String,
    pub suggested_next_step: String,
    pub evidence: FindingEvidence,
    pub actions: Vec<FindingAction>,
}
```

Add enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Validity,
    Structure,
    Composition,
    Duplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingConfidence {
    High,
    Moderate,
    Low,
}
```

- [ ] **Step 5: Build provenance from config**

Replace `build_provenance` in `src/models.rs` with behavior that uses `std::fs::metadata`:

```rust
fn build_provenance(config: &RunConfig, profile: &ProfileConfig) -> Provenance {
    let completed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let input_size_bytes = std::fs::metadata(&config.input)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    Provenance {
        profile: profile.name.clone(),
        threads: config.threads,
        fail_on: config.rules.fail_on.iter().cloned().collect(),
        thresholds: ProvenanceThresholds {
            high_n_sequence_fraction: profile.high_n_sequence_fraction,
            high_global_n_fraction: profile.high_global_n_fraction,
            min_contig_length: profile.min_contig_length,
            max_gap_run: profile.max_gap_run,
            gc_outlier_zscore: profile.gc_outlier_zscore,
        },
        command: config.command.clone(),
        started_at: config.started_at.clone(),
        completed_at,
        duration_ms: 0,
        input_size_bytes,
    }
}
```

Set `duration_ms` to `0` in this task. A later focused enhancement can measure it in `main.rs` without changing the contract.

- [ ] **Step 6: Update test report builders**

Every test helper constructing `MachineSummary`, `Provenance`, or `Finding` must include the new fields. Use:

```rust
routing_hints: Vec::new(),
command: "fastaguard input.fa".to_string(),
started_at: "2026-05-23T00:00:00Z".to_string(),
completed_at: "2026-05-23T00:00:00Z".to_string(),
duration_ms: 0,
input_size_bytes: 100,
category: FindingCategory::Validity,
confidence: FindingConfidence::High,
requires_followup_tool: false,
```

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo fmt --check
cargo test --locked report_includes_v0_2_provenance_and_routing_hints
cargo test --locked
```

Expected: tests pass.

Commit:

```bash
git add Cargo.toml Cargo.lock src/cli.rs src/models.rs tests/cli.rs src/report/multiqc.rs src/report/tsv.rs
git commit -m "feat: add v0.2 report provenance"
```

## Task 6: Add Outlier Stats Helpers

**Files:**
- Modify: `src/stats/outliers.rs`

- [ ] **Step 1: Add tests for robust length outliers**

Modify `src/stats/outliers.rs` test module:

```rust
    #[test]
    fn iqr_finds_low_and_high_length_outliers() {
        let values = vec![100, 101, 102, 103, 104, 105, 10_000];
        let outliers = iqr_outlier_indices(&values, 1.5);
        assert_eq!(outliers, vec![6]);
    }

    #[test]
    fn iqr_returns_empty_for_short_or_flat_inputs() {
        assert_eq!(iqr_outlier_indices(&[100, 101], 1.5), Vec::<usize>::new());
        assert_eq!(iqr_outlier_indices(&[100, 100, 100, 100], 1.5), Vec::<usize>::new());
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run:

```bash
cargo test --locked stats::outliers::tests::iqr_finds_low_and_high_length_outliers
```

Expected: FAIL because `iqr_outlier_indices` does not exist.

- [ ] **Step 3: Implement IQR helper**

Add to `src/stats/outliers.rs`:

```rust
pub fn iqr_outlier_indices(values: &[u64], multiplier: f64) -> Vec<usize> {
    if values.len() < 4 || !multiplier.is_finite() || multiplier <= 0.0 {
        return Vec::new();
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let q1 = percentile(&sorted, 0.25);
    let q3 = percentile(&sorted, 0.75);
    let iqr = q3 - q1;
    if iqr <= 0.0 {
        return Vec::new();
    }

    let lower = q1 - multiplier * iqr;
    let upper = q3 + multiplier * iqr;

    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let value = *value as f64;
            (value < lower || value > upper).then_some(index)
        })
        .collect()
}

fn percentile(sorted: &[u64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = quantile * (sorted.len().saturating_sub(1) as f64);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower] as f64
    } else {
        let weight = rank - lower as f64;
        sorted[lower] as f64 * (1.0 - weight) + sorted[upper] as f64 * weight
    }
}
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo fmt --check
cargo test --locked stats::outliers
```

Expected: outlier helper tests pass.

Commit:

```bash
git add src/stats/outliers.rs
git commit -m "feat: add robust length outlier helper"
```

## Task 7: Promote Assembly Outliers To Findings

**Files:**
- Modify: `src/metrics.rs`
- Modify: `src/findings.rs`
- Modify: `src/models.rs`
- Modify: `src/profile.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing CLI test for GC and length findings**

Modify `tests/cli.rs` with:

```rust
#[test]
fn assembly_outliers_are_promoted_to_findings_without_fail_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("outliers.fa");
    std::fs::write(
        &input,
        [
            ">normal_1\nAACCGGTTAACCGGTT\n",
            ">normal_2\nAACCGGTTAACCGGTT\n",
            ">normal_3\nAACCGGTTAACCGGTT\n",
            ">normal_4\nAACCGGTTAACCGGTT\n",
            ">normal_5\nAACCGGTTAACCGGTT\n",
            ">normal_6\nAACCGGTTAACCGGTT\n",
            ">gc_high\nGGGGGGGGGGGGGGGG\n",
            ">very_long\nAACCGGTTAACCGGTTAACCGGTTAACCGGTTAACCGGTTAACCGGTT\n",
        ]
        .join(""),
    )
    .unwrap();
    let outputs = output_paths(&temp_dir, "outlier_findings");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .args(["--min-contig-length", "1", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(1);

    let report = read_json(&outputs.json);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert!(finding_ids(&report).contains(&"gc_outliers".to_string()));
    assert!(finding_ids(&report).contains(&"length_outliers".to_string()));
    assert_eq!(report["findings"][0]["category"].is_string(), true);
}
```

Add helper near other test helpers:

```rust
fn finding_ids(report: &Value) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["id"].as_str().unwrap().to_string())
        .collect()
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --locked assembly_outliers_are_promoted_to_findings_without_fail_by_default
```

Expected: FAIL because new finding IDs are not emitted.

- [ ] **Step 3: Extend sequence summary signals**

Modify `SequenceSummary` in `src/metrics.rs`:

```rust
pub gc_outlier: bool,
pub length_outlier: bool,
pub composite_anomaly: bool,
pub gc_zscore: Option<f64>,
```

Set defaults in `SequenceSummaryBuilder::finish()`:

```rust
gc_outlier: false,
length_outlier: false,
composite_anomaly: false,
gc_zscore: None,
```

- [ ] **Step 4: Mark outlier signals after sequence collection**

In `MetricsAccumulator::finish`, after `self.lengths.sort_unstable();` and before returning `AssemblyMetrics`, call:

```rust
mark_gc_outliers(&mut self.sequences, self.profile.gc_outlier_zscore);
mark_length_outliers(&mut self.sequences);
mark_composite_anomalies(&mut self.sequences);
```

Add helpers in `src/metrics.rs`:

```rust
fn mark_gc_outliers(sequences: &mut [SequenceSummary], threshold: f64) {
    let values = sequences.iter().map(|sequence| sequence.gc_percent).collect::<Vec<_>>();
    let indices = crate::stats::outliers::zscore_outlier_indices(&values, threshold);
    let mean = if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    };
    let variance = if values.is_empty() {
        0.0
    } else {
        values.iter().map(|value| (value - mean) * (value - mean)).sum::<f64>()
            / values.len() as f64
    };
    let stddev = variance.sqrt();
    for index in indices {
        if let Some(sequence) = sequences.get_mut(index) {
            sequence.gc_outlier = true;
            if stddev > 0.0 {
                sequence.gc_zscore = Some(round2((sequence.gc_percent - mean) / stddev));
            }
        }
    }
}

fn mark_length_outliers(sequences: &mut [SequenceSummary]) {
    let values = sequences.iter().map(|sequence| sequence.length).collect::<Vec<_>>();
    for index in crate::stats::outliers::iqr_outlier_indices(&values, 1.5) {
        if let Some(sequence) = sequences.get_mut(index) {
            sequence.length_outlier = true;
        }
    }
}

fn mark_composite_anomalies(sequences: &mut [SequenceSummary]) {
    for sequence in sequences {
        let signal_count = [
            sequence.gc_outlier,
            sequence.length_outlier,
            sequence.n_fraction >= 0.20,
            sequence.duplicate_sequence,
            sequence.invalid_count > 0,
            sequence.max_gap_run > 100,
        ]
        .into_iter()
        .filter(|flag| *flag)
        .count();
        sequence.composite_anomaly = signal_count >= 2;
    }
}
```

- [ ] **Step 5: Add new findings**

Modify `src/findings.rs`:

```rust
    add_outlier_finding(
        &mut findings,
        "gc_outliers",
        Severity::Major,
        profile,
        metrics,
        |sequence| sequence.gc_outlier,
        "GC composition outlier",
        FindingText {
            message: format!("{} records have GC composition far from the assembly background.", count_sequences(metrics, |sequence| sequence.gc_outlier)),
            why_it_matters: "Unusual GC can indicate contamination, cobionts, plasmids, assembly artifacts, or real biological variation.",
            suggested_next_step: "Inspect flagged records and consider BlobToolKit, sourmash, Kraken, or related taxonomic checks if the pattern is strong.",
        },
    );
```

Add helper:

```rust
fn count_sequences(
    metrics: &AssemblyMetrics,
    predicate: impl Fn(&SequenceSummary) -> bool,
) -> u64 {
    metrics.sequences.iter().filter(|sequence| predicate(sequence)).count() as u64
}

fn add_outlier_finding(
    findings: &mut Vec<Finding>,
    id: &str,
    severity: Severity,
    profile: &ProfileConfig,
    metrics: &AssemblyMetrics,
    predicate: impl Fn(&SequenceSummary) -> bool,
    reason: &str,
    text: FindingText<'_>,
) {
    let affected_count = count_sequences(metrics, predicate);
    if affected_count == 0 {
        return;
    }
    findings.push(finding(
        id,
        severity,
        profile,
        affected_count,
        affected_fraction(affected_count, metrics.sequence_count),
        evidence_for_sequences(
            affected_count,
            metrics.sequences.iter().filter(|sequence| match id {
                "gc_outliers" => sequence.gc_outlier,
                "length_outliers" => sequence.length_outlier,
                "composite_anomalies" => sequence.composite_anomaly,
                _ => false,
            }),
            reason,
            EvidenceKind::Outlier,
        ),
        text,
    ));
}
```

Add `EvidenceKind::Outlier` and set evidence fields:

```rust
        EvidenceKind::Outlier => {
            record.gc_percent = Some(sequence.gc_percent);
            record.n_fraction = Some(round2(sequence.n_fraction));
            record.n_percent = Some(round2(sequence.n_fraction * 100.0));
        }
```

Add these two calls after the `gc_outliers` call:

```rust
    add_outlier_finding(
        &mut findings,
        "length_outliers",
        Severity::Minor,
        profile,
        metrics,
        |sequence| sequence.length_outlier,
        "length outlier",
        FindingText {
            message: format!(
                "{} records have lengths far from the assembly distribution.",
                count_sequences(metrics, |sequence| sequence.length_outlier)
            ),
            why_it_matters: "Extreme record lengths may be valid, but they should be visible before production use.",
            suggested_next_step: "Inspect length outlier records and confirm they are expected for this assembly.",
        },
    );

    add_outlier_finding(
        &mut findings,
        "composite_anomalies",
        Severity::Major,
        profile,
        metrics,
        |sequence| sequence.composite_anomaly,
        "multiple anomaly signals",
        FindingText {
            message: format!(
                "{} records have multiple FastaGuard anomaly signals.",
                count_sequences(metrics, |sequence| sequence.composite_anomaly)
            ),
            why_it_matters: "Records with multiple independent signals are higher priority for manual or downstream triage.",
            suggested_next_step: "Prioritize these records for inspection before running heavier assembly QC or taxonomy workflows.",
        },
    );
```

- [ ] **Step 6: Add action mappings**

Extend `finding_actions` in `src/models.rs`:

```rust
        "gc_outliers" => vec![
            action(
                "inspect_records",
                "GC outlier records",
                "Composition anomalies should be reviewed before interpreting downstream QC.",
                "BlobToolKit",
                true,
            ),
            action(
                "compare_kmers_or_taxonomy",
                "composition outlier records",
                "External evidence can help distinguish contamination, cobionts, plasmids, and biological variation.",
                "sourmash",
                true,
            ),
        ],
        "length_outliers" => vec![action(
            "inspect_records",
            "length outlier records",
            "Extreme record lengths may be valid, but should be visible before production use.",
            "seqkit",
            false,
        )],
        "composite_anomalies" => vec![action(
            "prioritize_records",
            "records with multiple anomaly signals",
            "Records with multiple independent signals are better candidates for manual or downstream triage.",
            "BlobToolKit",
            true,
        )],
```

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo fmt --check
cargo test --locked assembly_outliers_are_promoted_to_findings_without_fail_by_default
cargo test --locked
```

Expected: tests pass.

Commit:

```bash
git add src/metrics.rs src/findings.rs src/models.rs src/profile.rs tests/cli.rs
git commit -m "feat: add assembly outlier findings"
```

## Task 8: Update Contract Schema, Catalog, TSV, MultiQC, HTML, And Goldens

**Files:**
- Modify: `schema/fastaguard.schema.json`
- Modify: `schema/finding-catalog.json`
- Modify: `src/report/tsv.rs`
- Modify: `src/report/multiqc.rs`
- Modify: `src/report/html.rs`
- Modify: `tests/golden/*.json`
- Modify: `examples/reports/**`
- Test: `tests/schema_contract.rs`
- Test: `tests/cli.rs`

- [ ] **Step 1: Add failing schema expectations**

Modify `tests/cli.rs`:

```rust
#[test]
fn contract_finding_catalog_includes_v0_2_outlier_findings() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--finding-catalog")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""schema_version": "0.2.0""#))
        .stdout(predicate::str::contains(r#""gc_outliers""#))
        .stdout(predicate::str::contains(r#""length_outliers""#))
        .stdout(predicate::str::contains(r#""composite_anomalies""#));
}
```

- [ ] **Step 2: Update TSV metrics**

In `src/report/tsv.rs`, add:

```rust
    write_metric(&mut writer, "gc_outlier_count", count_finding(report, "gc_outliers"))?;
    write_metric(&mut writer, "length_outlier_count", count_finding(report, "length_outliers"))?;
    write_metric(
        &mut writer,
        "composite_anomaly_count",
        count_finding(report, "composite_anomalies"),
    )?;
```

Add helper:

```rust
fn count_finding(report: &FastaguardReport, id: &str) -> u64 {
    report
        .findings
        .iter()
        .find(|finding| finding.id == id)
        .map(|finding| finding.affected_count)
        .unwrap_or(0)
}
```

- [ ] **Step 3: Update MultiQC custom content**

In `src/report/multiqc.rs`, extend `MultiqcSummaryRow`:

```rust
    duplicate_id_count: u64,
    invalid_sequence_count: u64,
    high_n_sequence_count: u64,
    tiny_contig_count: u64,
    max_gap_run: u64,
    gc_outlier_count: u64,
    length_outlier_count: u64,
    composite_anomaly_count: u64,
```

Set values in `summary_row` using `report.summary` and the `count_finding` helper.

- [ ] **Step 4: Update schema**

Modify `schema/fastaguard.schema.json` so:

```json
"schema_version": {
  "const": "0.2.0"
}
```

Add `routing_hints` to `machine_summary.required` and define a `routing_hint` object in `$defs`:

```json
"routing_hint": {
  "type": "object",
  "required": ["condition", "suggested_route", "requires_external_database"],
  "properties": {
    "condition": { "type": "string" },
    "suggested_route": { "type": "string" },
    "requires_external_database": { "type": "boolean" }
  }
}
```

Add provenance required fields:

```json
"command",
"started_at",
"completed_at",
"duration_ms",
"input_size_bytes"
```

Add finding required fields:

```json
"category",
"confidence",
"requires_followup_tool"
```

Allowed `category` values:

```json
["validity", "structure", "composition", "duplication"]
```

Allowed `confidence` values:

```json
["high", "moderate", "low"]
```

- [ ] **Step 5: Update finding catalog**

Set top-level versions:

```json
"schema_version": "0.2.0",
"catalog_version": "0.2.0"
```

Add entries for `gc_outliers`, `length_outliers`, and `composite_anomalies` with `default_severity`, `meaning`, `why_it_matters`, `suggested_actions`, `recommended_next_tools`, and `cannot_conclude`.

- [ ] **Step 6: Regenerate golden and example reports**

Run:

```bash
cargo run --locked -- testdata/valid_assembly.fa --min-contig-length 1 --out examples/reports/assembly_pass/fastaguard_report.html --json examples/reports/assembly_pass/fastaguard.json --tsv examples/reports/assembly_pass/fastaguard.tsv --multiqc examples/reports/assembly_pass/fastaguard_mqc.json
cargo run --locked -- testdata/problem_assembly.fa --out examples/reports/assembly_fail/fastaguard_report.html --json examples/reports/assembly_fail/fastaguard.json --tsv examples/reports/assembly_fail/fastaguard.tsv --multiqc examples/reports/assembly_fail/fastaguard_mqc.json || test "$?" = "1"
cp examples/reports/assembly_pass/fastaguard.json tests/golden/valid_assembly.json
cp examples/reports/assembly_fail/fastaguard.json tests/golden/problem_assembly.json
cargo run --locked -- testdata/invalid_empty_record.fa --json tests/golden/invalid_empty_record.json --out /tmp/fastaguard_invalid.html --tsv /tmp/fastaguard_invalid.tsv --multiqc /tmp/fastaguard_invalid_mqc.json || test "$?" = "2"
```

Expected: pass fixture exits 0, problem fixture exits 1, invalid fixture exits 2.

- [ ] **Step 7: Verify schema, CLI, Python, and commit**

Run:

```bash
cargo fmt --check
cargo test --locked
python3 -m unittest discover tests/python -v
git diff --check
```

Expected: all pass.

Commit:

```bash
git add schema/fastaguard.schema.json schema/finding-catalog.json src/report/tsv.rs src/report/multiqc.rs src/report/html.rs tests/cli.rs tests/schema_contract.rs tests/golden examples/reports integrations/multiqc tests/python
git commit -m "feat: update v0.2 report contract outputs"
```

## Task 9: Add Benchmark And Evidence Page

**Files:**
- Modify: `docs/benchmarking.md`
- Modify: `README.md`
- Test: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Add docs test for evidence page content**

Modify `tests/python/test_adoption_assets.py`:

```python
    def test_benchmarking_docs_include_v0_2_evidence_topics(self):
        text = (ROOT / "docs" / "benchmarking.md").read_text()

        self.assertIn("duplicate IDs", text)
        self.assertIn("invalid characters", text)
        self.assertIn("high-N", text)
        self.assertIn("GC outliers", text)
        self.assertIn("QUAST", text)
        self.assertIn("BUSCO", text)
        self.assertIn("BlobToolKit", text)
```

- [ ] **Step 2: Run test to verify current gap**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_benchmarking_docs_include_v0_2_evidence_topics -v
```

Expected: FAIL if the current benchmarking page lacks the v0.2 evidence topics.

- [ ] **Step 3: Update benchmarking document**

Add this section to `docs/benchmarking.md`:

```markdown
## v0.2 Evidence Targets

FastaGuard should prove four preflight categories with small reproducible
fixtures:

| Evidence case | What FastaGuard catches | Why it should run before heavier tools |
| --- | --- | --- |
| duplicate IDs | repeated FASTA identifiers | prevents workflow joins, indexes, and annotations from becoming ambiguous |
| invalid characters | non-IUPAC sequence symbols | prevents downstream parser and aligner failures |
| high-N | ambiguous scaffolds and gap-heavy records | prevents low-confidence mapping and annotation from being treated as clean input |
| GC outliers | composition-anomalous records | routes suspicious records to BlobToolKit, sourmash, Kraken, or other follow-up tools |

FastaGuard should not replace QUAST, BUSCO, or BlobToolKit. It should make their
inputs safer and make obvious FASTA-level problems visible before those tools run.
```

- [ ] **Step 4: Link from README**

Ensure `README.md` documentation list contains:

```markdown
- [Benchmarking](docs/benchmarking.md)
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets -v
git diff --check
```

Expected: tests pass.

Commit:

```bash
git add docs/benchmarking.md README.md tests/python/test_adoption_assets.py
git commit -m "docs: add v0.2 benchmark evidence targets"
```

## Task 10: Prepare v0.2.0 Release Notes And Metadata

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `packaging/bioconda/meta.yaml`
- Create: `docs/releases/v0.2.0.md`
- Modify: `README.md`
- Test: `tests/python/test_release_metadata.py`

- [ ] **Step 1: Add failing metadata test for v0.2.0**

Modify `tests/python/test_release_metadata.py`:

```python
    def test_v0_2_0_release_notes_exist_when_version_bumps(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        if cargo["package"]["version"] != "0.2.0":
            self.skipTest("v0.2.0 version bump not started")

        notes = ROOT / "docs" / "releases" / "v0.2.0.md"
        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.2.0", text)
        self.assertIn("Assembly Trust", text)
        self.assertIn("Pipeline Adoption", text)
```

- [ ] **Step 2: Bump Rust package version**

Modify `Cargo.toml`:

```toml
version = "0.2.0"
```

Run:

```bash
cargo update -p fastaguard --precise 0.2.0
```

Expected: `Cargo.lock` package version updates to `0.2.0`.

- [ ] **Step 3: Create release notes**

Create `docs/releases/v0.2.0.md`:

````markdown
# FastaGuard v0.2.0

FastaGuard v0.2.0 is the Assembly Trust + Pipeline Adoption release.

## Highlights

- Adds explainable GC, length, and composite assembly outlier findings.
- Expands the machine-readable JSON contract with richer provenance, finding
  taxonomy, and routing hints.
- Hardens MultiQC plugin support for many FastaGuard reports.
- Adds Snakemake Bioconda environment starter material.
- Updates benchmark and evidence documentation.

## Install

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

## Positioning

FastaGuard still runs before QUAST, BUSCO, BlobToolKit, annotation, and
submission. It does not replace downstream biological completeness, assembly
correctness, or contamination workflows.

## Known Limits

- v0.2.0 remains assembly-focused.
- Composition outliers are not contamination calls.
- External taxonomy or k-mer database checks remain follow-up steps.
```
````

- [ ] **Step 4: Keep Bioconda recipe staged but not submitted**

In `packaging/bioconda/meta.yaml`, update:

```jinja
{% set version = "0.2.0" %}
```

Leave the SHA256 unchanged until a public `v0.2.0` source archive exists. Add a comment above `source:`:

```yaml
# Update sha256 after the v0.2.0 GitHub source archive is published.
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --check
cargo test --locked
python3 -m unittest discover tests/python -v
git diff --check
```

Expected: all pass.

Commit:

```bash
git add Cargo.toml Cargo.lock packaging/bioconda/meta.yaml docs/releases/v0.2.0.md README.md tests/python/test_release_metadata.py
git commit -m "chore: prepare v0.2.0 release metadata"
```

## Task 11: Full Verification Gate

**Files:**
- No source file changes expected unless a verification failure reveals a real issue.

- [ ] **Step 1: Run Rust formatting, tests, and clippy**

Run:

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Expected: all pass.

- [ ] **Step 2: Run Python tests**

Run:

```bash
python3 -m unittest discover tests/python -v
```

Expected: all pass.

- [ ] **Step 3: Run MultiQC strict check if MultiQC is installed**

Run:

```bash
if command -v multiqc >/dev/null 2>&1; then
  cd integrations/multiqc
  python3 -m pip install -e .
  cd ../..
  multiqc --strict examples/reports
else
  echo "multiqc not installed; skipping strict check"
fi
```

Expected: strict check passes when MultiQC is installed; otherwise the skip is explicit.

- [ ] **Step 4: Run Docker smoke test**

Run:

```bash
docker build -t fastaguard:local .
docker run --rm fastaguard:local --schema >/tmp/fastaguard_schema.json
docker run --rm -v "$PWD:/data" fastaguard:local /data/testdata/valid_assembly.fa --out /data/target/docker_fastaguard.html --json /data/target/docker_fastaguard.json --tsv /data/target/docker_fastaguard.tsv --multiqc /data/target/docker_fastaguard_mqc.json
```

Expected: Docker image builds and both container commands exit 0.

- [ ] **Step 5: Run diff hygiene**

Run:

```bash
git diff --check
rg -n "[ \t]+$" README.md docs examples packaging tests src schema integrations || true
git status --short --branch
```

Expected: no whitespace findings. `git status` shows a clean branch after all intended commits.

## Task 12: Open PR And Preserve Bioconda Update Path

**Files:**
- No source changes expected.

- [ ] **Step 1: Push branch**

Run:

```bash
git status --short --branch
git push origin main
```

Expected: `main` pushes with the v0.2 commits if direct push is acceptable for this repo. If branch protection blocks direct push, create a branch:

```bash
git switch -c fastaguard-v0.2-adoption
git push -u origin fastaguard-v0.2-adoption
```

- [ ] **Step 2: Create PR if using a branch**

Run:

```bash
gh pr create --repo ehsanestaji/FastaGuard --title "FastaGuard v0.2 adoption and outlier findings" --body "Implements the v0.2 Assembly Trust + Pipeline Adoption plan: outlier findings, richer report contract, MultiQC hardening, workflow starter polish, and benchmark evidence docs."
```

Expected: GitHub PR URL is printed.

- [ ] **Step 3: Record Bioconda follow-up**

After the GitHub `v0.2.0` release exists, update the Bioconda recipe SHA256 and open an upstream Bioconda PR. Do not submit a Bioconda update before the public source archive exists.
