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
mamba install -c conda-forge -c bioconda fastaguard
```

The local module also includes the pinned BioContainers image:

```text
quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0
```

Example include:

```nextflow
include { FASTAGUARD } from './modules/local/fastaguard'
```
