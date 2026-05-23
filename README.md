# FastaGuard

FastaGuard is a fast, explainable FASTA QC tool for validating assembly FASTA files before expensive downstream analysis.

It is not intended to compete with QUAST, BUSCO, BlobToolKit, FastQC, or MultiQC. FastaGuard is the earlier preflight and triage layer: the first command that answers whether a FASTA file is valid, sane, interpretable, and ready for downstream tools.

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

## Install

Recommended bioinformatics install:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

Verify the installed CLI:

```bash
fastaguard --version
fastaguard --schema
```

GitHub release binaries are also available for Linux and macOS:

```bash
tar -xzf fastaguard-v0.1.1-x86_64-unknown-linux-gnu.tar.gz
./fastaguard-v0.1.1-x86_64-unknown-linux-gnu/fastaguard --help
```

Local development build:

```bash
cargo build --release --locked
```

## Quickstart

Run the assembly preflight check:

```bash
fastaguard sample.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_mqc.json
```

Pipeline gate example:

```bash
fastaguard sample.fa --fail-on duplicate_ids,invalid_chars,high_n_rate
```

Inspect the machine-readable contract:

```bash
fastaguard --schema
fastaguard --finding-catalog
fastaguard --explain-finding high_n_rate
```

Build and run the Docker image:

```bash
docker build -t fastaguard:local .
docker run --rm -v "$PWD:/data" fastaguard:local /data/sample.fa \
  --profile assembly \
  --out /data/fastaguard_report.html \
  --json /data/fastaguard.json \
  --tsv /data/fastaguard.tsv \
  --multiqc /data/fastaguard_mqc.json
```

Exit codes:

```text
0 = pass
1 = warnings above configured threshold
2 = hard QC failure
3 = invalid input / tool error
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
  --multiqc fastaguard_mqc.json
```

The MVP focuses on:

- FASTA validity
- invalid FASTA structure reports with explainable FAIL verdicts
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
- explainable PASS / WARN / FAIL verdicts
- machine-readable summaries, actions, scope, and provenance
- stable JSON, TSV, HTML, and MultiQC-compatible outputs
- length histogram and GC-vs-length plot data in JSON and HTML

Planned after v0.1:

- composition outliers

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

- [Example reports](examples/reports/README.md)
- [Product thesis](docs/product-thesis.md)
- [MVP spec](docs/mvp-spec.md)
- [Output contract](docs/output-contract.md)
- [Tool landscape](docs/tool-landscape.md)
- [Adoption plan](docs/adoption-plan.md)
- [LLM and tooling vision](docs/llm-tooling-vision.md)
- [Benchmarking](docs/benchmarking.md)
- [Packaging](docs/packaging.md)
- [v0.1.1 release notes](docs/releases/v0.1.1.md)
- [v0.1.0 release notes](docs/releases/v0.1.0.md)
- [Roadmap](docs/roadmap.md)
- [First-release design](docs/superpowers/specs/2026-05-18-fastaguard-first-release-design.md)

## Status

v0.1 assembly MVP implemented as a Rust CLI. FastaGuard v0.1.1 is published
on Bioconda for `linux-64`, `linux-aarch64`, `osx-64`, and `osx-arm64`.
BioContainers image availability is still pending confirmation.
