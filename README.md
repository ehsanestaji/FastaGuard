# FastaGuard

**FASTA preflight QC for modern bioinformatics pipelines.**

FastaGuard checks assembly FASTA files before QUAST, BUSCO, BlobToolKit,
CheckM, annotation, or other expensive downstream steps. It validates structure,
flags obvious FASTA-level problems, and writes stable reports for humans,
workflow engines, and future tool agents.

Use it to validate first, fix early, and route smarter.

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

## Why FastaGuard?

Most bioinformatics QC tools answer downstream questions: assembly quality,
biological completeness, contamination evidence, taxonomy, annotation
readiness, or report aggregation. FastaGuard runs earlier. It answers whether
the FASTA itself is valid, sane, interpretable, and safe to pass downstream.

Use FastaGuard when you need:

- FASTA preflight before expensive QC, annotation, or submission workflows
- a deterministic PASS/WARN/FAIL gate for Nextflow, Snakemake, nf-core, Galaxy,
  or institutional pipelines
- batch triage across many FASTA files with `fastaguard compare`
- submission-readiness signals before official validators
- stable JSON, TSV, HTML, and MultiQC-compatible outputs for humans, workflows,
  and tool agents

If FastaGuard fails, fix the FASTA first. If it passes, route to the right
downstream tool.

## Release Status

| Channel | Status |
| --- | --- |
| Source/package metadata | this branch/package metadata targets `v0.5.0`; `v0.4.0` is the latest tagged source release |
| GitHub release | v0.4 GitHub release binaries are built from the `v0.4.0` tag |
| Bioconda | `v0.3.0` is live for Linux and macOS x86_64/ARM64; v0.5 is not yet published there |
| BioContainers | `v0.3.0` is live as a pinned workflow image; v0.5 is not yet published there |
| Source build | local checkout builds report the package version from `Cargo.toml` |

## Install

Published bioinformatics install:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.3.0
```

Published containerized workflow install:

```bash
docker pull quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0
```

Run through BioContainers:

```bash
docker run --rm quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0 fastaguard --version
```

GitHub release binary for Linux x86_64:

```bash
curl -L -O https://github.com/ehsanestaji/FastaGuard/releases/download/v0.4.0/fastaguard-v0.4.0-x86_64-unknown-linux-gnu.tar.gz
tar -xzf fastaguard-v0.4.0-x86_64-unknown-linux-gnu.tar.gz
./fastaguard-v0.4.0-x86_64-unknown-linux-gnu/fastaguard --version
```

GitHub release binary for macOS Apple Silicon:

```bash
curl -L -O https://github.com/ehsanestaji/FastaGuard/releases/download/v0.4.0/fastaguard-v0.4.0-aarch64-apple-darwin.tar.gz
tar -xzf fastaguard-v0.4.0-aarch64-apple-darwin.tar.gz
./fastaguard-v0.4.0-aarch64-apple-darwin/fastaguard --version
```

Build from the latest published Git tag:

```bash
cargo install --git https://github.com/ehsanestaji/FastaGuard --tag v0.4.0
fastaguard --version
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

Local release-prep install from this checkout:

```bash
cargo install --path . --locked
fastaguard --version
```

## Quickstart

The `--gate pipeline` examples below require FastaGuard `v0.3.0` or newer.
The `fastaguard compare` example requires FastaGuard `v0.4.0` or newer.
The `--gate submission` example requires the v0.5 source/package contract.

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

v0.4 compare starter example:

```bash
fastaguard compare assemblies/*.fa --profile assembly --gate pipeline
```

This command is part of the v0.4 GitHub release. Bioconda and BioContainers may
still be `v0.3.0` until packaging publication follow-up is complete.

Submission-readiness preflight:

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate submission \
  --submission-target ncbi \
  --json fastaguard.json \
  --out fastaguard_report.html
```

FastaGuard reports FASTA-level risks before official validators. It does not
guarantee NCBI, ENA, or DDBJ acceptance and does not replace NCBI FCS,
annotation validation, QUAST, BUSCO, BlobToolKit, or CheckM.

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

Published BioContainers provides the v0.3 image for workflow engines:

```bash
docker pull quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0
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

v0.4 adds preflight readiness and compare mode:

- readiness categories for file, structure, alphabet, index, assembly,
  submission, and machine readiness
- `fastaguard compare` for starter cohort triage across many FASTA files
- cohort JSON, TSV, HTML, and MultiQC-compatible outputs for workflow routing
- boundaries that keep FastaGuard upstream of QUAST, BUSCO, BlobToolKit,
  CheckM, official validators, and annotation workflows

v0.5 adds the submission-readiness gate:

- `--gate submission` for stricter FASTA-level submission preflight
- `--submission-target generic|ncbi` for target-aware identifier and header
  advisories
- submission-readiness fields in JSON, TSV, HTML, MultiQC, and compare outputs
- boundaries that keep FastaGuard upstream of official validators, NCBI FCS,
  annotation validation, QUAST, BUSCO, BlobToolKit, and CheckM

## Positioning

FastaGuard should recommend deeper tools when they are appropriate:

- FastQC for raw-read QC
- QUAST for assembly quality evaluation
- BUSCO for biological completeness
- BlobToolKit for contamination and cobiont exploration
- CheckM for microbial genome completeness and contamination
- seqkit for ad hoc sequence operations
- MultiQC for aggregating reports

The strategic wedge is earlier:

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```

## Documentation

- [Example reports](examples/reports/README.md)
- [Use cases and positioning](docs/use-cases.md)
- [Product thesis](docs/product-thesis.md)
- [Vision plan](docs/vision-plan.md)
- [MVP spec](docs/mvp-spec.md)
- [Preflight readiness](docs/preflight-readiness.md)
- [Compare mode](docs/compare-mode.md)
- [Value benchmark](docs/value-benchmark.md)
- [Output contract](docs/output-contract.md)
- [Tool landscape](docs/tool-landscape.md)
- [Adoption plan](docs/adoption-plan.md)
- [LLM and tooling vision](docs/llm-tooling-vision.md)
- [Benchmarking](docs/benchmarking.md)
- [v0.2 evidence pack](docs/evidence/fastaguard-v0.2-evidence.md)
- [v0.3 evidence workflow](docs/evidence/fastaguard-v0.3-evidence.md)
- [v0.5 submission readiness evidence](docs/evidence/fastaguard-v0.5-submission-readiness.md)
- [Packaging](docs/packaging.md)
- [v0.5.0 release notes](docs/releases/v0.5.0.md)
- [v0.4.0 release notes](docs/releases/v0.4.0.md)
- [v0.3.0 release notes](docs/releases/v0.3.0.md)
- [v0.2.0 release notes](docs/releases/v0.2.0.md)
- [v0.1.1 release notes](docs/releases/v0.1.1.md)
- [v0.1.0 release notes](docs/releases/v0.1.0.md)
- [Roadmap](docs/roadmap.md)
- [First-release design](docs/superpowers/specs/2026-05-18-fastaguard-first-release-design.md)

## Status

This branch/package metadata targets FastaGuard v0.5.0. The latest tagged
GitHub release remains v0.4.0, which adds preflight readiness, compare mode,
and cohort-level FASTA triage outputs.

v0.3.0 remains the current Bioconda and BioContainers release until packaging
follow-up is complete.

Bioconda serves v0.3.0 for `linux-64`, `linux-aarch64`, `osx-64`, and
`osx-arm64`. BioContainers publishes the pinned v0.3 workflow image
`quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0`.
