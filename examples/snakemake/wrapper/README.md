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

FastaGuard returns `0` after successfully writing PASS, WARN, or FAIL reports.
The wrapper therefore remains a thin report-producing step with four explicit
outputs, normal Snakemake logging, and caller-supplied `params.extra` arguments.
It does not add an exit-code output, `--outdir`, or `--threads`.

Apply workflow stop/go policy only after the final JSON report has been
collected. The optional local helper at
`examples/workflows/check_fastaguard_gate.py` reads `gate.can_continue` as a
strict JSON boolean and returns workflow-local status `0` for `true`, `2` for
`false`, or `3` when the field is missing or malformed. It prints the report
verdict and gate context for logs; it does not run FastaGuard and never guesses
continuation from `verdict.status` or `gate.status`.

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
