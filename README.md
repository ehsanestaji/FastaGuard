# FastaGuard

**FASTA preflight QC for modern bioinformatics pipelines.**

FastaGuard checks assembly FASTA files before QUAST, BUSCO, BlobToolKit,
CheckM, annotation, or other expensive downstream steps. It validates structure,
flags obvious FASTA-level problems, and writes stable reports for humans,
workflow engines, and future tool agents.

Run it first when you need to know:

- is this FASTA file structurally valid?
- are identifiers, records, and sequence characters sane?
- are duplicate IDs, high-N content, gap runs, tiny contigs, or GC/length
  anomalies worth attention?
- can a workflow make a PASS/WARN/FAIL decision from machine-readable output?

FastaGuard is not a replacement for QUAST, BUSCO, BlobToolKit, CheckM, FastQC,
seqkit, or MultiQC. It is the earlier preflight and triage layer.

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

## Current Release

| Channel | Status |
| --- | --- |
| GitHub release | `v0.3.0` is live with Linux and macOS binaries |
| Bioconda | `v0.2.0` is live; `v0.3.0` update is under Bioconda review |
| BioContainers | `v0.2.0` is live; `v0.3.0` follows the Bioconda update |
| Source build | `v0.3.0` can be built from the Git tag |

## Install

Latest release binary for Linux x86_64:

```bash
curl -L -O https://github.com/ehsanestaji/FastaGuard/releases/download/v0.3.0/fastaguard-v0.3.0-x86_64-unknown-linux-gnu.tar.gz
tar -xzf fastaguard-v0.3.0-x86_64-unknown-linux-gnu.tar.gz
./fastaguard-v0.3.0-x86_64-unknown-linux-gnu/fastaguard --version
```

Latest release binary for macOS Apple Silicon:

```bash
curl -L -O https://github.com/ehsanestaji/FastaGuard/releases/download/v0.3.0/fastaguard-v0.3.0-aarch64-apple-darwin.tar.gz
tar -xzf fastaguard-v0.3.0-aarch64-apple-darwin.tar.gz
./fastaguard-v0.3.0-aarch64-apple-darwin/fastaguard --version
```

Build from the released Git tag:

```bash
cargo install --git https://github.com/ehsanestaji/FastaGuard --tag v0.3.0
fastaguard --version
```

Bioconda install, currently serving the published `v0.2.0` package until the
`v0.3.0` recipe update merges:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

Verify any installed CLI:

```bash
fastaguard --version
fastaguard --schema
```

Local development build:

```bash
cargo build --release --locked
```

## Quickstart

The `--gate pipeline` examples below require FastaGuard `v0.3.0` or newer.

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
fastaguard sample.fa --profile assembly --gate pipeline
```

The `pipeline` gate is the v0.3 assembly preset for workflow stop/go decisions.
It fails on duplicate IDs, invalid characters, invalid FASTA structure, and
high-N content. GC and length outliers remain advisory by default because they
are routing signals, not proof of contamination or misassembly. To make an
advisory finding block a pipeline, add it explicitly with `--fail-on`.

Inspect the machine-readable contract:

```bash
fastaguard --schema
fastaguard --finding-catalog
fastaguard --explain-finding high_n_rate
```

Build and run the local Docker image:

```bash
docker build -t fastaguard:local .
docker run --rm -v "$PWD:/data" fastaguard:local /data/sample.fa \
  --profile assembly \
  --out /data/fastaguard_report.html \
  --json /data/fastaguard.json \
  --tsv /data/fastaguard.tsv \
  --multiqc /data/fastaguard_mqc.json
```

Published BioContainers currently provides the v0.2 image, which does not
include v0.3 gate behavior yet. Use it for v0.2 workflows until the Bioconda
v0.3 update propagates:

```bash
docker pull quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0
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

## Assembly Scope

FastaGuard is assembly-first.

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate pipeline \
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

v0.2 expands the assembly preflight layer with:

- composition outliers
- richer provenance, taxonomy context, and routing hints
- hardened MultiQC and pipeline adoption material

v0.3 adds the assembly gate contract:

- `--gate pipeline` for default workflow blocking behavior
- `gate.blocking_findings` for machine stop/go decisions
- checksum provenance with `provenance.input_sha256`
- explicit advisory findings for evidence that should route follow-up QC rather
  than stop a pipeline by default

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
- [Vision plan](docs/vision-plan.md)
- [MVP spec](docs/mvp-spec.md)
- [Output contract](docs/output-contract.md)
- [Tool landscape](docs/tool-landscape.md)
- [Adoption plan](docs/adoption-plan.md)
- [LLM and tooling vision](docs/llm-tooling-vision.md)
- [Benchmarking](docs/benchmarking.md)
- [v0.2 evidence pack](docs/evidence/fastaguard-v0.2-evidence.md)
- [v0.3 evidence workflow](docs/evidence/fastaguard-v0.3-evidence.md)
- [Packaging](docs/packaging.md)
- [v0.3.0 release notes](docs/releases/v0.3.0.md)
- [v0.2.0 release notes](docs/releases/v0.2.0.md)
- [v0.1.1 release notes](docs/releases/v0.1.1.md)
- [v0.1.0 release notes](docs/releases/v0.1.0.md)
- [Roadmap](docs/roadmap.md)
- [First-release design](docs/superpowers/specs/2026-05-18-fastaguard-first-release-design.md)

## Status

v0.3.0 is published on GitHub with Linux and macOS release binaries. It adds the
assembly gate contract, checksum provenance, and evidence workflow.

Bioconda currently serves v0.2.0 for `linux-64`, `linux-aarch64`, `osx-64`,
and `osx-arm64`; the v0.3.0 Bioconda update is open and passing CI. The
BioContainers v0.3 image will become available after the Bioconda package
propagates.
