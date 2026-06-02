# Value Benchmark

Measured locally with fastaguard 0.3.0, commit 1873216, macOS ARM64:

| Input | Result | Time | Memory |
| --- | --- | ---: | ---: |
| 10 Mbp synthetic FASTA, 10k records | PASS | 0.51 seconds | about 17 MB RSS |
| 100 Mbp synthetic FASTA, 100k records | WARN for GC outliers | 0.98 seconds | about 50 MB RSS |

Frame timings as evidence, not formal universal benchmark claims. These numbers
show the order of magnitude for local preflight overhead on deterministic
synthetic FASTA files.

This is a v0.3 single-file baseline. It is not a v0.4 compare-mode timing and
should not be read as a cohort benchmark.

The practical value is pipeline triage: a sub-second to seconds-level FASTA
preflight can prevent minutes, CPU-hours, or days of downstream work on inputs
that are malformed, ambiguous, hard to index, or not ready for submission.

Record new value benchmarks with:

- FastaGuard version
- commit
- platform
- input size and sequence count
- verdict and top findings
- elapsed time
- peak RSS when measured externally
- whether heavier downstream QC was avoided or routed differently
