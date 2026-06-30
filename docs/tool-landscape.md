# Tool Landscape

## Positioning

FastaGuard should not compete with established downstream tools. It should make
their inputs safer and easier to triage.

Recommended slogan:

```text
Run FastaGuard first.
```

Long-form positioning:

```text
The FASTA preflight QC layer for modern bioinformatics pipelines.
```

v0.3 positioning:

```text
The assembly FASTA gate before expensive QC.
```

v0.4 positioning:

```text
Preflight readiness and starter cohort triage before interpretive QC.
```

v0.5 positioning:

```text
Submission-readiness preflight before official validators and expensive QC.
```

## Where FastaGuard Fits

| Tool | Primary role | When it runs | What FastaGuard adds before it |
| --- | --- | --- | --- |
| FastQC | Raw read QC | Before assembly or mapping | FastaGuard targets FASTA assemblies/references, not read files |
| seqkit | General sequence toolkit | Any ad hoc sequence operation | FastaGuard turns common FASTA checks into one opinionated QC contract |
| QUAST | Assembly quality evaluation | After assembly | FastaGuard catches structural FASTA problems before assembly QC |
| BUSCO | Completeness assessment | After assembly/transcriptome/protein prediction | FastaGuard checks parseability and composition before biological completeness |
| BlobToolKit | Contamination/cobiont exploration | After assembly and supporting evidence | FastaGuard flags FASTA-level anomalies before taxonomy workflows |
| MultiQC | Report aggregation | End of pipelines | FastaGuard emits data MultiQC can aggregate |
| Custom scripts | Pipeline-specific checks | Anywhere | FastaGuard replaces fragile repeated scripts with a versioned schema |

## The Gap

Without FastaGuard, users typically combine several partial checks:

- run `seqkit stats` for counts and lengths
- run custom scripts for duplicate IDs or invalid symbols
- run QUAST for assembly metrics
- run BUSCO for biological completeness
- run BlobToolKit or taxonomy tooling for contamination exploration
- rely on pipeline-specific assumptions for exit codes and report parsing

That works, but it is fragmented. The missing layer is a default, explainable,
machine-readable FASTA preflight contract.

v0.4 extends that contract with readiness categories and `fastaguard compare`.
Readiness separates file, structure, alphabet, index, assembly, submission, and
machine concerns. Compare mode gives many FASTA files one starter cohort triage
table before teams spend time in QUAST, BUSCO, BlobToolKit, CheckM, official
validators, annotation, or other interpretive tools.

v0.5 makes the submission part of readiness explicit with `--gate submission`
and `--submission-target generic|ncbi`. The useful product move is not to
replace NCBI, ENA, DDBJ, FCS, official validators, or annotation validators. It
is to catch FASTA-level submission hazards first: unsafe identifiers, duplicate
first-token IDs, invalid characters, gap-like `N` runs, high ambiguity, and
tiny-record advisories.

## Product Evidence We Have

Current product evidence:

- Rust CLI builds and runs as a single binary.
- Docker build and smoke test pass.
- GitHub release workflow builds Linux and macOS binaries.
- FastaGuard v0.3.0 is published on GitHub with Linux and macOS binaries.
- FastaGuard v0.3.0 is published on Bioconda for `linux-64`,
  `linux-aarch64`, `osx-64`, and `osx-arm64`.
- Clean Bioconda install has been smoke-tested with `fastaguard --schema`.
- BioContainers publishes
  `quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0`.
- JSON Schema validates committed golden reports.
- Reports include bounded evidence records and suggested actions.
- The v0.3 gate contract exposes `gate.blocking_findings`,
  `gate.advisory_findings`, and `provenance.input_sha256` for workflow engines.
- MultiQC custom-content JSON is emitted as `fastaguard_mqc.json`.
- A native MultiQC plugin starter exists under `integrations/multiqc/`.
- Bioconda recipe mirror exists under `packaging/bioconda/`.
- nf-core, Nextflow, and Snakemake starters exist under `examples/`.
- The v0.2 evidence workflow is documented in
  `docs/evidence/fastaguard-v0.2-evidence.md`.
- The v0.3 gate evidence workflow is documented in
  `docs/evidence/fastaguard-v0.3-evidence.md`.
- v0.4 documentation defines preflight readiness, compare mode, and local value
  benchmark framing for adoption discussions.
- v0.5 documentation defines the submission-readiness gate, local evidence
  commands, and the boundary before official validators.

Evidence still needed:

- committed benchmark summaries from public assemblies
- user feedback from real pipeline authors
- broader public assembly evidence runs
- real cohort compare-mode examples from public assemblies
- submission-readiness examples that show fixable FASTA hazards before official
  validators
- official MultiQC module or packaged plugin
- comparison examples showing what FastaGuard catches before QUAST/BUSCO/BlobToolKit

## Message Discipline

Say:

```text
FastaGuard catches FASTA-level problems before expensive downstream QC.
```

Do not say:

```text
FastQC for FASTA.
```

That phrase is tempting, but it hides the more important product idea:
FastaGuard is a pipeline-native preflight contract, not just a report.
