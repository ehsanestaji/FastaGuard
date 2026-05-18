# FastaGuard Assembly MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the v0.1 Rust CLI that streams assembly FASTA files, computes FASTA-level QC metrics, produces explainable verdicts, and writes JSON, TSV, HTML, and MultiQC-compatible outputs.

**Architecture:** The implementation is a Rust binary plus library. The CLI parses options, builds an assembly profile, streams records through a parser, accumulates compact per-sequence summaries, derives findings and a verdict, writes reports, and exits with documented pipeline codes.

**Tech Stack:** Rust 2021, `clap`, `serde`, `serde_json`, `anyhow`, `thiserror`, `flate2`, `sha2`, `hex`, `assert_cmd`, `predicates`, `tempfile`.

---

## File Structure

Create these files:

- `Cargo.toml`: crate metadata, runtime dependencies, and test dependencies.
- `src/main.rs`: process entrypoint that maps CLI results to process exit codes.
- `src/lib.rs`: public module exports and `run_check` orchestration.
- `src/cli.rs`: `clap` arguments, output path defaults, and rule parsing.
- `src/models.rs`: stable JSON contract structs.
- `src/profile.rs`: assembly thresholds and fail/warn rule configuration.
- `src/parser.rs`: streaming FASTA reader for plain and gzipped input.
- `src/metrics.rs`: per-record summaries, duplicate detection, and assembly aggregate metrics.
- `src/findings.rs`: findings engine and verdict logic.
- `src/report/mod.rs`: report module exports.
- `src/report/json.rs`: JSON writer.
- `src/report/tsv.rs`: TSV writer.
- `src/report/multiqc.rs`: MultiQC-compatible JSON writer.
- `src/report/html.rs`: self-contained static HTML writer.
- `src/stats/mod.rs`: stats module exports.
- `src/stats/nxx.rs`: N50/N90/L50/L90 calculations.
- `src/stats/composition.rs`: percentage helpers.
- `src/stats/outliers.rs`: simple outlier detection helpers.
- `tests/cli.rs`: end-to-end CLI tests.
- `testdata/valid_assembly.fa`: small valid assembly fixture.
- `testdata/problem_assembly.fa`: assembly fixture with duplicate IDs, high Ns, invalid chars, and tiny contigs.

Modify these files:

- `README.md`: add install, quickstart, and v0.1 usage once the CLI exists.
- `docs/mvp-spec.md`: add a short implementation status section.
- `docs/output-contract.md`: align example JSON with the actual serialized fields if names change during implementation.

---

## Task 1: Scaffold The Rust Crate

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Initialize the crate**

Run:

```bash
cargo init --bin --name fastaguard .
```

Expected:

```text
Created binary (application) package
```

- [ ] **Step 2: Replace `Cargo.toml` with pinned project metadata**

Replace `Cargo.toml` with:

```toml
[package]
name = "fastaguard"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "FASTA preflight QC for assembly pipelines"
repository = "https://github.com/ehsanestaji/FastaGuard"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
flate2 = "1"
hex = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "1"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 3: Create the temporary binary entrypoint**

Replace `src/main.rs` with:

```rust
fn main() {
    println!("FastaGuard");
}
```

- [ ] **Step 4: Run the compiler**

Run:

```bash
cargo check
```

Expected:

```text
Finished
```

- [ ] **Step 5: Commit the crate scaffold**

Run:

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: scaffold Rust crate"
```

Expected:

```text
[main <hash>] chore: scaffold Rust crate
```

---

## Task 2: Define CLI, Profile, And Output Models

**Files:**
- Create: `src/cli.rs`
- Create: `src/profile.rs`
- Create: `src/models.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the model and CLI tests**

Create `tests/cli.rs` with:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_preflight_positioning() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("FASTA preflight QC"));
}
```

- [ ] **Step 2: Create CLI argument parsing**

Create `src/cli.rs` with:

```rust
use anyhow::{anyhow, Result};
use clap::Parser;
use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::profile::ThresholdOverrides;

#[derive(Debug, Parser)]
#[command(name = "fastaguard")]
#[command(version)]
#[command(about = "FASTA preflight QC for assembly pipelines")]
pub struct Cli {
    /// Input FASTA file. Plain .fa/.fasta and gzipped .gz files are supported.
    pub input: PathBuf,

    /// QC profile. v0.1 supports assembly.
    #[arg(long, default_value = "assembly")]
    pub profile: String,

    /// HTML report path.
    #[arg(long, default_value = "fastaguard_report.html")]
    pub out: PathBuf,

    /// JSON report path.
    #[arg(long, default_value = "fastaguard.json")]
    pub json: PathBuf,

    /// TSV summary path.
    #[arg(long, default_value = "fastaguard.tsv")]
    pub tsv: PathBuf,

    /// MultiQC-compatible JSON path.
    #[arg(long, default_value = "fastaguard_multiqc.json")]
    pub multiqc: PathBuf,

    /// Comma-separated rule IDs that should fail the run when triggered.
    #[arg(long, value_delimiter = ',')]
    pub fail_on: Vec<String>,

    /// Maximum allowed global N fraction before a high_n_rate finding.
    #[arg(long)]
    pub max_n_rate: Option<f64>,

    /// Minimum contig length used for tiny_contigs finding.
    #[arg(long)]
    pub min_contig_length: Option<u64>,

    /// Worker thread count reserved for later parallel post-processing.
    #[arg(long, default_value_t = 1)]
    pub threads: usize,
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    pub input: PathBuf,
    pub profile: String,
    pub outputs: OutputPaths,
    pub rules: RuleConfig,
    pub thresholds: ThresholdOverrides,
    pub threads: usize,
}

#[derive(Debug, Clone)]
pub struct OutputPaths {
    pub html: PathBuf,
    pub json: PathBuf,
    pub tsv: PathBuf,
    pub multiqc: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub fail_on: BTreeSet<String>,
}

impl Cli {
    pub fn to_run_config(&self) -> Result<RunConfig> {
        if self.profile != "assembly" {
            return Err(anyhow!("unsupported profile '{}'; v0.1 supports assembly", self.profile));
        }
        if self.threads == 0 {
            return Err(anyhow!("--threads must be at least 1"));
        }

        Ok(RunConfig {
            input: self.input.clone(),
            profile: self.profile.clone(),
            outputs: OutputPaths {
                html: self.out.clone(),
                json: self.json.clone(),
                tsv: self.tsv.clone(),
                multiqc: self.multiqc.clone(),
            },
            rules: RuleConfig {
                fail_on: normalize_rules(&self.fail_on),
            },
            thresholds: ThresholdOverrides {
                max_n_rate: self.max_n_rate,
                min_contig_length: self.min_contig_length,
            },
            threads: self.threads,
        })
    }
}

fn normalize_rules(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
```

- [ ] **Step 3: Create profile thresholds**

Create `src/profile.rs` with:

