# Snakemake Wrapper Starter

This is a local wrapper-style starter for FastaGuard. It assumes `fastaguard` is available on `PATH`.

Recommended install:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

Run from this directory with a `sample.fa` input:

```bash
snakemake -s Snakefile --cores 1
```

The wrapper also includes a Conda environment:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```

For containerized workflow runs, use the pinned BioContainers image:

```text
quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0
```

The wrapper emits:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`
