# Benchmarking

## Recommendation

Benchmark FastaGuard with deterministic synthetic FASTA files before adding heavier biology-specific modules. The product promise is "fast preflight"; every release should be able to prove that on a repeatable input.

## Smoke Benchmark

Build a debug binary and run a tiny benchmark:

```bash
cargo build
python3 scripts/benchmark_large_fasta.py \
  --records 10 \
  --length 100 \
  --binary target/debug/fastaguard \
  --out-dir target/bench-smoke
```

This should finish quickly and produce `fastaguard.json`, `fastaguard.tsv`, `fastaguard_report.html`, and `fastaguard_mqc.json` in `target/bench-smoke/`.

For the v0.3 assembly gate contract, add the pipeline gate preset:

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

For the v0.4 compare-mode starter pattern, benchmark the same local binaries
across a directory of FASTA files before sending the cohort to interpretive QC:

```bash
fastaguard compare assemblies/*.fa --profile assembly --gate pipeline
```

## Larger Local Benchmark

Build an optimized binary:

```bash
cargo build --release --locked
```

Run a 10 Mbp synthetic FASTA:

```bash
python3 scripts/benchmark_large_fasta.py \
  --records 10000 \
  --length 1000 \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/10mbp
```

Run a 100 Mbp synthetic FASTA:

```bash
python3 scripts/benchmark_large_fasta.py \
  --records 100000 \
  --length 1000 \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/100mbp
```

The script prints a JSON timing summary with:

- record count
- bases per record
- total bases
- FASTA bytes
- elapsed seconds
- bases per second
- FastaGuard verdict
- output artifact paths

The summary records the observed release/debug binary version and checksum plus
the local platform and Python runtime. Output references are normalized file
names rather than absolute paths. The benchmark command continues to pass
`--force` for its owned report files, so repeating the same command replaces
the prior generated benchmark bundle deterministically.

## Keeping Generated FASTA

The generated FASTA is removed after the run by default to avoid leaving large files in `target/`. Keep it for inspection with:

```bash
python3 scripts/benchmark_large_fasta.py \
  --records 10000 \
  --length 1000 \
  --keep-fasta
```

## Interpreting Results

The synthetic benchmark is not a biological benchmark. It measures parser, metric, and report overhead on deterministic valid FASTA content.

Use it to answer:

- did runtime regress between commits?
- did output generation become unexpectedly expensive?
- does the tool still behave well on large record counts?

Do not use it to claim performance on contaminated assemblies, highly ambiguous assemblies, or compressed FASTA until separate fixtures cover those cases.

## Local Value Evidence Baseline

The value benchmark is documented in
[`docs/value-benchmark.md`](value-benchmark.md) as a v0.3 single-file baseline.
It is local evidence for preflight overhead, not a v0.4 compare-mode timing. The
measured local frame is:

- `fastaguard 0.3.0`, commit `1873216`, macOS ARM64
- 10 Mbp synthetic FASTA, 10k records: PASS, 0.51 seconds, about 17 MB RSS
- 100 Mbp synthetic FASTA, 100k records: WARN for GC outliers, 0.98 seconds,
  about 50 MB RSS

Frame timings as evidence, not formal universal benchmark claims. Use them to
show that FastaGuard is cheap enough to run before QUAST, BUSCO, BlobToolKit,
CheckM, official validators, annotation, or other heavier follow-up tools.

## v0.7 Qualification And Performance Manifests

Clean policy qualification and performance evidence answer different questions.
The synthetic fixtures in `testdata/clean_corpus/clean_cases.json` prove that
known-good FASTA records have no NCBI submission blockers. They are not timing
baselines. Conversely, `docs/evidence/benchmark-inputs.json` identifies
checksum-pinned inputs for runtime observations; it does not claim that a public
assembly is submission-ready or biologically complete.

Build the release binary before a benchmark run:

```bash
cargo build --release --locked
```

Run only the deterministic 10,000-record local case, without network access:

```bash
python3 scripts/benchmark_manifest.py \
  --manifest docs/evidence/benchmark-inputs.json \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/v0.7-manifest \
  --local-synthetic-only
```

Public assembly downloads never happen implicitly. The operator must opt in,
and the runner verifies the compressed input SHA-256 before starting FastaGuard:

```bash
python3 scripts/benchmark_manifest.py \
  --manifest docs/evidence/benchmark-inputs.json \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/v0.7-public \
  --download
```

The bacterial, fungal, GRCh38, and T2T FASTA files remain under the ignored
benchmark output directory and must never be committed. Only the small synthetic
fixtures in `testdata/` belong in Git.

The runner writes `benchmark_summary.json` and `benchmark_summary.tsv` with the
runner worktree commit/state, observed binary checksum/version, platform/runtime
context, input checksum and scale, runtime, verdict, and core metrics.
`runner_worktree_commit` and `runner_worktree_dirty` describe the checkout that
invoked the benchmark; they do not attest which commit produced the observed
binary. The binary itself is identified by `fastaguard_version` and
`binary_sha256`.
Publishable summaries contain no absolute paths, commands, FASTA sequence data,
or time/memory
pass-fail thresholds.

