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

## v0.2 Evidence Targets

FastaGuard should prove four preflight categories with small reproducible
fixtures:

| Evidence case | What FastaGuard catches | Why it should run before heavier tools |
| --- | --- | --- |
| duplicate IDs | repeated FASTA identifiers | helps prevent workflow joins, indexes, and annotations from becoming ambiguous |
| invalid characters | non-IUPAC sequence symbols | flags inputs that may trigger downstream parser and aligner failures |
| high-N | ambiguous scaffolds and gap-heavy records | flags low-confidence mapping and annotation inputs before they are treated as clean |
| GC outliers | composition-anomalous records | supports routing suspicious records to BlobToolKit, sourmash, Kraken, or other follow-up tools |

FastaGuard should not replace QUAST, BUSCO, or BlobToolKit. It should make their
inputs safer and make obvious FASTA-level problems visible before those tools run.

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

This evidence matters more than synthetic speed alone because it shows the wedge: cheap FASTA preflight before expensive downstream QC.
