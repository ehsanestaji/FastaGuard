# Snakemake Wrapper Starter

This is a local wrapper-style starter for FastaGuard. It assumes `fastaguard` is available on `PATH`.

Published Bioconda currently provides v0.2.0:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

The v0.3 gate example below needs a current local v0.3 binary until the v0.3
Bioconda package is published.

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

The wrapper also includes a v0.3 Conda environment for use after the v0.3
Bioconda package is published:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```

For current published containerized workflow runs, the latest pinned
BioContainers image is still v0.2:

```text
quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0
```

Do not pair that v0.2 image with `--gate pipeline`; update the container tag
when a v0.3 BioContainers image is available for gate runs.

The wrapper emits:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`