To compare with a prior observation, pass its captured JSON summary:

```bash
python3 scripts/benchmark_manifest.py \
  --manifest docs/evidence/benchmark-inputs.json \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/v0.7-repeat \
  --local-synthetic-only \
  --baseline target/benchmarks/v0.7-manifest/benchmark_summary.json
```

Baseline comparison is accepted only when the recorded system, release,
architecture, and Python runtime match the current pinned runner. A matching case
ID must also retain the same accession, assembly version, source URL, category,
expected scale, and input SHA-256. The elapsed ratio is contextual evidence; it
is not a universal performance guarantee and does not produce a performance
pass/fail verdict.

Each result also includes `scale_comparison`, which records expected and observed
base/record counts plus their deltas. This makes a changed public payload or an
imprecise scale expectation visible without turning scale agreement into an
acceptance or performance verdict.

## Evidence Targets

FastaGuard should prove four preflight categories with small reproducible
fixtures. For v0.3, the evidence should also show whether each category blocks
the pipeline gate:

| Evidence case | Gate behavior | What FastaGuard catches | Why it should run before heavier tools |
| --- | --- | --- | --- |
| duplicate IDs | blocking | repeated FASTA identifiers | helps prevent workflow joins, indexes, and annotations from becoming ambiguous |
| invalid characters | blocking | non-IUPAC sequence symbols | flags inputs that may trigger downstream parser and aligner failures |
| high-N | blocking | ambiguous scaffolds and gap-heavy records | flags low-confidence mapping and annotation inputs before they are treated as clean |
| GC outliers | advisory by default | composition-anomalous records | supports routing suspicious records to BlobToolKit, sourmash, Kraken, or other follow-up tools |

FastaGuard should not replace QUAST, BUSCO, or BlobToolKit. It should make their
inputs safer and make obvious FASTA-level problems visible before those tools run.
For automated workflows, record `gate.blocking_findings` and
`provenance.input_sha256` alongside runtime and verdict so the gate decision can
be audited against exact input bytes.

## Evidence To Collect Next

Use release binaries and public assemblies to build a small evidence table for the README and release notes:

- bacterial assembly around 5 Mbp
- fungal or small eukaryotic assembly around 30-50 Mbp
- large fragmented assembly with many contigs
- gzipped FASTA input
- intentionally problematic FASTA fixture with duplicate IDs and high-N scaffolds

For each run, record:

- FastaGuard version
- platform
- input size and sequence count
- elapsed seconds
- peak memory if measured externally
- verdict and top findings
- whether downstream tools would have been blocked or recommended
- gate status and `gate.blocking_findings` when run with `--gate pipeline`
- `provenance.input_sha256`

This evidence matters more than synthetic speed alone because it shows the wedge: cheap FASTA preflight before expensive downstream QC.

The v0.5 public evidence workflow is documented in
`docs/evidence/fastaguard-v0.5-public-evidence.md`. It extends the compact
summary with manifest fields such as `evidence_role`, `expected_scale`, and
`downstream_route` so benchmark tables explain why each case was selected and
what FastaGuard should route toward after preflight.

## Evidence Pack Workflow

The original v0.2 evidence workflow is documented in
`docs/evidence/fastaguard-v0.2-evidence.md`. The v0.3 gate evidence workflow is
documented in `docs/evidence/fastaguard-v0.3-evidence.md`.

CI-safe local run:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/local-smoke \
  --local-only
```

Public NCBI run:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3
```

The public run uses NCBI Datasets commands such as
`datasets download genome accession <ACCESSION> --include genome --filename <zip>`.
It writes compact `evidence_summary.json` and `evidence_summary.tsv` files while
leaving downloaded FASTA files and full reports under `target/`.

## Parser robustness and fuzz checks

FastaGuard keeps parser robustness evidence separate from runtime dependencies.
The property suite is a normal development dependency; `cargo-fuzz` and
libFuzzer are external development tooling confined to `fuzz/`.

Run the deterministic property check from the repository root:

```bash
cargo test --locked --test parser_properties
```

The property test generates one to eight valid records using only A, C, G, T,
and N. For each case it parses seven wrap widths with both LF and CRLF endings,
then checks exact record count, total length, N50, N90, and duplicate counts. It
also checks that finding IDs retain the same order for every serialization.

Install `cargo-fuzz` as external development tooling, then run the bounded
fuzz checks:

```bash
cargo fuzz run parser_events -- -max_total_time=60
cargo fuzz run report_serialization -- -max_total_time=60
```

`parser_events` passes arbitrary in-memory bytes through the reader parser and
accepts either a completed event stream or a structured error.
`report_serialization` builds bounded synthetic records and writes JSON, TSV,
MultiQC JSON, and HTML only inside a per-input temporary directory. Generated
fuzz corpora and artifacts are disposable test outputs and are not committed.
