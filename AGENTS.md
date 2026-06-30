# FastaGuard Project Memory

This file is durable project memory for Codex and other agent sessions working in this repository.

## Product Thesis

FastaGuard is the missing FASTA preflight and triage layer for modern bioinformatics pipelines.

It should not compete with FastQC, QUAST, BUSCO, BlobToolKit, CheckM, seqkit, or MultiQC. Those tools remain important. FastaGuard runs earlier:

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

The core promise:

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```

## Tool Landscape

- FastQC is mainly raw-read QC for FASTQ/BAM/SAM data: base quality, per-sequence GC, N content, duplication, adapters, and overrepresented sequences.
- QUAST evaluates genome assemblies and assembly-level quality.
- BUSCO estimates biological completeness.
- BlobToolKit helps explore contamination, cobionts, coverage, GC, and taxonomy signals.
- MultiQC aggregates reports from other tools.
- seqkit is a fast FASTA/FASTQ toolkit with useful stats and manipulation commands.

FastaGuard's gap is not "replace all QC tools." The gap is:

```text
There is no modern, default, explainable, machine-readable FASTA preflight tool.
```

## Product Direction

FastaGuard should unify fragmented FASTA preflight checks:

- FASTA validity
- malformed or empty records
- duplicate IDs
- duplicate sequences
- invalid characters
- basic structural stats
- N50, N90, L50, L90
- GC, AT, N, and ambiguity rates
- gap runs
- tiny contigs
- composition red flags
- stable JSON, TSV, HTML, and MultiQC-compatible outputs

After FastaGuard:

- if the FASTA fails, fix the FASTA first
- if the FASTA passes, route to QUAST, BUSCO, BlobToolKit, CheckM, seqkit, or annotation depending on the biological question

## Machine-Actionable Vision

FastaGuard should prepare for a future where machines, LLMs, workflow engines, and tool agents talk to QC tools directly.

Principles:

- JSON is the source of truth; HTML is a human view.
- Machines should not scrape HTML or logs.
- Stable finding IDs matter as much as pretty reports.
- Every finding should expose verdict, severity, evidence, thresholds, and suggested next actions as structured fields.
- Reports should include provenance and scope limits so agents know what FastaGuard can and cannot conclude.
- Optional LLM summaries must be local-metrics-only and traceable back to structured fields.

Current foundation:

```text
stable JSON and finding IDs
JSON Schema and finding catalog
machine_summary, structured finding actions, provenance, and scope
per-record finding evidence with bounded affected-record lists
contract discovery commands: --schema, --finding-catalog, --explain-finding
golden JSON conformance fixtures for pass, fail, and invalid FASTA cases
```

Recommended next sequence:

```text
next: richer evidence tables for additional profiles and compare mode
later: MCP/tool-agent interface and optional local summaries
```

## Deep Release Vision

Durable vision document:

```text
docs/vision-plan.md
```

FastaGuard should become the FASTA preflight operating system for modern
bioinformatics pipelines: validate the FASTA, explain red flags, emit a stable
contract, and route to the right downstream tools.

The release strategy is evidence before expansion:

```text
v0.3: evidence pack + assembly gate + provenance checksums
v0.4: compare mode for many FASTA files
v0.5: submission readiness gate
v0.6: transcriptome profile
v0.7: protein profile
v0.8: reference-panel profile
later: MCP/tool-agent interface and optional local summaries
```

Default product boundaries:

- stay fast and database-free by default
- keep JSON as the source of truth
- keep HTML as a human view
- make findings machine-actionable with stable IDs, severity, evidence, thresholds, actions, and scope
- keep optional generated summaries local-metrics-only and traceable back to structured fields
- never claim to replace QUAST, BUSCO, BlobToolKit, CheckM, seqkit, MultiQC, or annotation workflows

Recommended next big release:

```text
v0.5 should make submission readiness concrete before adding broad new biological profiles.
```

The next planned feature direction is:

```text
Submission Readiness Gate: --gate submission with --submission-target generic|ncbi.
```

This should stay FASTA-level and database-free. It should check identifier
safety, duplicate first-token IDs, unsafe characters, long identifiers, gap-like
N runs, high ambiguity, and tiny-record advisories. It must not claim repository
acceptance, biological completeness, annotation correctness, or contamination
confirmation.

## Collaboration Preference

When moving the project forward, provide a clear recommendation first, then proceed when the user approves or explicitly asks to continue. The default recommendation should favor boring, stable contracts over flashy AI features.