```rust
#[derive(Debug, Clone, Copy)]
pub struct ThresholdOverrides {
    pub max_n_rate: Option<f64>,
    pub min_contig_length: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ProfileConfig {
    pub name: String,
    pub high_n_sequence_fraction: f64,
    pub high_global_n_fraction: f64,
    pub min_contig_length: u64,
    pub max_gap_run: u64,
    pub gc_outlier_zscore: f64,
}

impl ProfileConfig {
    pub fn assembly(overrides: ThresholdOverrides) -> Self {
        Self {
            name: "assembly".to_string(),
            high_n_sequence_fraction: 0.20,
            high_global_n_fraction: overrides.max_n_rate.unwrap_or(0.05),
            min_contig_length: overrides.min_contig_length.unwrap_or(200),
            max_gap_run: 100,
            gc_outlier_zscore: 3.0,
        }
    }
}
```

- [ ] **Step 4: Create stable output models**

Create `src/models.rs` with:

```rust
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const TOOL_NAME: &str = "FastaGuard";
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastaguardReport {
    pub schema_version: String,
    pub tool: ToolInfo,
    pub input: InputInfo,
    pub verdict: Verdict,
    pub summary: Summary,
    pub findings: Vec<Finding>,
    pub artifacts: Artifacts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputInfo {
    pub path: String,
    pub profile: String,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub status: VerdictStatus,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum VerdictStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Minor,
    Major,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub profile: String,
    pub affected_count: u64,
    pub affected_fraction: f64,
    pub message: String,
    pub why_it_matters: String,
    pub suggested_next_step: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub sequence_count: u64,
    pub total_length: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub n50: u64,
    pub n90: u64,
    pub l50: u64,
    pub l90: u64,
    pub gc_percent: f64,
    pub at_percent: f64,
    pub n_percent: f64,
    pub ambiguity_percent: f64,
    pub duplicate_id_count: u64,
    pub duplicate_sequence_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifacts {
    pub html: String,
    pub tsv: String,
    pub multiqc: String,
}
```

- [ ] **Step 5: Create the library module shell and CLI-aware entrypoint**

Create `src/lib.rs` with:

```rust
pub mod cli;
pub mod models;
pub mod profile;
```

Replace `src/main.rs` with:

```rust
use clap::Parser;
use fastaguard::cli::Cli;

fn main() {
    let _cli = Cli::parse();
    eprintln!("fastaguard implementation is not wired yet");
    std::process::exit(3);
}
```

- [ ] **Step 6: Run the help test**

Run:

```bash
cargo test help_mentions_preflight_positioning
```

Expected:

```text
test help_mentions_preflight_positioning ... ok
```

- [ ] **Step 7: Commit CLI and model contracts**

Run:

```bash
git add src/main.rs src/lib.rs src/cli.rs src/profile.rs src/models.rs tests/cli.rs
git commit -m "feat: define CLI and output contract models"
```

Expected:

```text
[main <hash>] feat: define CLI and output contract models
```

---

## Task 3: Implement Stats Helpers

**Files:**
- Create: `src/stats/mod.rs`
- Create: `src/stats/nxx.rs`
- Create: `src/stats/composition.rs`
- Create: `src/stats/outliers.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Wire the stats module and write N-stat tests**

Append this line to `src/lib.rs`:

```rust
pub mod stats;
```

Create `src/stats/mod.rs` with:

```rust
pub mod nxx;
```

Create `src/stats/nxx.rs` with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nx {
    pub nx: u64,
    pub lx: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_n50_and_l50_for_sorted_or_unsorted_lengths() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.50);
        assert_eq!(result, Nx { nx: 40, lx: 2 });
    }

    #[test]
    fn computes_n90_and_l90() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.90);
        assert_eq!(result, Nx { nx: 10, lx: 4 });
    }

    #[test]
    fn empty_lengths_return_zeroes() {
        assert_eq!(nx_lx(&[], 0.50), Nx { nx: 0, lx: 0 });
    }
}
```

- [ ] **Step 2: Run N-stat tests to verify failure**

Run:

```bash
cargo test stats::nxx
```

Expected:

```text
cannot find function `nx_lx` in this scope
```

- [ ] **Step 3: Add N-stat implementation**

Replace `src/stats/nxx.rs` with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nx {
    pub nx: u64,
    pub lx: u64,
}

pub fn nx_lx(lengths: &[u64], fraction: f64) -> Nx {
    if lengths.is_empty() {
        return Nx { nx: 0, lx: 0 };
    }

    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let total: u64 = sorted.iter().sum();
    let target = (total as f64 * fraction).ceil() as u64;
    let mut cumulative = 0_u64;

    for (index, length) in sorted.iter().enumerate() {
        cumulative += *length;
        if cumulative >= target {
            return Nx {
                nx: *length,
                lx: (index + 1) as u64,
            };
        }
    }

    Nx {
        nx: *sorted.last().unwrap_or(&0),
        lx: sorted.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_n50_and_l50_for_sorted_or_unsorted_lengths() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.50);
        assert_eq!(result, Nx { nx: 40, lx: 2 });
    }

    #[test]
    fn computes_n90_and_l90() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.90);
        assert_eq!(result, Nx { nx: 10, lx: 4 });
    }

    #[test]
    fn empty_lengths_return_zeroes() {
        assert_eq!(nx_lx(&[], 0.50), Nx { nx: 0, lx: 0 });
    }
}
```

- [ ] **Step 4: Add composition and outlier helpers**

Create `src/stats/composition.rs` with:

```rust
pub fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        round2((part as f64 / total as f64) * 100.0)
    }
}

