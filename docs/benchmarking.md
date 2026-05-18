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

This should finish quickly and produce `fastaguard.json`, `fastaguard.tsv`, `fastaguard_report.html`, and `fastaguard_multiqc.json` in `target/bench-smoke/`.

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
