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

## Product Evidence We Have

Current product evidence:

- Rust CLI builds and runs as a single binary.
- Docker build and smoke test pass.
- GitHub release workflow builds Linux and macOS binaries.
- FastaGuard v0.2.0 is published on GitHub with Linux and macOS binaries.
- FastaGuard v0.1.1 is published on Bioconda for Linux and macOS platforms,
  with the v0.2.0 recipe update ready for upstream merge.
- Clean Bioconda install has been smoke-tested with `fastaguard --schema`.
- JSON Schema validates committed golden reports.
- Reports include bounded evidence records and suggested actions.
- MultiQC custom-content JSON is emitted as `fastaguard_mqc.json`.
- A native MultiQC plugin starter exists under `integrations/multiqc/`.
- Bioconda recipe mirror exists under `packaging/bioconda/`.
- nf-core, Nextflow, and Snakemake starters exist under `examples/`.

Evidence still needed:

- benchmarks on public assemblies
- user feedback from real pipeline authors
- BioContainers image/tag confirmation
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
