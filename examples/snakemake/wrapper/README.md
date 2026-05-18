# Snakemake Wrapper Starter

This is a local wrapper-style starter for FastaGuard. It assumes `fastaguard` is available on `PATH` until Bioconda packaging exists.

Run from this directory with a `sample.fa` input:

```bash
snakemake -s Snakefile --cores 1
```

The wrapper emits:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`
