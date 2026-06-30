# FastaGuard Use Cases And Positioning

## GitHub Description

Recommended short repository description:

```text
Explainable FASTA preflight QC before QUAST, BUSCO, BlobToolKit, annotation, and submission.
```

Longer description for release notes, package pages, or project summaries:

```text
Explainable FASTA preflight QC for bioinformatics pipelines. Validate FASTA
files, catch structural issues early, and emit machine-readable JSON, TSV, HTML,
and MultiQC reports before QUAST, BUSCO, BlobToolKit, annotation, or submission.
```

## Core Message

```text
Run FastaGuard first.
```

FastaGuard is the FASTA preflight layer: validate first, fix early, and route
smarter. It checks whether FASTA input is structurally valid, identifier-safe,
compositionally sane, and ready to continue into heavier tools.

FastaGuard does not replace FastQC, QUAST, BUSCO, BlobToolKit, CheckM, seqkit,
MultiQC, or official submission validators. It runs before them so their inputs
are cleaner and their failures are easier to interpret.

## Community Role

FastaGuard helps the bioinformatics community stop wasting compute and human
review time on broken FASTA files. Its role is to provide one dependable,
explainable first command for FASTA sanity:

```text
Is this FASTA valid, sane, interpretable, and safe to pass downstream?
```

That makes FastaGuard useful to researchers, workflow developers, core
facilities, package maintainers, and future tool-using agents.

## Use Cases

### FASTA Preflight Before Expensive QC

Run FastaGuard before assembly QC, completeness checks, contamination
workflows, annotation, or submission. It catches malformed records, empty
records, invalid characters, duplicate IDs, duplicate first-token IDs, high-N
content, gap runs, tiny contigs, and unsafe headers before downstream tools
spend time on bad input.

### Pipeline Gate For Workflow Engines

FastaGuard gives Nextflow, Snakemake, nf-core, Galaxy, and institutional
pipelines a simple PASS/WARN/FAIL decision with deterministic exit codes and
structured evidence.

Pipelines can stop early on hard FASTA failures, continue with warnings, or
route samples to follow-up tools based on machine-readable findings.

### Batch Triage With Compare Mode

Use `fastaguard compare` to inspect many FASTA files at once. Compare mode helps
teams identify which assemblies are ready to continue, which need FASTA-level
fixes, and which should be prioritized for deeper QC.

### Submission Readiness Preflight

FastaGuard helps identify FASTA-level issues worth fixing before official
validators or archive submission workflows. It does not replace NCBI, ENA,
DDBJ, FCS, or annotation validation; it helps users arrive there with cleaner
input.

### Machine-Readable QC For Tool Agents

FastaGuard treats JSON as the source of truth. Reports include stable finding
IDs, severity, evidence, thresholds, actions, provenance, and scope limits so
workflow engines, LLMs, and tool agents can consume QC results without scraping
HTML or logs.

## How FastaGuard Fits With Existing Tools

| Tool | Use it for | What FastaGuard does first |
| --- | --- | --- |
| FastQC | Raw-read QC | Focuses on FASTA assemblies, references, and sequence records |
| seqkit | Ad hoc sequence statistics and manipulation | Turns common FASTA checks into one opinionated QC contract |
| QUAST | Assembly quality evaluation | Catches structural FASTA problems before assembly QC |
| BUSCO | Biological completeness | Checks parseability and composition before completeness analysis |
| BlobToolKit, FCS, Kraken, sourmash | Contamination and taxonomy investigation | Flags FASTA-level anomalies worth routing to deeper evidence |
| CheckM | Microbial genome completeness and contamination | Validates FASTA-level input before microbial interpretation |
| MultiQC | Report aggregation | Emits summary data that can be aggregated across samples |

## Message Discipline

Use:

```text
FastaGuard catches FASTA-level problems before expensive downstream QC.
```

Use:

```text
Validate the FASTA before the pipeline pays for it.
```

Avoid:

```text
FastaGuard replaces submission validators.
```

Avoid:

```text
FastaGuard proves biological completeness or contamination status.
```

The correct boundary is:

```text
FastaGuard finds FASTA-level risks early. Official validators and downstream
tools still decide biological, taxonomic, annotation, and submission outcomes.
```