pub fn fraction(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

pub fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_rounds_to_two_decimals() {
        assert_eq!(percent(1, 3), 33.33);
    }

    #[test]
    fn zero_total_is_zero() {
        assert_eq!(percent(4, 0), 0.0);
        assert_eq!(fraction(4, 0), 0.0);
    }
}
```

Create `src/stats/outliers.rs` with:

```rust
pub fn zscore_outlier_indices(values: &[f64], threshold: f64) -> Vec<usize> {
    if values.len() < 3 {
        return Vec::new();
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    let stddev = variance.sqrt();

    if stddev == 0.0 {
        return Vec::new();
    }

    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let z = (value - mean).abs() / stddev;
            (z >= threshold).then_some(index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_extreme_value() {
        let values = vec![50.0, 51.0, 49.0, 90.0, 50.5];
        assert_eq!(zscore_outlier_indices(&values, 1.5), vec![3]);
    }
}
```

Replace `src/stats/mod.rs` with:

```rust
pub mod composition;
pub mod nxx;
pub mod outliers;
```

- [ ] **Step 5: Run stats tests**

Run:

```bash
cargo test stats
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit stats helpers**

Run:

```bash
git add src/lib.rs src/stats
git commit -m "feat: add assembly statistics helpers"
```

Expected:

```text
[main <hash>] feat: add assembly statistics helpers
```

---

## Task 4: Implement Streaming FASTA Parser

**Files:**
- Create: `src/parser.rs`
- Create: `testdata/valid_assembly.fa`
- Create: `testdata/problem_assembly.fa`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add FASTA fixtures**

Create `testdata/valid_assembly.fa` with:

```text
>contig_1
ACGTACGTACGTAAAA
>contig_2 description
GGGGCCCCAAAATTTT
>contig_3
ACGTMRWSYKVHDBN
```

Create `testdata/problem_assembly.fa` with:

```text
>dup
ACGTACGT
>dup
NNNNNNNNNNNNNNNN
>tiny
ACGT
>bad_chars
ACGTXYZ
>gap_rich
AAAAANNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNCCCC
```

- [ ] **Step 2: Wire the parser module and write parser tests**

Append this line to `src/lib.rs`:

```rust
pub mod parser;
```

Create `src/parser.rs` with:

```rust
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastaRecord {
    pub id: String,
    pub header: String,
    pub sequence: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_multirecord_fasta() {
        let records = read_fasta(Path::new("testdata/valid_assembly.fa")).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].id, "contig_1");
        assert_eq!(records[1].id, "contig_2");
        assert_eq!(records[0].sequence, b"ACGTACGTACGTAAAA");
    }

    #[test]
    fn rejects_sequence_before_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, "ACGT\n>later\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("sequence before first header"));
    }

    #[test]
    fn rejects_empty_header_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, ">\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("empty FASTA header"));
    }
}
```

- [ ] **Step 3: Run parser tests to verify failure**

Run:

```bash
cargo test parser
```

Expected:

```text
cannot find function `read_fasta` in this scope
```

- [ ] **Step 4: Add parser implementation**

Replace `src/parser.rs` with:

```rust
use anyhow::{anyhow, Context, Result};
use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastaRecord {
    pub id: String,
    pub header: String,
    pub sequence: Vec<u8>,
}

pub fn read_fasta(path: &Path) -> Result<Vec<FastaRecord>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader: Box<dyn Read> = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gz"))
        .unwrap_or(false)
    {
        Box::new(MultiGzDecoder::new(file))
    } else {
        Box::new(file)
    };

    parse_reader(BufReader::new(reader))
}

fn parse_reader<R: BufRead>(reader: R) -> Result<Vec<FastaRecord>> {
    let mut records = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_id: Option<String> = None;
    let mut current_sequence: Vec<u8> = Vec::new();

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line_result.with_context(|| format!("failed to read line {line_number}"))?;
        let trimmed = line.trim_end_matches('\r');

        if trimmed.starts_with('>') {
            if let Some(header) = current_header.take() {
                records.push(FastaRecord {
                    id: current_id.take().unwrap(),
                    header,
                    sequence: std::mem::take(&mut current_sequence),
                });
            }

            let header = trimmed[1..].trim().to_string();
            if header.is_empty() {
                return Err(anyhow!("empty FASTA header at line {line_number}"));
            }
            let id = header
                .split_whitespace()
                .next()
                .ok_or_else(|| anyhow!("empty FASTA header at line {line_number}"))?
                .to_string();
            current_header = Some(header);
            current_id = Some(id);
        } else if trimmed.trim().is_empty() {
            continue;
        } else {
            if current_header.is_none() {
                return Err(anyhow!("sequence before first header at line {line_number}"));
            }
            current_sequence.extend(trimmed.trim().as_bytes());
        }
    }

    if let Some(header) = current_header.take() {
        records.push(FastaRecord {
            id: current_id.take().unwrap(),
            header,
            sequence: current_sequence,
        });
    }

    if records.is_empty() {
        return Err(anyhow!("input contains no FASTA records"));
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_multirecord_fasta() {
        let records = read_fasta(Path::new("testdata/valid_assembly.fa")).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].id, "contig_1");
        assert_eq!(records[1].id, "contig_2");
        assert_eq!(records[0].sequence, b"ACGTACGTACGTAAAA");
    }

    #[test]
    fn rejects_sequence_before_header() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, "ACGT\n>later\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("sequence before first header"));
    }

    #[test]
    fn rejects_empty_header_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.fa");
        std::fs::write(&path, ">\nACGT\n").unwrap();
        let error = read_fasta(&path).unwrap_err().to_string();
        assert!(error.contains("empty FASTA header"));
    }
}
```

- [ ] **Step 5: Run parser tests**

Run:

```bash
cargo test parser
```

Expected:

```text
test result: ok
```

- [ ] **Step 6: Commit parser and fixtures**

Run:

```bash
git add src/lib.rs src/parser.rs testdata
git commit -m "feat: add streaming FASTA parser"
```

Expected:

```text
[main <hash>] feat: add streaming FASTA parser
```

---

## Task 5: Implement Assembly Metrics

**Files:**
- Create: `src/metrics.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Wire the metrics module and write metrics tests**

Append this line to `src/lib.rs`:

```rust
pub mod metrics;
```

Create `src/metrics.rs` with:

```rust
use crate::parser::FastaRecord;
use crate::profile::ProfileConfig;

#[derive(Debug, Clone)]
pub struct AssemblyMetrics {
    pub sequence_count: u64,
    pub total_length: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub n50: u64,
    pub n90: u64,
    pub l50: u64,
    pub l90: u64,
    pub gc_percent: f64,
    pub at_percent: f64,
    pub n_percent: f64,
    pub ambiguity_percent: f64,
    pub duplicate_id_count: u64,
    pub duplicate_sequence_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ThresholdOverrides;

    fn profile() -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: Some(10),
        })
    }

    #[test]
    fn summarizes_valid_records() {
        let records = vec![
            FastaRecord { id: "a".into(), header: "a".into(), sequence: b"ACGTNN".to_vec() },
            FastaRecord { id: "b".into(), header: "b".into(), sequence: b"GGCC".to_vec() },
        ];
        let metrics = AssemblyMetrics::from_records(records, &profile());
        assert_eq!(metrics.sequence_count, 2);
        assert_eq!(metrics.total_length, 10);
        assert_eq!(metrics.n50, 6);
        assert_eq!(metrics.gc_percent, 60.0);
        assert_eq!(metrics.n_percent, 20.0);
    }

    #[test]
    fn detects_duplicate_ids_invalid_chars_tiny_contigs_and_gap_runs() {
        let records = vec![
            FastaRecord { id: "dup".into(), header: "dup".into(), sequence: b"ACGT".to_vec() },
            FastaRecord { id: "dup".into(), header: "dup second".into(), sequence: b"ACGT".to_vec() },
            FastaRecord { id: "bad".into(), header: "bad".into(), sequence: b"ACGTXYZ".to_vec() },
            FastaRecord { id: "gap".into(), header: "gap".into(), sequence: b"AAANNNNNCCCC".to_vec() },
        ];
        let metrics = AssemblyMetrics::from_records(records, &profile());
        assert_eq!(metrics.duplicate_id_count, 1);
        assert_eq!(metrics.duplicate_sequence_count, 1);
        assert_eq!(metrics.invalid_sequence_count, 1);
        assert_eq!(metrics.tiny_contig_count, 3);
        assert_eq!(metrics.max_gap_run, 5);
    }
}
```

- [ ] **Step 2: Run metrics tests to verify failure**

Run:

```bash
cargo test metrics
```

Expected:

```text
no function or associated item named `from_records`
```

- [ ] **Step 3: Add metrics implementation**

Replace `src/metrics.rs` with:

```rust
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use crate::parser::FastaRecord;
use crate::profile::ProfileConfig;
use crate::stats::composition::{fraction, percent, round2};
use crate::stats::nxx::nx_lx;

