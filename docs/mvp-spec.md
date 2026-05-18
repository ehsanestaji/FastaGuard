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
  --multiqc fastaguard_multiqc.json
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
MultiQC JSON = fastaguard_multiqc.json
```

## In Scope

### Inputs

- plain FASTA
- gzipped FASTA

### Profile

- assembly

### FASTA Validity

- malformed headers
- empty records
- duplicate IDs
- duplicate sequences
- invalid nucleotide/IUPAC symbols
- mixed or suspicious non-nucleotide content
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
- length histogram data
- ultra-short and ultra-long outliers

### Composition Stats

- GC percent
- AT percent
- N percent
- IUPAC ambiguity rate
- per-sequence composition outliers
- GC-vs-length anomaly data

### Assembly QC

- gap runs
- scaffold fragmentation heuristics
- suspicious tiny contigs
- high-N scaffolds
- GC-vs-length anomaly plot data

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
- duplicate IDs when default fail rules are enabled
- invalid nucleotide symbols above configured threshold

Default WARN conditions:

- high N content
- many high-N scaffolds
- excessive tiny contigs
- extreme GC outliers
- suspiciously many duplicate sequences
- very long gap runs

## Exit Codes

```text
0 = pass
1 = warnings above configured threshold
2 = hard QC failure
3 = invalid input / tool error
```

## Success Criteria

The first release is successful if:

- it validates huge FASTA files without loading full sequences into memory
- it produces useful HTML, JSON, TSV, and MultiQC-compatible outputs
- it catches duplicate IDs, invalid characters, high-N content, length outliers, and suspicious composition
- it has deterministic, documented exit codes
- it can be added to a Nextflow or Snakemake pipeline in under 5 minutes

## Implementation Status

The v0.1 assembly MVP is implemented as a Rust CLI with:

- streaming FASTA parsing for plain and gzipped files
- assembly metrics
- explainable findings
- deterministic verdict exit codes
- JSON, TSV, HTML, and MultiQC-compatible outputs
