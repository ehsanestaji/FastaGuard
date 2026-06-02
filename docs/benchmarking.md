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

## Local Value Evidence

The v0.4 value benchmark is documented in
[`docs/value-benchmark.md`](value-benchmark.md). The measured local frame is:

- `fastaguard 0.3.0`, commit `1873216`, macOS ARM64
- 10 Mbp synthetic FASTA, 10k records: PASS, 0.51 seconds, about 17 MB RSS
- 100 Mbp synthetic FASTA, 100k records: WARN for GC outliers, 0.98 seconds,
  about 50 MB RSS

Frame timings as evidence, not formal universal benchmark claims. Use them to
show that FastaGuard is cheap enough to run before QUAST, BUSCO, BlobToolKit,
CheckM, official validators, annotation, or other heavier follow-up tools.

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
