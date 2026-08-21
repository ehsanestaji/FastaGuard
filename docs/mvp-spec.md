# MVP Spec

## Recommendation

v0.1 should be assembly-only, database-free, streaming-first, and report-first.

The first release should do one thing well:

```text
Catch FASTA-level assembly problems before expensive assembly QC.
```

## Command

```bash
fastaguard sample.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_mqc.json
```

The default command should be useful:

```bash
fastaguard sample.fa
```

Default inferred behavior:

```text
profile = assembly
HTML report = fastaguard_report.html
JSON report = fastaguard.json
TSV summary = fastaguard.tsv
MultiQC JSON = fastaguard_mqc.json
```

## Implemented In v0.1

### Inputs

- plain FASTA
- gzipped FASTA

### Profile

- assembly

### FASTA Validity

- malformed headers
- empty records
- sequence before first header
- empty input
- duplicate IDs
- duplicate sequences
- invalid nucleotide/IUPAC symbols
- bad line endings and hidden characters where detectable

### Structural Stats

- sequence count
- total length
- minimum length
- maximum length
- mean length
- median length
- N50
- N90
- L50
- L90

### Composition Stats

- GC percent
- AT percent
- N percent
- IUPAC ambiguity rate

### Assembly QC

- gap runs
- suspicious tiny contigs
- high-N scaffolds
- length histogram data
- GC-vs-length plot data

### Explainability

Every meaningful finding should include:

- what was found
- why it matters
- suggested next step
- supporting evidence

Example:

```text
Major finding: 12.8% of sequences contain more than 20% Ns.
Why it matters: high ambiguity can reduce annotation and mapping quality.
Suggested next step: inspect high-N scaffolds or run gap closing/polishing.
```

## Out of Scope

- BUSCO-style completeness
- QUAST-style reference or assembly correctness evaluation
- BlobToolKit-style taxonomy or contamination analysis
- external databases
- k-mer or minimizer sketches
- transcriptome-specific heuristics
- protein-specific checks
- cohort compare mode
- browser-based contig filtering
- AI-generated summaries

## Planned After v0.1

- ultra-short and ultra-long outliers
- per-sequence composition outliers
- scaffold fragmentation heuristics beyond the current tiny-contig and gap-run checks

## Verdicts

Verdict levels:

```text
PASS
WARN
FAIL
```

Default FAIL conditions:

- invalid FASTA structure
- empty input
- duplicate IDs
- invalid nucleotide symbols

Default WARN conditions:

- high N content
- many high-N scaffolds
- excessive tiny contigs
- suspiciously many duplicate sequences
- very long gap runs

## Exit Codes

Starting with FastaGuard v0.6.0:

```text
0 = completed report generation for PASS, WARN, and FAIL results
2 = argument parsing error
3 = configuration, input-access/I/O, runtime, or output-write error
```

QC PASS/WARN/FAIL decisions are report fields, not process-failure signals.
Pipelines should read `verdict.status`, `gate.status`, and
`gate.blocking_findings` from JSON/TSV outputs. Single-file TSV reports include
`input_path`, `verdict`, and `gate_status` for downstream routing.

`machine_summary.safe_for_downstream` is true only for an overall PASS verdict.
`gate.can_continue` is true when the selected gate has no blocking findings, so
a WARN report can have `gate.can_continue = true`. JSON-driven workflows should
use the gate fields for their configured stop/go policy and should not treat
process status or the conservative summary safety flag as the gate decision.

## Success Criteria

The first release is successful if:

- it validates huge FASTA files without loading full sequences into memory
- it produces useful HTML, JSON, TSV, and MultiQC-compatible outputs
- it catches invalid FASTA structure, duplicate IDs, invalid characters, and high-N content
- it has deterministic, documented status fields and tool-error exit codes
- it can be added to a Nextflow or Snakemake pipeline in under 5 minutes

## Implementation Status

The v0.1 assembly MVP is implemented as a Rust CLI with:

- streaming FASTA parsing for plain and gzipped files
- assembly metrics
- explainable findings
- deterministic verdict status fields
- JSON, TSV, HTML, and MultiQC-compatible outputs
