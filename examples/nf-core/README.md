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

The module assumes `fastaguard` is available on `PATH` until a Bioconda package and BioContainers image exist.

Example include:

```nextflow
include { FASTAGUARD } from './modules/local/fastaguard'
```
