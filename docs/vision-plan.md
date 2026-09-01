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

1. **Trust:** reproducible evidence, stable schemas, clear status fields, installable packages.
2. **Integration:** Bioconda, BioContainers, MultiQC, Nextflow, Snakemake, Galaxy.
3. **Scale:** compare mode for many FASTA files and batch pipeline reports.
4. **Readiness depth:** submission-oriented preflight checks before official validators.
5. **Breadth:** transcriptome, protein, and reference-panel profiles.
6. **Intelligence:** optional local-metrics-only summaries, MCP/tool-agent interfaces, and workflow routing.

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
- document Bioconda and BioContainers v0.3 availability
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

### v0.5: Submission Readiness Gate

Goal:

```text
Make assembly FASTA files safer to hand to official validators, annotation, and downstream QC.
```

Submission readiness should stay FASTA-level and database-free:

- explicit `--gate submission` workflow behavior
- stricter identifier and first-token ID safety checks
- target-aware advisories with `--submission-target generic|ncbi`
- gap-like `N` run summaries
- high ambiguity and tiny-record submission advisories
- JSON, TSV, HTML, and MultiQC fields that pipelines can route on
- clear recommendations to continue with official validators, NCBI FCS, QUAST,
  BUSCO, BlobToolKit, CheckM, or annotation tools when appropriate

FastaGuard should not claim repository acceptance, biological completeness,
annotation correctness, or contamination confirmation. It should help users fix
FASTA-level blockers before those later checks.

The v0.5 contract should make workflow routing explicit through `gate.mode`,
`gate.status`, `gate.blocking_findings`, and the
`readiness.categories[id=submission]` record, while still pointing users to
official validators for final repository-specific checks.

### v0.6: Workflow-Compatible Exit Contract

Goal:

```text
Let workflows collect complete QC reports before applying their own stop/go policy.
```

Successful report generation should exit with code `0` for PASS, WARN, and FAIL
reports. Argument parsing errors should use code `2`; configuration,
input-access, runtime, and output-write errors should use code `3`. JSON and TSV
remain the source of truth for `verdict.status`, `gate.status`, and blocking
findings.

Single-file TSV reports include `input_path` alongside status fields so workflow
engines can route samples without scraping logs or HTML. FastaGuard v0.6.0 is
published on GitHub, Bioconda, and BioContainers; future releases update
existing integrations after GitHub and package publication.

### v0.7: Operational Trust

Goal:

```text
Make the assembly preflight contract predictable to run, publish, and automate.
```

Operational trust includes:

- deterministic four-report bundles with exact filenames
- no-clobber output validation with explicit `--force` replacement
- per-file temporary staging before sequential publication
- an NCBI genome FASTA policy with stable provenance and explicit exclusions
- a documented distinction between summary safety and gate continuation
- complete release archives with the binary, README, license, and schema assets

The process exit contract remains:

```text
0 = completed report generation for PASS, WARN, and FAIL results
2 = argument parsing error
3 = configuration, input-access/I/O, runtime, or output-write error
```

### Future Biological Profiles

Goal:

```text
Extend the trusted FASTA contract through separately scoped biological profiles.
```

Transcriptome, protein, and reference-panel work remains valuable, but it is
deferred until each profile has its own contract and evidence scope. Candidate
checks include:

- very short transcripts, duplicate transcript sequences, and polyA/polyT tails
- invalid amino-acid symbols
- internal stop codons
- terminal stop codons
- low-complexity summaries
- suspicious nucleotide-looking proteins
- stricter ID normalization
- naming convention reports
- sequence uniqueness
- panel consistency summaries
- submission-readiness warnings

Future profiles must keep the same product boundary: flag preflight hazards and
route to biological completeness, annotation, or validation tools without
claiming those conclusions.

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
- clear tool-error exit codes
- MultiQC compatibility
- Nextflow, nf-core, Snakemake, and Galaxy examples
- small public evidence pack
- clear release notes and migration notes

The best way to become frequent in bioinformatics pipelines is not flashy AI.
It is being the boring, dependable first QC gate that saves expensive downstream
time.

## Current Recommendation

FastaGuard v1.0.0-rc.1 is the current source release candidate. It adds the
Reference Contract Gate while preserving the operational-trust behaviour of
v0.7. Published GitHub, Bioconda, and BioContainers artifacts remain v0.6.0.
Bioconda serves `linux-64`, `linux-aarch64`, `osx-64`, and `osx-arm64`, and the
published BioContainers tag is `0.6.0--hfa8f182_0`.

Recommended sequence:

```text
v0.3: evidence pack + assembly gate + provenance checksums
v0.4: compare mode for many FASTA files
v0.5: submission readiness gate
v0.6: workflow-compatible exit contract
v0.7: operational trust for outputs, policies, gating, and archives
v1.0: Reference Contract Gate for explicit reference compatibility
future: separately scoped transcriptome, protein, and reference-panel profiles
later: MCP/tool-agent interface and optional local summaries
```

This path gives FastaGuard the best chance to become a default tool: prove the
assembly gate first, scale to batches, make submission readiness concrete,
simplify workflow integration, harden operational behavior, then scope each new
profile independently.