#[derive(Debug, Clone)]
pub struct SequenceSummary {
    pub id: String,
    pub length: u64,
    pub gc_count: u64,
    pub at_count: u64,
    pub n_count: u64,
    pub ambiguity_count: u64,
    pub invalid_count: u64,
    pub max_gap_run: u64,
    pub n_fraction: f64,
    pub gc_percent: f64,
}

#[derive(Debug, Clone)]
pub struct AssemblyMetrics {
    pub sequence_count: u64,
    pub total_length: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub n50: u64,
    pub n90: u64,
    pub l50: u64,
    pub l90: u64,
    pub gc_percent: f64,
    pub at_percent: f64,
    pub n_percent: f64,
    pub ambiguity_percent: f64,
    pub duplicate_id_count: u64,
    pub duplicate_sequence_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
    pub sequences: Vec<SequenceSummary>,
}

impl AssemblyMetrics {
    pub fn from_records(records: Vec<FastaRecord>, profile: &ProfileConfig) -> Self {
        let mut seen_ids = BTreeSet::new();
        let mut duplicate_id_count = 0_u64;
        let mut sequence_hashes: BTreeMap<String, u64> = BTreeMap::new();
        let mut duplicate_sequence_count = 0_u64;
        let mut lengths = Vec::new();
        let mut summaries = Vec::new();
        let mut gc_total = 0_u64;
        let mut at_total = 0_u64;
        let mut n_total = 0_u64;
        let mut ambiguity_total = 0_u64;
        let mut invalid_sequence_count = 0_u64;
        let mut high_n_sequence_count = 0_u64;
        let mut tiny_contig_count = 0_u64;
        let mut global_max_gap_run = 0_u64;

        for record in records {
            if !seen_ids.insert(record.id.clone()) {
                duplicate_id_count += 1;
            }

            let hash = sequence_hash(&record.sequence);
            let count = sequence_hashes.entry(hash).or_insert(0);
            if *count > 0 {
                duplicate_sequence_count += 1;
            }
            *count += 1;

            let summary = summarize_sequence(record, profile);
            lengths.push(summary.length);
            gc_total += summary.gc_count;
            at_total += summary.at_count;
            n_total += summary.n_count;
            ambiguity_total += summary.ambiguity_count;
            invalid_sequence_count += (summary.invalid_count > 0) as u64;
            high_n_sequence_count += (summary.n_fraction >= profile.high_n_sequence_fraction) as u64;
            tiny_contig_count += (summary.length < profile.min_contig_length) as u64;
            global_max_gap_run = global_max_gap_run.max(summary.max_gap_run);
            summaries.push(summary);
        }

        lengths.sort_unstable();
        let sequence_count = lengths.len() as u64;
        let total_length: u64 = lengths.iter().sum();
        let min_length = *lengths.first().unwrap_or(&0);
        let max_length = *lengths.last().unwrap_or(&0);
        let mean_length = if sequence_count == 0 {
            0.0
        } else {
            round2(total_length as f64 / sequence_count as f64)
        };
        let median_length = median(&lengths);
        let n50 = nx_lx(&lengths, 0.50);
        let n90 = nx_lx(&lengths, 0.90);

        Self {
            sequence_count,
            total_length,
            min_length,
            max_length,
            mean_length,
            median_length,
            n50: n50.nx,
            n90: n90.nx,
            l50: n50.lx,
            l90: n90.lx,
            gc_percent: percent(gc_total, total_length),
            at_percent: percent(at_total, total_length),
            n_percent: percent(n_total, total_length),
            ambiguity_percent: percent(ambiguity_total, total_length),
            duplicate_id_count,
            duplicate_sequence_count,
            invalid_sequence_count,
            high_n_sequence_count,
            tiny_contig_count,
            max_gap_run: global_max_gap_run,
            sequences: summaries,
        }
    }
}

fn summarize_sequence(record: FastaRecord, _profile: &ProfileConfig) -> SequenceSummary {
    let mut gc_count = 0_u64;
    let mut at_count = 0_u64;
    let mut n_count = 0_u64;
    let mut ambiguity_count = 0_u64;
    let mut invalid_count = 0_u64;
    let mut current_gap_run = 0_u64;
    let mut max_gap_run = 0_u64;

    for base in record.sequence.iter().map(|base| base.to_ascii_uppercase()) {
        match base {
            b'G' | b'C' => {
                gc_count += 1;
                current_gap_run = 0;
            }
            b'A' | b'T' | b'U' => {
                at_count += 1;
                current_gap_run = 0;
            }
            b'N' => {
                n_count += 1;
                ambiguity_count += 1;
                current_gap_run += 1;
                max_gap_run = max_gap_run.max(current_gap_run);
            }
            b'M' | b'R' | b'W' | b'S' | b'Y' | b'K' | b'V' | b'H' | b'D' | b'B' => {
                ambiguity_count += 1;
                current_gap_run = 0;
            }
            _ => {
                invalid_count += 1;
                current_gap_run = 0;
            }
        }
    }

    let length = record.sequence.len() as u64;
    SequenceSummary {
        id: record.id,
        length,
        gc_count,
        at_count,
        n_count,
        ambiguity_count,
        invalid_count,
        max_gap_run,
        n_fraction: fraction(n_count, length),
        gc_percent: percent(gc_count, length),
    }
}

fn sequence_hash(sequence: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sequence.iter().map(|base| base.to_ascii_uppercase()).collect::<Vec<u8>>());
    hex::encode(hasher.finalize())
}

