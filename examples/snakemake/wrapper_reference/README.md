# Snakemake Reference Wrapper Starter

This directory provides an upstream-ready local starter for a
`bio/fastaguard/reference` wrapper targeting `snakemake-wrappers`, alongside the
merged assembly wrapper in `../wrapper/`.

See `../../../docs/workflow-readiness.md` for the current upstream checklist.

Published Bioconda and BioContainers provide FastaGuard v1.0.0:

```bash
mamba install -c conda-forge -c bioconda fastaguard=1.0.0
docker pull quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0
```

Run the local starter from this directory with a `reference.fa`,
`reference.fa.fai`, and `reference.dict` bundle:

```bash
snakemake -s Snakefile --cores 1
```

## Inputs and Outputs

`wrapper.py` invokes `fastaguard reference` with `snakemake.input.reference` and
maps optional companion declarations when present:

- `fai`: `--fai`
- `dict`: `--dict`
- `alignment` (single path or list): repeatable `--alignment`
- `variants` (single path or list): repeatable `--variants`
- `annotation` (single path or list): repeatable `--annotation`
- `alias_map`: `--alias-map`

It emits the standard four-report bundle plus an optional lockfile when
`snakemake.output.lock` is declared:

- `html` (`--out`)
- `json` (`--json`)
- `tsv` (`--tsv`)
- `multiqc` (`--multiqc`)
- `lock` (`--write-lock --lock`, optional)

Callers pass compatibility policy and required companion kinds through
`params.extra`:

```python
params:
    extra="--policy coordinate --require fai,dict"
```

## Gate Routing

`fastaguard reference` returns exit code `0` after writing PASS, WARN, or FAIL
reports. Apply workflow stop/go policy only after the final JSON report has been
collected using `examples/workflows/check_fastaguard_gate.py`, which inspects
`gate.can_continue` as a strict JSON boolean without guessing continuation from
`verdict.status` or `gate.status`.

For a future upstream submission to `snakemake-wrappers`, use this safe local
order:

1. Run the repository Python contract tests.
2. Validate the wrapper with Snakemake in an isolated test environment.
3. Copy `wrapper.py`, `environment.yaml`, `meta.yaml`, and `test/Snakefile` into
   `bio/fastaguard/reference` in a fresh `snakemake-wrappers` checkout.
4. Run `test_wrappers.py` and regenerate `environment.linux-64.pin.txt` if
   required by upstream CI.
