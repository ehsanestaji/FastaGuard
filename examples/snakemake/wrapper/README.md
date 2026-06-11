# Snakemake Wrapper Starter

This is a local wrapper-style starter for FastaGuard. It assumes `fastaguard` is available on `PATH`.

Published Bioconda provides v0.3.0:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.3.0
```

Run from this directory with a `sample.fa` input:

```bash
snakemake -s Snakefile --cores 1
```

The wrapper command uses the v0.3 assembly gate:

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

That gate blocks downstream workflow steps on duplicate IDs, invalid characters,
invalid FASTA structure, and high-N content. GC and length outliers remain
advisory unless explicitly added with `--fail-on`. Gate failures intentionally exit with code `2` after writing reports, so downstream workflow steps stop while the JSON/HTML evidence remains available.

The wrapper also includes a v0.3 Conda environment:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```

For v0.5 submission-readiness preflight before official validators, use the
source/package contract rather than the currently published v0.3 package or
container:

```bash
fastaguard {input.fasta} --gate submission --submission-target ncbi
```

Pipeline authors should route on:

- `gate.mode`
- `gate.status`
- `gate.blocking_findings`
- `readiness.categories[id=submission]`

For containerized workflow runs, the latest pinned BioContainers image is:

```text
quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0
```

The wrapper emits:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`
