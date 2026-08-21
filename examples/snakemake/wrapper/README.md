# Snakemake Wrapper Reference

This directory mirrors the FastaGuard wrapper merged into
snakemake-wrappers in
[PR #5436](https://github.com/snakemake/snakemake-wrappers/pull/5436). It is a
local compatibility reference and copy-paste starter for future releases.

See `../../../docs/workflow-readiness.md` for the current update checklist.

Published Bioconda and BioContainers provide FastaGuard v0.6.0:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.6.0
docker pull quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0
```

Run the local starter from this directory with a `sample.fa` input:

```bash
snakemake -s Snakefile --cores 1
```

The starter passes its example policy through the wrapper's only optional
interface, `params.extra`:

```python
params:
    extra="--profile assembly --gate pipeline"
```

The reusable `wrapper.py` itself has no default profile or gate. It emits
exactly four reports:

- `fastaguard_report.html`
- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_mqc.json`

FastaGuard v0.6 returns `0` after successfully writing PASS, WARN, or FAIL
reports. Apply workflow stop/go policy downstream by parsing the JSON or TSV,
especially `verdict.status`, `gate.status`, and `gate.blocking_findings`.

The wrapper includes a v0.6 Conda environment:

```bash
snakemake -s Snakefile --cores 1 --use-conda
```

For submission-readiness preflight before official validators, pass a different
`params.extra` value:

```python
params:
    extra="--profile assembly --gate submission --submission-target ncbi"
```

This remains FASTA-level readiness only and does not replace repository
validators or downstream interpretive QC.

For a future upstream update, use this safe local order:

1. Run the repository Python contract tests and the local copy-paste starter.
2. Install Snakemake in an isolated workflow test environment.
3. Run the upstream `test/Snakefile` and `test_wrappers.py` suite.
4. Regenerate `environment.linux-64.pin.txt` when the upstream repository
   requires a solver-produced pin file.
5. Submit an autobump or manual update only when the published dependency or
   wrapper interface needs to change.
