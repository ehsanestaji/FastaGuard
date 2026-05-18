# Roadmap

## v0.1: Assembly Preflight

Goal:

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```

Capabilities:

- assembly profile
- plain and gzipped FASTA input
- streaming parser
- duplicate IDs
- duplicate sequences
- invalid nucleotide/IUPAC symbols
- empty records
- core assembly stats
- N50, N90, L50, L90
- GC, AT, N, and ambiguity rates
- high-N scaffolds
- gap runs
- tiny contig heuristics
- PASS / WARN / FAIL verdict
- JSON, TSV, HTML, and MultiQC-compatible outputs
- deterministic exit codes

## Later Assembly Enhancements

- length and composition outliers
- length histogram data
- GC-vs-length anomaly data and plots

## v0.2: Transcriptome Profile

Potential additions:

- excessive duplicate transcripts
- polyA and polyT tails
- very short transcripts
- extreme GC outliers
- isoform-heavy warning heuristics

## v0.3: Protein Profile

Potential additions:

- invalid amino acid symbols
- internal stop codons
- terminal stop codons
- low-complexity regions
- suspicious nucleotide-looking proteins

## v0.4: Reference Panel Profile

Potential additions:

- stricter ID normalization checks
- reference naming conventions
- sequence uniqueness checks
- panel consistency summaries
- submission-readiness warnings

## v0.5: Compare Mode

Example:

```bash
fastaguard compare *.fa --profile assembly --out cohort_report.html
```

Potential additions:

- cross-file metrics table
- cohort-level outliers
- sample-to-sample summary
- batch pipeline reports

## v1.x: Integration Layer

Potential additions:

- Bioconda recipe
- nf-core module
- Snakemake wrapper
- Galaxy wrapper
- dedicated MultiQC module
- Docker image
- Homebrew formula

## Later Innovation

Avoid external databases in v0.1. Later releases can explore:

- k-mer sketching with minimizers
- contamination suspicion without taxonomy
- optional sourmash, Kraken, or BlobToolKit hooks
- AI-generated report summaries from local metrics only
- browser-based interactive contig filtering
- WASM report viewer
