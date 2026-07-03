# Snakemake Wrapper Starter

This is a local wrapper-style starter for FastaGuard. It assumes `fastaguard` is available on `PATH`.

See `../../../docs/workflow-readiness.md` for the current upstream readiness
checklist before submitting this starter as an official Snakemake wrapper.

Published Bioconda provides v0.5.0:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.5.0
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

The wrapper also includes a v0.5 Conda environment:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```

For v0.5 submission-readiness preflight before official validators, use the
published v0.5 package or container:

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
quay.io/biocontainers/fastaguard:0.5.0--hfa8f182_0
```

Use this safe local order before upstream submission:

1. Run repository Python tests that inspect this wrapper layout.
2. Install Snakemake in a workflow test environment.
3. Run `snakemake -s test/Snakefile --cores 1 --use-conda`.
4. Generate a real upstream `environment.linux-64.pin.yaml` if the upstream
   wrapper repository requires a solver-produced pin file.
5. Adapt `test/test_wrappers.py` into the upstream wrapper repository test
   harness.

The wrapper emits:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`
