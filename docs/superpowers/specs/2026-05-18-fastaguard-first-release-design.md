# FastaGuard First-Release Design

Date: 2026-05-18

Status: approved for written spec, pending implementation plan

## Executive Summary

FastaGuard should become the default FASTA preflight and triage layer before heavier downstream tools run.

It should not compete with FastQC, QUAST, BUSCO, BlobToolKit, or MultiQC. Instead, it should own the earlier question:

```text
Is this assembly FASTA valid, sane, interpretable, and ready for downstream tools?
```

The v0.1 release should be assembly-first, database-free, streaming-first, and report-first.

## Product Positioning

Core thesis:

```text
FastaGuard is a fast, explainable FASTA QC tool that validates assembly FASTA files, detects structural and composition red flags, and produces pipeline-ready reports before expensive downstream analysis.
```

Strategic wording:

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

FastaGuard should explicitly avoid claims about biological completeness, assembly correctness, contamination classification, taxonomic assignment, or annotation quality. Those remain the domain of downstream tools.

The v0.1 promise is:

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```

## Target Users

- genome assembly teams
- microbial genomics pipelines
- nf-core, Nextflow, and Snakemake pipeline authors
- bioinformatics core facilities
- reference database maintainers
- transcriptome and protein FASTA users in later releases

## v0.1 Scope

The first release supports the `assembly` profile only.

Supported inputs:

- plain FASTA
- gzipped FASTA

Core checks:

- malformed headers
- empty records
- duplicate IDs
- duplicate sequences
- invalid nucleotide/IUPAC symbols
- mixed or suspicious non-nucleotide content
- bad line endings and hidden characters where detectable
- sequence count
- total length
- min, max, mean, and median length
- N50, N90, L50, and L90
- GC percent
- AT percent
- N percent
- ambiguity rate
- suspicious tiny contigs
- high-N scaffolds
- long gap runs

Planned beyond v0.1:

- per-sequence composition outliers
- length outliers
- GC-vs-length anomaly data

Outputs:

- HTML report
- JSON report
- TSV summary
- MultiQC-compatible JSON

## Non-Goals

v0.1 does not include:

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

## CLI Design

Primary command:

```bash
fastaguard sample.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_multiqc.json
```

Default command:

```bash
fastaguard sample.fa
```

Default inferred behavior:

```text
profile = assembly
HTML report = fastaguard_report.html
JSON report = fastaguard.json
TSV summary = fastaguard.tsv
MultiQC JSON = fastaguard_multiqc.json
```

Pipeline controls:

```bash
fastaguard sample.fa \
  --fail-on duplicate_ids,invalid_chars,high_n_rate,gap_runs \
  --max-n-rate 0.05 \
  --min-contig-length 200 \
  --threads 8
