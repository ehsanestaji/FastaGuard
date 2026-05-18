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

## LLM And Tool-Agent Readiness

FastaGuard should prepare for a future where machines talk to QC tools directly. The near-term path is not a chatbot feature; it is a cleaner contract.

Completed foundation:

- publish `schema/fastaguard.schema.json`
- document a stable finding catalog
- add structured `actions[]` records to findings
- add bounded per-record evidence to findings
- add provenance for profile, thresholds, fail rules, and thread count
- add explicit scope fields for what FastaGuard can and cannot conclude
- add `--schema`, `--finding-catalog`, and `--explain-finding <id>` commands
- add golden JSON conformance tests

Recommended next sequence:

- extend evidence tables across future transcriptome, protein, reference, and compare modes
- enrich provenance with command, timestamps, input size, and checksums
- explore an MCP or tool-server interface after the CLI schema is stable
