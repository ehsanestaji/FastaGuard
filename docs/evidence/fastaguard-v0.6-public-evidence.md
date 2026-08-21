# FastaGuard v0.6 Public Evidence

This evidence pack records one reproducible assembly-profile run with
FastaGuard 0.6.0. It combines three deterministic local cases with two exact
NCBI assembly accessions:

- `GCF_000005845.2`, Escherichia coli K-12 MG1655
- `GCF_000182925.2`, Neurospora crassa OR74A

The portable results are available as
[JSON](results/v0.6/evidence_summary.json) and
[TSV](results/v0.6/evidence_summary.tsv). They contain input sizes and SHA-256
checksums, release provenance, structural metrics, verdict and gate status,
and all finding IDs. Downloaded FASTA files and complete reports are not part
of the repository.

## Run Method

The release binary was built with the locked dependency graph. The tracked
Rust and Cargo source was verified byte-for-byte against release commit
`cf27295da0cb9b1a48318caa9e3b8739cfd0c104` before collection. The collector
then downloaded only the two manifest accessions with the NCBI Datasets CLI
and ran each case with the assembly profile, pipeline gate, and a one-base
minimum contig length:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.6-public \
  --portable-summary-dir docs/evidence/results/v0.6
```

## Results

| Case | Scale | Verdict / gate | Structural result | Finding IDs |
| --- | ---: | --- | --- | --- |
| Deterministic synthetic FASTA | 604 bp, 4 records | PASS / PASS | N50 160; N90 140 | none |
| Problem assembly fixture | 145 bp, 5 records | FAIL / FAIL | N50 110; N90 8 | `duplicate_ids`, `duplicate_first_token_ids`, `invalid_chars`, `high_n_rate`, `gap_runs`, `terminal_ns`, `length_outliers`, `composite_anomalies` |
| Gzipped valid fixture | 47 bp, 3 records | WARN / WARN | N50 16; N90 15 | `terminal_ns` |
| `GCF_000005845.2` | 4,641,652 bp, 1 record | PASS / PASS | N50 and N90 4,641,652 | none |
| `GCF_000182925.2` | 41,102,378 bp, 21 records | WARN / WARN | N50 6,000,761; N90 4,218,384 | `gap_runs`, `gap_pattern_warnings` |

The E. coli reference passed without findings. The Neurospora reference had
gap-pattern advisories but no pipeline blockers, so its WARN result routes to
deeper assembly or biological QC instead of stopping report generation. The
problem fixture demonstrates that blocking FASTA defects remain machine
visible even though v0.6 returns success after writing a valid FAIL report.

## Environment and Runtime Context

This run used macOS 26.5.1 on arm64 with Python 3.14.5. Measured FastaGuard
elapsed times ranged from 0.0145 seconds for a tiny fixture to 0.3441 seconds
for the 41.1 Mbp fungal assembly. These timings are contextual measurements
from one machine, not cross-platform benchmarks or performance guarantees.

## Scope Limits

This is FASTA-level preflight evidence for two public reference assemblies,
not a representative biological benchmark. It does not establish biological
completeness, contamination status, taxonomic correctness, annotation quality,
assembly correctness, or repository acceptance. A PASS or non-blocking WARN
means only that the input satisfied the selected FastaGuard gate policy and can
be routed to appropriate downstream tools such as QUAST, BUSCO, BlobToolKit,
CheckM, official validators, or annotation workflows.
