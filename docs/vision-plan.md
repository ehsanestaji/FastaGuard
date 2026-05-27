# FastaGuard Vision Plan

## North Star

FastaGuard should become the FASTA preflight operating system for modern
bioinformatics pipelines.

That does not mean replacing FastQC, QUAST, BUSCO, BlobToolKit, CheckM, seqkit,
MultiQC, or annotation workflows. It means owning the layer before them:

```text
Validate the FASTA.
Explain the red flags.
Emit a stable contract.
Route to the right downstream tools.
```

The mature product should feel boringly reliable to pipeline authors and
surprisingly useful to scientists: one fast command that tells them whether a
FASTA file is valid, sane, interpretable, and ready for heavier analysis.

## Strategic Release Principle

The release strategy is evidence before expansion.

FastaGuard should not rush into many biological profiles until the assembly
preflight contract is trusted. The product should earn adoption in this order:

1. **Trust:** reproducible evidence, stable schemas, clear exit codes, installable packages.
2. **Integration:** Bioconda, BioContainers, MultiQC, Nextflow, Snakemake, Galaxy.
3. **Scale:** compare mode for many FASTA files and batch pipeline reports.
4. **Breadth:** transcriptome, protein, and reference-panel profiles.
5. **Intelligence:** optional local-metrics-only summaries, MCP/tool-agent interfaces, and workflow routing.

This keeps the project from becoming a bag of heuristics. Each release should
make the contract more useful, more trusted, or more integrated.

## Big Release Direction

### v0.3: Evidence And Assembly Gate

Goal:

```text
Make FastaGuard credible enough for pipeline authors to add as a default assembly gate.
```

Priorities:

- publish a small evidence pack from local fixtures and public assemblies
- document Bioconda and BioContainers v0.2 availability
- add input checksums to provenance
- add clearer machine-readable threshold metadata
- add an assembly gate preset for common pipeline behavior
- improve report sections that explain what should block downstream tools

The v0.3 promise should be:

```text
FastaGuard gives assembly pipelines a fast, explainable PASS/WARN/FAIL gate before expensive QC.
```

### v0.4: Compare Mode

Goal:

```text
Make many FASTA files easy to rank, filter, and route.
```

Compare mode should support:

- cohort-level table across many FASTA files
- sample-to-sample summaries
- batch outlier detection
- combined HTML report
- combined JSON/TSV/MultiQC output
- stable machine-actionable ranking fields

This is more strategically important than adding many profile-specific checks
too early, because pipeline authors often need to triage batches, not one file.

### v0.5: Transcriptome Profile

Goal:

```text
Extend the FASTA preflight contract to transcriptome assemblies.
```

Initial transcriptome checks should stay lightweight:

- very short transcripts
- excessive duplicate transcript sequences
- polyA/polyT tail summaries
- GC outliers
- isoform-heavy warning heuristics

FastaGuard should not claim transcriptome biological completeness. It should
route users to transcriptome-specific completeness and annotation tools when
needed.

### v0.6: Protein Profile

Goal:

```text
Validate protein FASTA files before annotation, clustering, search, or database submission.
```

Initial protein checks:

- invalid amino-acid symbols
- internal stop codons
- terminal stop codons
- low-complexity summaries
- suspicious nucleotide-looking proteins

Protein mode should be strict about alphabet validity and careful about biology:
it should flag preflight problems, not infer functional correctness.

### v0.7: Reference-Panel Profile

Goal:

```text
Make curated reference FASTA panels safer to maintain and distribute.
```

Initial reference-panel checks:

- stricter ID normalization
- naming convention reports
- sequence uniqueness
- panel consistency summaries
- submission-readiness warnings

This is useful for labs, databases, and core facilities maintaining reference
sets that many downstream workflows depend on.

## Machine-Actionable Vision

FastaGuard should be designed for humans, pipelines, and future tool-using
agents.

Principles:

- JSON remains the source of truth.
- HTML remains a human view.
- Machines should never scrape HTML or logs.
- Finding IDs must remain stable and documented.
- Every finding should expose severity, evidence, thresholds, actions, and scope.
- Optional generated summaries must be local-metrics-only and traceable back to structured fields.
- MCP or tool-server support should come after the CLI contract is stable.

The long-term machine-actionable direction:

```text
fastaguard run sample.fa --json report.json
fastaguard compare *.fa --json cohort.json
agent reads schema + finding catalog + report
agent routes safely to QUAST, BUSCO, BlobToolKit, CheckM, seqkit, or annotation
```

The agent should know what FastaGuard can conclude, what it cannot conclude,
and which downstream tool is appropriate next.

## Product Boundaries

FastaGuard should remain fast and database-free by default.

Do not make default FastaGuard depend on:

- taxonomy databases
- large marker-gene databases
- internet access
- GPU inference
- external aligners

Optional integrations can exist later, but the default product should stay a
single reliable preflight binary that works in constrained pipelines.

## Adoption Strategy

The project should optimize for maintainers and workflow authors.

Required adoption qualities:

- one-command install through Bioconda
- generated BioContainers image
- stable JSON schema
- deterministic outputs
- clear exit codes
- MultiQC compatibility
- Nextflow, nf-core, Snakemake, and Galaxy examples
- small public evidence pack
- clear release notes and migration notes

The best way to become frequent in bioinformatics pipelines is not flashy AI.
It is being the boring, dependable first QC gate that saves expensive downstream
time.

## Current Recommendation

The next big release should not be a huge biology expansion yet.

Recommended sequence:

```text
v0.3: evidence pack + assembly gate + provenance checksums
v0.4: compare mode for many FASTA files
v0.5: transcriptome profile
v0.6: protein profile
v0.7: reference-panel profile
later: MCP/tool-agent interface and optional local summaries
```

This path gives FastaGuard the best chance to become a default tool: prove the
assembly gate first, then scale to batches, then expand profiles.
