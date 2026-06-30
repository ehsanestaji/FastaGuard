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

Development scope:

- public evidence pack from local fixtures and public assemblies
- Bioconda and BioContainers v0.3 availability documented as live
- input checksum provenance with `provenance.input_sha256`
- clearer machine-readable threshold metadata
- assembly gate preset with `--gate pipeline`
- explicit `gate.blocking_findings` and `gate.advisory_findings` for workflow
  engines and humans
- clearer blocking vs follow-up recommendations; GC and length outliers remain
  advisory unless added with `--fail-on`

## v0.4: Preflight Readiness + Compare Mode

Goal:

```text
Make many FASTA files easy to rank, filter, and route before interpretive QC.
```

Development scope:

- readiness categories for file, structure, alphabet, index, assembly,
  submission, and machine readiness
- index-readiness checks for duplicate first-token IDs and related identifier
  hazards
- submission-readiness advisories for FASTA-level issues worth fixing before
  official validators
- `fastaguard compare` as a starter cohort triage layer
- cross-file metrics table
- cohort-level outliers
- sample-to-sample summary
- combined JSON, TSV, HTML, and MultiQC outputs
- explicit boundaries: compare mode routes to QUAST, BUSCO, BlobToolKit,
  CheckM, official validators, annotation, or other downstream tools; it does
  not replace them

## v0.5: Submission Readiness Gate

Goal:

```text
Make assembly FASTA files safer to hand to official validators, annotation, and downstream QC.
```

Development scope:

- `--gate submission` for stricter assembly FASTA preflight
- `--submission-target <generic|ncbi>` for target-aware submission advisories
- stricter identifier and first-token ID safety checks
- gap-like `N` run summaries for submission review
- high ambiguity and tiny-record submission advisories
- submission readiness fields in JSON, TSV, HTML, and MultiQC outputs
- compare-mode aggregation of submission readiness across many FASTA files
- evidence and release notes for `--gate submission` workflows before official
  validators
- clear scope boundaries: FastaGuard does not replace NCBI, ENA, DDBJ, FCS,
  QUAST, BUSCO, BlobToolKit, CheckM, or annotation validation
- FastaGuard does not replace NCBI, ENA, DDBJ official validators or guarantee
  repository acceptance

## v0.6: Transcriptome Profile

Potential additions:

- excessive duplicate transcripts
- polyA and polyT tails
- very short transcripts
- extreme GC outliers
- isoform-heavy warning heuristics

## v0.7: Protein Profile

Potential additions:

- invalid amino acid symbols
- internal stop codons
- terminal stop codons
- low-complexity regions
- suspicious nucleotide-looking proteins

## v0.8: Reference Panel Profile

Potential additions:

- stricter ID normalization checks
- reference naming conventions
- sequence uniqueness checks
- panel consistency summaries
- submission-readiness warnings

## Pipeline Integration Maturity

Potential additions:

- publish and verify package updates for each released contract
- upstream nf-core module submission
- official Snakemake wrapper submission
- Galaxy wrapper
- upstream MultiQC distribution path
- BioContainers verification for each published package
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
- add input checksum provenance with `provenance.input_sha256`
- add explicit scope fields for what FastaGuard can and cannot conclude
- add structured routing hints for workflow engines and tool agents
- add `--schema`, `--finding-catalog`, and `--explain-finding <id>` commands
- add golden JSON conformance tests

Recommended next sequence:

- extend evidence tables across submission, transcriptome, protein, reference, and compare modes
- keep the v0.3 gate contract stable through workflow adoption examples
- explore an MCP or tool-server interface after the CLI schema is stable