fn median(lengths: &[u64]) -> f64 {
    if lengths.is_empty() {
        return 0.0;
    }

    let mid = lengths.len() / 2;
    if lengths.len() % 2 == 0 {
        round2((lengths[mid - 1] + lengths[mid]) as f64 / 2.0)
    } else {
        lengths[mid] as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ThresholdOverrides;

    fn profile() -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: Some(10),
        })
    }

    #[test]
    fn summarizes_valid_records() {
        let records = vec![
            FastaRecord { id: "a".into(), header: "a".into(), sequence: b"ACGTNN".to_vec() },
            FastaRecord { id: "b".into(), header: "b".into(), sequence: b"GGCC".to_vec() },
        ];
        let metrics = AssemblyMetrics::from_records(records, &profile());
        assert_eq!(metrics.sequence_count, 2);
        assert_eq!(metrics.total_length, 10);
        assert_eq!(metrics.n50, 6);
        assert_eq!(metrics.gc_percent, 60.0);
        assert_eq!(metrics.n_percent, 20.0);
    }

    #[test]
    fn detects_duplicate_ids_invalid_chars_tiny_contigs_and_gap_runs() {
        let records = vec![
            FastaRecord { id: "dup".into(), header: "dup".into(), sequence: b"ACGT".to_vec() },
            FastaRecord { id: "dup".into(), header: "dup second".into(), sequence: b"ACGT".to_vec() },
            FastaRecord { id: "bad".into(), header: "bad".into(), sequence: b"ACGTXYZ".to_vec() },
            FastaRecord { id: "gap".into(), header: "gap".into(), sequence: b"AAANNNNNCCCC".to_vec() },
        ];
        let metrics = AssemblyMetrics::from_records(records, &profile());
        assert_eq!(metrics.duplicate_id_count, 1);
        assert_eq!(metrics.duplicate_sequence_count, 1);
        assert_eq!(metrics.invalid_sequence_count, 1);
        assert_eq!(metrics.tiny_contig_count, 3);
        assert_eq!(metrics.max_gap_run, 5);
    }
}
```

- [ ] **Step 4: Run metrics tests**

Run:

```bash
cargo test metrics
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit assembly metrics**

Run:

```bash
git add src/lib.rs src/metrics.rs
git commit -m "feat: compute assembly FASTA metrics"
```

Expected:

```text
[main <hash>] feat: compute assembly FASTA metrics
```

---

## Task 6: Implement Findings And Verdicts

**Files:**
- Create: `src/findings.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Wire the findings module and write findings tests**

Append this line to `src/lib.rs`:

```rust
pub mod findings;
```

Create `src/findings.rs` with:

```rust
use crate::models::{Finding, Severity, VerdictStatus};

#[derive(Debug, Clone)]
pub struct Analysis {
    pub status: VerdictStatus,
    pub reasons: Vec<String>,
    pub findings: Vec<Finding>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::RuleConfig;
    use crate::metrics::AssemblyMetrics;
    use crate::profile::{ProfileConfig, ThresholdOverrides};
    use std::collections::BTreeSet;

    fn metrics() -> AssemblyMetrics {
        AssemblyMetrics {
            sequence_count: 10,
            total_length: 1000,
            min_length: 10,
            max_length: 500,
            mean_length: 100.0,
            median_length: 80.0,
            n50: 200,
            n90: 40,
            l50: 3,
            l90: 8,
            gc_percent: 51.0,
            at_percent: 44.0,
            n_percent: 5.0,
            ambiguity_percent: 5.0,
            duplicate_id_count: 1,
            duplicate_sequence_count: 2,
            invalid_sequence_count: 0,
            high_n_sequence_count: 3,
            tiny_contig_count: 4,
            max_gap_run: 120,
            sequences: Vec::new(),
        }
    }

    #[test]
    fn duplicate_ids_can_fail_when_configured() {
        let rules = RuleConfig {
            fail_on: BTreeSet::from(["duplicate_ids".to_string()]),
        };
        let profile = ProfileConfig::assembly(ThresholdOverrides { max_n_rate: None, min_contig_length: None });
        let analysis = analyze(&metrics(), &profile, &rules);
        assert_eq!(analysis.status, VerdictStatus::Fail);
        assert!(analysis.reasons.contains(&"duplicate_ids".to_string()));
    }

    #[test]
    fn high_n_defaults_to_warning() {
        let rules = RuleConfig { fail_on: BTreeSet::new() };
        let profile = ProfileConfig::assembly(ThresholdOverrides { max_n_rate: None, min_contig_length: None });
        let analysis = analyze(&metrics(), &profile, &rules);
        assert_eq!(analysis.status, VerdictStatus::Warn);
        assert!(analysis.reasons.contains(&"high_n_rate".to_string()));
    }
}
```

- [ ] **Step 2: Run findings tests to verify failure**

Run:

```bash
cargo test findings
```

Expected:

```text
cannot find function `analyze` in this scope
```

- [ ] **Step 3: Add findings implementation**

Append this implementation above the test module in `src/findings.rs`:

```rust
use crate::cli::RuleConfig;
use crate::metrics::AssemblyMetrics;
use crate::profile::ProfileConfig;
use crate::stats::composition::{fraction, round2};

pub fn analyze(metrics: &AssemblyMetrics, profile: &ProfileConfig, rules: &RuleConfig) -> Analysis {
    let mut findings = Vec::new();

    push_if(
        &mut findings,
        metrics.duplicate_id_count > 0,
        Finding {
            id: "duplicate_ids".to_string(),
            severity: Severity::Critical,
            profile: profile.name.clone(),
            affected_count: metrics.duplicate_id_count,
            affected_fraction: fraction(metrics.duplicate_id_count, metrics.sequence_count),
            message: format!("{} duplicate FASTA IDs were found.", metrics.duplicate_id_count),
            why_it_matters: "Duplicate IDs can break indexing, annotation, mapping, and workflow joins.".to_string(),
            suggested_next_step: "Rename or remove duplicate records before running downstream tools.".to_string(),
        },
    );

    push_if(
        &mut findings,
        metrics.invalid_sequence_count > 0,
        Finding {
            id: "invalid_chars".to_string(),
            severity: Severity::Critical,
            profile: profile.name.clone(),
            affected_count: metrics.invalid_sequence_count,
            affected_fraction: fraction(metrics.invalid_sequence_count, metrics.sequence_count),
            message: format!("{} sequences contain invalid nucleotide symbols.", metrics.invalid_sequence_count),
            why_it_matters: "Invalid symbols can make parsers, aligners, and annotation tools fail or silently misinterpret records.".to_string(),
            suggested_next_step: "Inspect the affected records and replace or remove invalid characters.".to_string(),
        },
    );

    push_if(
        &mut findings,
        metrics.n_percent / 100.0 >= profile.high_global_n_fraction || metrics.high_n_sequence_count > 0,
        Finding {
            id: "high_n_rate".to_string(),
            severity: Severity::Major,
            profile: profile.name.clone(),
            affected_count: metrics.high_n_sequence_count,
            affected_fraction: fraction(metrics.high_n_sequence_count, metrics.sequence_count),
            message: format!("{} sequences contain at least {:.0}% Ns.", metrics.high_n_sequence_count, profile.high_n_sequence_fraction * 100.0),
            why_it_matters: "High ambiguity can reduce annotation, mapping, and assembly interpretation quality.".to_string(),
            suggested_next_step: "Inspect high-N scaffolds or run gap closing, polishing, or filtering.".to_string(),
        },
    );

    push_if(
        &mut findings,
        metrics.tiny_contig_count > 0,
        Finding {
            id: "tiny_contigs".to_string(),
            severity: Severity::Minor,
            profile: profile.name.clone(),
            affected_count: metrics.tiny_contig_count,
            affected_fraction: fraction(metrics.tiny_contig_count, metrics.sequence_count),
            message: format!("{} sequences are shorter than {} bp.", metrics.tiny_contig_count, profile.min_contig_length),
            why_it_matters: "Many tiny contigs can indicate fragmentation and may add noise to annotation or submission.".to_string(),
            suggested_next_step: "Review length filters and assembly cleanup settings before downstream analysis.".to_string(),
        },
    );

    push_if(
        &mut findings,
        metrics.max_gap_run > profile.max_gap_run,
        Finding {
            id: "gap_runs".to_string(),
            severity: Severity::Major,
            profile: profile.name.clone(),
            affected_count: 1,
            affected_fraction: round2(metrics.max_gap_run as f64),
            message: format!("The longest N gap run is {} bp.", metrics.max_gap_run),
            why_it_matters: "Long gap runs can mark unresolved scaffold joins and reduce interpretability.".to_string(),
            suggested_next_step: "Inspect scaffolds with long N-runs and consider gap closing or splitting.".to_string(),
        },
    );

    push_if(
        &mut findings,
        metrics.duplicate_sequence_count > 0,
        Finding {
            id: "duplicate_sequences".to_string(),
            severity: Severity::Minor,
            profile: profile.name.clone(),
            affected_count: metrics.duplicate_sequence_count,
            affected_fraction: fraction(metrics.duplicate_sequence_count, metrics.sequence_count),
            message: format!("{} duplicate sequence bodies were found.", metrics.duplicate_sequence_count),
            why_it_matters: "Duplicate sequence bodies can indicate redundant contigs or repeated export artifacts.".to_string(),
            suggested_next_step: "Inspect duplicate sequences and confirm whether they are expected.".to_string(),
        },
    );

    let mut reasons = Vec::new();
    let mut status = VerdictStatus::Pass;

    for finding in &findings {
        if rules.fail_on.contains(&finding.id) || matches!(finding.severity, Severity::Critical) {
            status = VerdictStatus::Fail;
            reasons.push(finding.id.clone());
        } else if status != VerdictStatus::Fail {
            status = VerdictStatus::Warn;
            reasons.push(finding.id.clone());
        }
    }

    Analysis {
        status,
        reasons,
        findings,
    }
}