```

Exit codes:

```text
0 = pass
1 = warnings above configured threshold
2 = hard QC failure
3 = invalid input / tool error
```

## Output Contract

FastaGuard should produce a stable FASTA QC contract:

```text
fastaguard.json
fastaguard.tsv
fastaguard_report.html
fastaguard_multiqc.json
```

The JSON output should be versioned from the beginning. Pipeline authors should be able to depend on stable fields such as:

- `schema_version`
- `tool.version`
- `input.profile`
- `verdict.status`
- `verdict.reasons`
- `summary.sequence_count`
- `summary.total_length`
- `summary.n50`
- `summary.n90`
- `summary.l50`
- `summary.l90`
- `summary.gc_percent`
- `summary.n_percent`
- `findings[].id`
- `findings[].severity`

Example JSON shape:

```json
{
  "schema_version": "0.1.0",
  "tool": {
    "name": "FastaGuard",
    "version": "0.1.0"
  },
  "input": {
    "path": "sample.fa",
    "profile": "assembly",
    "compressed": false
  },
  "verdict": {
    "status": "WARN",
    "reasons": ["high_n_rate", "duplicate_ids"]
  },
  "summary": {
    "sequence_count": 481,
    "total_length": 5042301,
    "n50": 128003,
    "n90": 24013,
    "l50": 12,
    "l90": 81,
    "gc_percent": 51.8,
    "n_percent": 3.4
  },
  "findings": [
    {
      "id": "high_n_rate",
      "severity": "major",
      "profile": "assembly",
      "affected_count": 62,
      "affected_fraction": 0.128,
      "message": "12.8% of sequences contain more than 20% Ns.",
      "why_it_matters": "High ambiguity can reduce annotation and mapping quality.",
      "suggested_next_step": "Inspect high-N scaffolds or run gap closing/polishing."
    }
  ],
  "artifacts": {
    "html": "fastaguard_report.html",
    "tsv": "fastaguard.tsv",
    "multiqc": "fastaguard_multiqc.json"
  }
}
```

## Rust Architecture

Recommended stack:

- Rust core engine
- Rust CLI with `clap`
- streaming parser with `needletail` or `noodles-fasta`
- `serde` and `serde_json` for JSON output
- `rayon` for parallel post-processing where useful
- HTML report generated from templates
- embedded Plotly or Observable-style charts
- optional WASM report viewer later

Suggested crate shape:

```text
fastaguard/
  Cargo.toml
  src/
    main.rs
    lib.rs
    cli.rs
    parser.rs
    profile.rs
    metrics.rs
    validators.rs
    findings.rs
    report/
      mod.rs
      json.rs
      tsv.rs
      html.rs
      multiqc.rs
    stats/
      mod.rs
      nxx.rs
      composition.rs
      outliers.rs
```

Core data flow:

```text
CLI args
  -> ProfileConfig
  -> Streaming FASTA parser
  -> Per-record validation + metrics
  -> Aggregated assembly summary
  -> Findings engine
  -> Verdict engine
  -> JSON / TSV / HTML / MultiQC outputs
  -> Exit code
```

The engine should stream records and keep compact per-sequence summaries rather than full sequences:

- ID
- length
- stable hash or fingerprint for duplicate detection
- GC count
- AT count
- N count
- ambiguity count
- invalid character count
- longest gap run
- flags

Duplicate sequence detection should use a strong stable hash in v0.1. Collision confirmation can be added later if needed.

## Findings and Verdicts

Finding model:

```text
id
severity
profile
affected_count
affected_fraction
message
why_it_matters
suggested_next_step
evidence
```

Severity levels:

```text
info
minor
major
critical
```

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

## Report Design

The report should have three layers:

```text
1. Verdict
   PASS / WARN / FAIL
   top reasons
   recommended next action

2. Evidence
   summary stats
   N50 / N90 / L50 / L90
   length summary
   GC / N composition
   top problematic sequences

3. Actions
   suggested follow-up tools:
   QUAST, BUSCO, BlobToolKit, CheckM, seqkit
```

The HTML report should be self-contained, static, and shareable. It should embed plots and the summary JSON directly so it can be opened without a server.

## Testing Strategy

Unit tests:

- N50, N90, L50, L90 calculations
- composition percentages
- verdict rules
- finding generation
- duplicate ID detection
- invalid character detection

Fixture tests:

- valid small assembly
- duplicate IDs
- empty records
- invalid characters
- high-N scaffolds
- many tiny contigs
- gzipped FASTA

Snapshot or golden tests:

- JSON output schema
- TSV output
- MultiQC JSON
- representative HTML report content

## Packaging and Adoption

Day one:

- GitHub release binaries
- Docker image
- documented local build from Cargo

Early adoption:

- Bioconda recipe
- nf-core module
- Snakemake wrapper
- MultiQC module
- Galaxy wrapper

Later:

- Homebrew
- WASM/browser report viewer

## v0.1 Success Criteria

The first release is successful if:

- it validates huge FASTA files without loading full sequences into memory
- it produces useful HTML, JSON, TSV, and MultiQC-compatible output
- it catches invalid FASTA structure, duplicate IDs, invalid chars, and high-N content
- it installs with one command
- it can be added to a Nextflow or Snakemake pipeline in under five minutes
- MultiQC can consume its output

## Approved Direction

The approved first-release approach is:

```text
Assembly Preflight Contract
```

This approach makes the strategic wedge clear, delivers practical utility quickly, and avoids diluting the first release with shallow multi-profile support.
