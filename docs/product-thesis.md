# Product Thesis

## Strategic Point

FastaGuard should not compete with FastQC, QUAST, BUSCO, BlobToolKit, or MultiQC.

It should become the default FASTA preflight and triage layer before those tools run.

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

## Problem

FASTA files are a foundation of bioinformatics, but FASTA QC is fragmented. People use `seqkit stats`, QUAST, BUSCO, BlobToolKit, custom scripts, and pipeline-specific checks. These tools are valuable, but none is the simple default first command for:

```text
Is this FASTA file valid, sane, interpretable, and ready for downstream tools?
```

The result is avoidable waste:

- malformed inputs reach expensive tools
- duplicate IDs break downstream assumptions
- invalid characters surface late in pipelines
- high-N scaffolds distort annotation and mapping
- tiny contigs and composition outliers go unnoticed
- pipeline authors reinvent FASTA checks repeatedly

## Pitch

```text
FastaGuard is a fast, explainable FASTA QC tool that validates assembly FASTA files, detects structural and composition red flags, and produces pipeline-ready reports before expensive downstream analysis.
```

## Wedge

QUAST evaluates genome assemblies. BUSCO checks biological completeness. BlobToolKit helps investigate contamination and cobionts in assemblies. MultiQC aggregates reports. Those tools are powerful, but they are heavier, more context-dependent, or downstream from basic FASTA sanity.

FastaGuard owns the earlier layer:

```text
Can downstream tools safely consume this FASTA, and what obvious red flags should be handled first?
```

## Target Users

- genome assembly teams
- microbial genomics pipelines
- transcriptome assembly users after the first release
- protein FASTA users after the first release
- reference database maintainers
- nf-core, Nextflow, and Snakemake pipeline authors
- bioinformatics core facilities

## Differentiator

FastaGuard should produce a FASTA QC contract:

```text
fastaguard.json
fastaguard.tsv
fastaguard_report.html
fastaguard_mqc.json
```

The product is not only a nice report. It is a stable, versioned schema that pipelines can depend on.

That contract should also be designed for future tool-using LLM agents. Machines should be able to inspect FastaGuard output, understand the verdict, trace each finding to evidence, and choose safe next steps without scraping HTML or logs.

In that sense, FastaGuard is also a machine-actionable FASTA QC layer:

```text
Human-readable reports for scientists.
Stable structured outputs for pipelines and agents.
```

## Positioning Language

Use:

```text
The FASTA preflight QC layer for modern bioinformatics pipelines.
```

or:

```text
A lightweight, explainable QC gate for assemblies, transcriptomes, proteins, and reference FASTA files.
```

Avoid:

```text
FastQC for FASTA.
```

That comparison is tempting, but it understates the pipeline contract and creates an unnecessary frame around another tool.

## First Public Promise

```text
FastaGuard catches FASTA-level assembly problems before expensive assembly QC.
```
