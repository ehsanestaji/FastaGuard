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
- length histogram data and HTML plot
- GC-vs-length plot data and HTML plot
- PASS / WARN / FAIL verdict
- JSON, TSV, HTML, and MultiQC-compatible outputs
- deterministic exit codes

## v0.2: Assembly Trust + Pipeline Adoption

Goal:

```text
Make assembly preflight reports easier to trust, route, and consume in real pipelines.
```

Capabilities:

- GC, length, and composite assembly outlier findings
- finding taxonomy, confidence, and follow-up-tool metadata
- structured routing hints for workflow engines and tool agents
- richer provenance with command, timestamps, duration, and input size
- expanded TSV summary rows for outlier counts
- hardened MultiQC plugin and custom-content support
- Snakemake and nf-core starter material
- ready Bioconda recipe metadata for the v0.2.0 update
- benchmark evidence guidance for adoption decisions

## v0.3: Evidence And Assembly Gate

Goal:

```text
Make FastaGuard credible enough for pipeline authors to add as a default assembly gate.
```

Potential additions:

- public evidence pack from local fixtures and public assemblies
- Bioconda and BioContainers v0.2 availability documented
- input checksum provenance
- clearer machine-readable threshold metadata
- assembly gate preset for common pipeline behavior
- clearer blocking vs follow-up recommendations

## v0.4: Compare Mode

Goal:

```text
Make many FASTA files easy to rank, filter, and route.
```

Potential additions:

- cross-file metrics table
- cohort-level outliers
- sample-to-sample summary
- batch pipeline reports
- combined JSON, TSV, HTML, and MultiQC outputs

## v0.5: Transcriptome Profile

Potential additions:

- excessive duplicate transcripts
- polyA and polyT tails
- very short transcripts
- extreme GC outliers
- isoform-heavy warning heuristics

## v0.6: Protein Profile

Potential additions:

- invalid amino acid symbols
- internal stop codons
- terminal stop codons
- low-complexity regions
- suspicious nucleotide-looking proteins

## v0.7: Reference Panel Profile

Potential additions:

- stricter ID normalization checks
- reference naming conventions
- sequence uniqueness checks
- panel consistency summaries
- submission-readiness warnings

## Pipeline Integration Maturity

Potential additions:

- merge and verify the v0.2 Bioconda update
- upstream nf-core module submission
- official Snakemake wrapper submission
- Galaxy wrapper
- upstream MultiQC distribution path
- BioContainers verification for the v0.2 package
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
- add provenance for profile, thresholds, fail rules, thread count, command, timestamps, duration, and input size
- add explicit scope fields for what FastaGuard can and cannot conclude
- add structured routing hints for workflow engines and tool agents
- add `--schema`, `--finding-catalog`, and `--explain-finding <id>` commands
- add golden JSON conformance tests

Recommended next sequence:

- extend evidence tables across future transcriptome, protein, reference, and compare modes
- enrich provenance with input checksums
- explore an MCP or tool-server interface after the CLI schema is stable
