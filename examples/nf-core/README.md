# nf-core Local Module Starter

This directory is a starter for a local nf-core-style FastaGuard module. It is not yet an upstream nf-core module.

Expected input channel:

```nextflow
tuple val(meta), path(fasta)
```

Emitted outputs:

- `html`
- `json`
- `tsv`
- `mqc`
- `versions`

The module assumes `fastaguard` is available on `PATH` when run without a
container. The recommended install is:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.3.0
```

Published BioContainers provides the pinned v0.3 image:

```text
quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0
```

The command block is written for the v0.3 assembly gate and runs:

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

That gate contract blocks downstream workflow steps on duplicate IDs, invalid
characters, invalid FASTA structure, and high-N content. Gate failures intentionally exit with code `2` after writing reports, so downstream workflow steps stop while the JSON/HTML evidence remains available.

Example include:

```nextflow
include { FASTAGUARD } from './modules/local/fastaguard'
```