fn push_if(findings: &mut Vec<Finding>, condition: bool, finding: Finding) {
    if condition {
        findings.push(finding);
    }
}
```

- [ ] **Step 4: Run findings tests**

Run:

```bash
cargo test findings
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit findings and verdicts**

Run:

```bash
git add src/lib.rs src/findings.rs
git commit -m "feat: generate explainable findings"
```

Expected:

```text
[main <hash>] feat: generate explainable findings
```

---

## Task 7: Implement Report Writers

**Files:**
- Create: `src/report/mod.rs`
- Create: `src/report/json.rs`
- Create: `src/report/tsv.rs`
- Create: `src/report/multiqc.rs`
- Create: `src/report/html.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Wire report module exports**

Append this line to `src/lib.rs`:

```rust
pub mod report;
```

Create `src/report/mod.rs` with:

```rust
pub mod html;
pub mod json;
pub mod multiqc;
pub mod tsv;

use anyhow::Result;

use crate::cli::OutputPaths;
use crate::models::FastaguardReport;

pub fn write_all(report: &FastaguardReport, outputs: &OutputPaths) -> Result<()> {
    json::write(report, &outputs.json)?;
    tsv::write(report, &outputs.tsv)?;
    multiqc::write(report, &outputs.multiqc)?;
    html::write(report, &outputs.html)?;
    Ok(())
}
```

- [ ] **Step 2: Add JSON writer**

Create `src/report/json.rs` with:

```rust
use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

use crate::models::FastaguardReport;

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let file = File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}
```

- [ ] **Step 3: Add TSV writer**

Create `src/report/tsv.rs` with:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::FastaguardReport;

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let summary = &report.summary;
    let mut rows = Vec::new();
    rows.push(("schema_version", report.schema_version.clone()));
    rows.push(("profile", report.input.profile.clone()));
    rows.push(("verdict", format!("{:?}", report.verdict.status).to_uppercase()));
    rows.push(("sequence_count", summary.sequence_count.to_string()));
    rows.push(("total_length", summary.total_length.to_string()));
    rows.push(("n50", summary.n50.to_string()));
    rows.push(("n90", summary.n90.to_string()));
    rows.push(("l50", summary.l50.to_string()));
    rows.push(("l90", summary.l90.to_string()));
    rows.push(("gc_percent", summary.gc_percent.to_string()));
    rows.push(("n_percent", summary.n_percent.to_string()));
    rows.push(("finding_count", report.findings.len().to_string()));

    let mut content = String::from("metric\tvalue\n");
    for (metric, value) in rows {
        content.push_str(metric);
        content.push('\t');
        content.push_str(&value);
        content.push('\n');
    }

    fs::write(path, content).with_context(|| format!("failed to write TSV report {}", path.display()))
}
```

- [ ] **Step 4: Add MultiQC writer**

Create `src/report/multiqc.rs` with:

```rust
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::File;
use std::path::Path;

use crate::models::FastaguardReport;

#[derive(Serialize)]
struct MultiqcReport<'a> {
    id: &'static str,
    section_name: &'static str,
    description: &'static str,
    report: &'a FastaguardReport,
}

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let multiqc = MultiqcReport {
        id: "fastaguard",
        section_name: "FastaGuard",
        description: "FASTA preflight QC summary",
        report,
    };
    let file = File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    serde_json::to_writer_pretty(file, &multiqc)
        .with_context(|| format!("failed to write MultiQC JSON {}", path.display()))
}
```

- [ ] **Step 5: Add HTML writer**

