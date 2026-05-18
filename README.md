# FastaGuard

FastaGuard is a fast, explainable FASTA QC tool for validating assembly FASTA files before expensive downstream analysis.

It is not intended to compete with QUAST, BUSCO, BlobToolKit, FastQC, or MultiQC. FastaGuard is the earlier preflight and triage layer: the first command that answers whether a FASTA file is valid, sane, interpretable, and ready for downstream tools.

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

## Product Thesis

FASTA files are everywhere, but FASTA QC is fragmented across ad hoc scripts, `seqkit stats`, assembly QC tools, completeness tools, contamination workflows, and pipeline-specific checks. Each is useful, but none is the simple default first command for:

```text
Is this FASTA file valid, sane, interpretable, and ready for downstream tools?
```

FastaGuard fills that gap:

```text
FastaGuard is a fast, explainable FASTA QC tool that validates assembly FASTA files, detects structural and composition red flags, and produces pipeline-ready reports before expensive downstream analysis.
```

## v0.1 Scope

The first release is assembly-first.

```bash
fastaguard sample.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_multiqc.json
```

The MVP focuses on:

- FASTA validity
- duplicate IDs
- duplicate sequences
- invalid nucleotide/IUPAC characters
- empty records
- core assembly stats
- N50, N90, L50, L90
- GC, AT, N, and ambiguity rates
- high-N scaffolds
- gap runs
- suspicious tiny contigs
- length and composition outliers
- explainable PASS / WARN / FAIL verdicts
- stable JSON, TSV, HTML, and MultiQC-compatible outputs

## Positioning

FastaGuard should recommend deeper tools when they are appropriate:

- QUAST for assembly quality evaluation
- BUSCO for biological completeness
- BlobToolKit for contamination and cobiont exploration
- CheckM for microbial genome completeness and contamination
- seqkit for ad hoc sequence operations

The strategic wedge is earlier:

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```

## Documentation

- [Product thesis](docs/product-thesis.md)
- [MVP spec](docs/mvp-spec.md)
- [Output contract](docs/output-contract.md)
- [Roadmap](docs/roadmap.md)
- [First-release design](docs/superpowers/specs/2026-05-18-fastaguard-first-release-design.md)

## Status

Private planning repository. Implementation has not started yet.