Create `src/report/html.rs` with:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::FastaguardReport;

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let findings = report
        .findings
        .iter()
        .map(|finding| {
            format!(
                "<section><h3>{}</h3><p><strong>Severity:</strong> {:?}</p><p>{}</p><p><strong>Why it matters:</strong> {}</p><p><strong>Suggested next step:</strong> {}</p></section>",
                escape(&finding.id),
                finding.severity,
                escape(&finding.message),
                escape(&finding.why_it_matters),
                escape(&finding.suggested_next_step)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let embedded_json = escape(&serde_json::to_string_pretty(report)?);
    let summary = &report.summary;
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>FastaGuard Report</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #1f2933; }}
    header {{ border-bottom: 1px solid #d8dee4; margin-bottom: 1.5rem; }}
    .verdict {{ font-size: 1.5rem; font-weight: 700; }}
    table {{ border-collapse: collapse; width: 100%; max-width: 780px; }}
    th, td {{ border: 1px solid #d8dee4; padding: 0.45rem 0.6rem; text-align: left; }}
    section {{ margin: 1.25rem 0; }}
    pre {{ background: #f6f8fa; padding: 1rem; overflow: auto; }}
  </style>
</head>
<body>
  <header>
    <h1>FastaGuard Report</h1>
    <p class="verdict">Verdict: {:?}</p>
    <p>Before QUAST. Before BUSCO. Before BlobToolKit. Run FastaGuard first.</p>
  </header>
  <main>
    <h2>Summary</h2>
    <table>
      <tr><th>Metric</th><th>Value</th></tr>
      <tr><td>Sequences</td><td>{}</td></tr>
      <tr><td>Total length</td><td>{}</td></tr>
      <tr><td>N50</td><td>{}</td></tr>
      <tr><td>N90</td><td>{}</td></tr>
      <tr><td>GC%</td><td>{}</td></tr>
      <tr><td>N%</td><td>{}</td></tr>
    </table>
    <h2>Findings</h2>
    {}
    <h2>Embedded JSON</h2>
    <pre>{}</pre>
  </main>
</body>
</html>
"#,
        report.verdict.status,
        summary.sequence_count,
        summary.total_length,
        summary.n50,
        summary.n90,
        summary.gc_percent,
        summary.n_percent,
        findings,
        embedded_json
    );

    fs::write(path, html).with_context(|| format!("failed to write HTML report {}", path.display()))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
```

- [ ] **Step 6: Run report-related tests through the full test suite**

Run:

```bash
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 7: Commit report writers**

Run:

```bash
git add src/lib.rs src/report
git commit -m "feat: write FastaGuard report artifacts"
```

Expected:

```text
[main <hash>] feat: write FastaGuard report artifacts
```

---

## Task 8: Wire End-To-End CLI Behavior

**Files:**
- Modify: `tests/cli.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/models.rs`

- [ ] **Step 1: Add CLI integration tests**

Replace `tests/cli.rs` with:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_preflight_positioning() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("FASTA preflight QC"));
}

#[test]
fn valid_assembly_writes_all_outputs_and_passes() {
    let dir = tempfile::tempdir().unwrap();
    let html = dir.path().join("report.html");
    let json = dir.path().join("report.json");
    let tsv = dir.path().join("report.tsv");
    let multiqc = dir.path().join("multiqc.json");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--out").arg(&html)
        .arg("--json").arg(&json)
        .arg("--tsv").arg(&tsv)
        .arg("--multiqc").arg(&multiqc)
        .assert()
        .success();

    assert!(html.exists());
    assert!(json.exists());
    assert!(tsv.exists());
    assert!(multiqc.exists());
    let json_text = std::fs::read_to_string(json).unwrap();
    assert!(json_text.contains("\"status\": \"PASS\""));
}

#[test]
fn problem_assembly_returns_failure_for_default_critical_findings() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/problem_assembly.fa")
        .arg("--out").arg(dir.path().join("report.html"))
        .arg("--json").arg(dir.path().join("report.json"))
        .arg("--tsv").arg(dir.path().join("report.tsv"))
        .arg("--multiqc").arg(dir.path().join("multiqc.json"))
        .assert()
        .code(2)
        .stderr(predicate::str::is_empty());
}

#[test]
fn unsupported_profile_is_tool_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--profile")
        .arg("protein")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unsupported profile"));
}
```

- [ ] **Step 2: Wire the library orchestration and process exit behavior**

Append this implementation to `src/models.rs`:

```rust
use std::path::Path;

use crate::cli::RunConfig;
use crate::findings::Analysis;
use crate::metrics::AssemblyMetrics;

impl FastaguardReport {
    pub fn from_analysis(config: RunConfig, metrics: AssemblyMetrics, analysis: Analysis) -> Self {
        let input_path = config.input.display().to_string();
        let compressed = path_is_gzip(&config.input);
        let profile = config.profile.clone();
        let html = config.outputs.html.display().to_string();
        let tsv = config.outputs.tsv.display().to_string();
        let multiqc = config.outputs.multiqc.display().to_string();

        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: ToolInfo {
                name: TOOL_NAME.to_string(),
                version: TOOL_VERSION.to_string(),
            },
            input: InputInfo {
                path: input_path,
                profile,
                compressed,
            },
            verdict: Verdict {
                status: analysis.status,
                reasons: analysis.reasons,
            },
            summary: Summary {
                sequence_count: metrics.sequence_count,
                total_length: metrics.total_length,
                min_length: metrics.min_length,
                max_length: metrics.max_length,
                mean_length: metrics.mean_length,
                median_length: metrics.median_length,
                n50: metrics.n50,
                n90: metrics.n90,
                l50: metrics.l50,
                l90: metrics.l90,
                gc_percent: metrics.gc_percent,
                at_percent: metrics.at_percent,
                n_percent: metrics.n_percent,
                ambiguity_percent: metrics.ambiguity_percent,
                duplicate_id_count: metrics.duplicate_id_count,
                duplicate_sequence_count: metrics.duplicate_sequence_count,
                invalid_sequence_count: metrics.invalid_sequence_count,
                high_n_sequence_count: metrics.high_n_sequence_count,
                tiny_contig_count: metrics.tiny_contig_count,
                max_gap_run: metrics.max_gap_run,
            },
            findings: analysis.findings,
            artifacts: Artifacts { html, tsv, multiqc },
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self.verdict.status {
            VerdictStatus::Pass => 0,
            VerdictStatus::Warn => 1,
            VerdictStatus::Fail => 2,
        }
    }
}

fn path_is_gzip(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gz"))
        .unwrap_or(false)
}
```

Replace `src/lib.rs` with:

```rust
pub mod cli;
pub mod findings;
pub mod metrics;
pub mod models;
pub mod parser;
pub mod profile;
pub mod report;
pub mod stats;

use anyhow::Result;
use cli::Cli;

pub fn run(cli: Cli) -> Result<i32> {
    let config = cli.to_run_config()?;
    let profile = profile::ProfileConfig::assembly(config.thresholds);
    let records = parser::read_fasta(&config.input)?;
    let metrics = metrics::AssemblyMetrics::from_records(records, &profile);
    let analysis = findings::analyze(&metrics, &profile, &config.rules);
    let output = models::FastaguardReport::from_analysis(config.clone(), metrics, analysis);
    report::write_all(&output, &config.outputs)?;
    Ok(output.exit_code())
}
```

Replace `src/main.rs` with:

```rust
use clap::Parser;
use fastaguard::cli::Cli;

fn main() {
    let cli = Cli::parse();
    match fastaguard::run(cli) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("fastaguard error: {error}");
            std::process::exit(3);
        }
    }
}
```

- [ ] **Step 3: Run CLI tests**

Run:

```bash
cargo test --test cli
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Run a manual valid report command**

Run:

```bash
cargo run -- testdata/valid_assembly.fa --out /tmp/fastaguard_valid.html --json /tmp/fastaguard_valid.json --tsv /tmp/fastaguard_valid.tsv --multiqc /tmp/fastaguard_valid_multiqc.json
```

Expected exit code:

```text
0
```

Expected files:

```text
/tmp/fastaguard_valid.html
/tmp/fastaguard_valid.json
/tmp/fastaguard_valid.tsv
/tmp/fastaguard_valid_multiqc.json
```

- [ ] **Step 5: Run a manual problem report command**

Run:

```bash
cargo run -- testdata/problem_assembly.fa --out /tmp/fastaguard_problem.html --json /tmp/fastaguard_problem.json --tsv /tmp/fastaguard_problem.tsv --multiqc /tmp/fastaguard_problem_multiqc.json
```

Expected exit code:

```text
2
```

Expected JSON content:

```text
"duplicate_ids"
"invalid_chars"
"high_n_rate"
```

- [ ] **Step 6: Commit CLI integration**

Run:

```bash
git add tests/cli.rs src/lib.rs src/main.rs src/models.rs
git commit -m "feat: wire end-to-end CLI checks"
```

Expected:

```text
[main <hash>] feat: wire end-to-end CLI checks
```

---

## Task 9: Update Documentation And Pipeline Examples

**Files:**
- Modify: `README.md`
- Modify: `docs/mvp-spec.md`
- Modify: `docs/output-contract.md`
- Create: `examples/nextflow/main.nf`
- Create: `examples/snakemake/Snakefile`

- [ ] **Step 1: Add quickstart to `README.md`**

Append this section to `README.md`:

```markdown
## Quickstart

Build locally:

```bash
cargo build --release
```

Run the assembly preflight check:

```bash
./target/release/fastaguard sample.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_multiqc.json
```

Pipeline gate example:

```bash
./target/release/fastaguard sample.fa --fail-on duplicate_ids,invalid_chars,high_n_rate
```

Exit codes:

```text
0 = pass
1 = warnings above configured threshold
2 = hard QC failure
3 = invalid input / tool error
```
```

- [ ] **Step 2: Add Nextflow example**

Create `examples/nextflow/main.nf` with:

```groovy
nextflow.enable.dsl = 2

params.fasta = "sample.fa"

process FASTAGUARD {
    input:
    path fasta

    output:
    path "fastaguard_report.html"
    path "fastaguard.json"
    path "fastaguard.tsv"
    path "fastaguard_multiqc.json"

    script:
    """
    fastaguard ${fasta} \
      --profile assembly \
      --out fastaguard_report.html \
      --json fastaguard.json \
      --tsv fastaguard.tsv \
      --multiqc fastaguard_multiqc.json
    """
}

workflow {
    FASTAGUARD(file(params.fasta))
}
```

- [ ] **Step 3: Add Snakemake example**

Create `examples/snakemake/Snakefile` with:

```python
rule fastaguard:
    input:
        fasta="sample.fa"
    output:
        html="fastaguard_report.html",
        json="fastaguard.json",
        tsv="fastaguard.tsv",
        multiqc="fastaguard_multiqc.json"
    shell:
        """
        fastaguard {input.fasta} \
          --profile assembly \
          --out {output.html} \
          --json {output.json} \
          --tsv {output.tsv} \
          --multiqc {output.multiqc}
        """
```

- [ ] **Step 4: Add implementation status to `docs/mvp-spec.md`**

Append:

```markdown
## Implementation Status

The v0.1 assembly MVP is implemented as a Rust CLI with:

- streaming FASTA parsing for plain and gzipped files
- assembly metrics
- explainable findings
- deterministic verdict exit codes
- JSON, TSV, HTML, and MultiQC-compatible outputs
```

- [ ] **Step 5: Run docs search checks**

Run:

```bash
rg -n "<avoid-positioning-phrase>|<fix-marker>" README.md docs examples
```

Expected:

```text
docs/product-thesis.md:<line>:<avoid-positioning-phrase>.
```

The single expected match is in the explicit "Avoid" positioning section.

- [ ] **Step 6: Commit docs and examples**

Run:

```bash
git add README.md docs/mvp-spec.md docs/output-contract.md examples
git commit -m "docs: add FastaGuard usage and pipeline examples"
```

Expected:

```text
[main <hash>] docs: add FastaGuard usage and pipeline examples
```

---

## Task 10: Final Verification

**Files:**
- Modify only files required by formatter output.

- [ ] **Step 1: Format code**

Run:

```bash
cargo fmt
```

Expected:

```text
no terminal output
```

- [ ] **Step 2: Run all tests**

Run:

```bash
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run clippy**

Run:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected:

```text
Finished
```

- [ ] **Step 4: Verify report artifacts with the valid fixture**

Run:

```bash
cargo run -- testdata/valid_assembly.fa --out /tmp/fastaguard_report.html --json /tmp/fastaguard.json --tsv /tmp/fastaguard.tsv --multiqc /tmp/fastaguard_multiqc.json
```

Expected exit code:

```text
0
```

Run:

```bash
test -s /tmp/fastaguard_report.html && test -s /tmp/fastaguard.json && test -s /tmp/fastaguard.tsv && test -s /tmp/fastaguard_multiqc.json
```

Expected exit code:

```text
0
```

- [ ] **Step 5: Verify report artifacts with the problem fixture**

Run:

```bash
cargo run -- testdata/problem_assembly.fa --out /tmp/fastaguard_problem.html --json /tmp/fastaguard_problem.json --tsv /tmp/fastaguard_problem.tsv --multiqc /tmp/fastaguard_problem_multiqc.json
```

Expected exit code:

```text
2
```

Run:

```bash
rg -n "duplicate_ids|invalid_chars|high_n_rate" /tmp/fastaguard_problem.json
```

Expected:

```text
matches for duplicate_ids, invalid_chars, and high_n_rate
```

- [ ] **Step 6: Commit formatter or final verification changes**

If `git status --short` shows changes after formatting, run:

```bash
git add .
git commit -m "chore: finalize assembly MVP verification"
```

Expected when formatter changed files:

```text
[main <hash>] chore: finalize assembly MVP verification
```

Expected when no files changed:

```text
nothing to commit, working tree clean
```

---

## Self-Review Notes

Spec coverage:

- Product positioning is covered by `README.md`, CLI help text, and report copy.
- Assembly-only scope is covered by `src/cli.rs` profile rejection and `src/profile.rs`.
- FASTA validity is covered by `src/parser.rs` and critical findings for invalid characters.
- Structural stats are covered by `src/metrics.rs` and `src/stats/nxx.rs`.
- Composition stats are covered by `src/metrics.rs` and `src/stats/composition.rs`.
- Assembly QC findings are covered by `src/findings.rs`.
- Output contract is covered by `src/models.rs` and `src/report/*`.
- Exit codes are covered by `src/models.rs`, `src/main.rs`, and CLI integration tests.
- Pipeline adoption is covered by `examples/nextflow/main.nf` and `examples/snakemake/Snakefile`.

Known implementation choice:

- The v0.1 parser is a small in-repo streaming parser over `BufRead` and `flate2`. This keeps line-level diagnostics explicit. The parser is isolated in `src/parser.rs`, so it can be replaced by `needletail` or `noodles-fasta` later without changing the CLI, metrics, findings, or report contract.
