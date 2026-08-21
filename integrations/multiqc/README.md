# MultiQC FastaGuard Module

This directory contains the unpublished `multiqc-fastaguard` plugin package.
Its package version remains `0.1.0` until the initial release is approved and
published.

FastaGuard already emits MultiQC custom-content JSON as `fastaguard_mqc.json`.
The native module adds FastaGuard verdicts, gate status, readiness status,
submission readiness, and key assembly preflight metrics directly to MultiQC
reports.

## Development Install

From this directory:

```bash
python -m pip install -e .
cd path/to/fastaguard/results
multiqc .
```

After installation, the native FastaGuard module is discovered during a normal
MultiQC run. The plugin looks for `fastaguard_mqc.json` and
`*.fastaguard_mqc.json` files and reads the same custom content contract emitted
by the CLI.

## Build and Isolated Verification

Create a disposable environment outside the repository, build both distribution
formats, install the wheel, and run only the FastaGuard module in strict mode:

```bash
validation_dir="$(mktemp -d)"
python3 -m venv "$validation_dir/venv"
"$validation_dir/venv/bin/python" -m pip install build
"$validation_dir/venv/bin/python" -m build integrations/multiqc
"$validation_dir/venv/bin/python" -m pip install \
  integrations/multiqc/dist/multiqc_fastaguard-0.1.0-py3-none-any.whl
"$validation_dir/venv/bin/multiqc" \
  --strict \
  --module fastaguard \
  --outdir "$validation_dir/report" \
  examples/reports
```

The strict report data at
`$validation_dir/report/multiqc_data/multiqc_fastaguard.txt` should contain the
committed pass and fail examples. It includes `verdict`, `gate_mode`,
`gate_status`, `readiness_status`, `submission_target`, `submission_status`,
unsafe and long identifier counts, duplicate first-token ID counts, and
gap-like N-run counts.

After reviewing the output, remove the exact temporary directory and the
untracked local build artifacts:

```bash
test -n "$validation_dir" && rm -rf -- "$validation_dir"
rm -rf -- integrations/multiqc/dist
```

## Scope

- Parse FastaGuard custom-content JSON.
- Add verdict and summary metrics to the MultiQC general stats table.
- Add one FastaGuard summary table section with gate, readiness, and
  submission fields including `submission_target`, `submission_status`,
  unsafe identifier counts, long identifier counts, duplicate first-token ID
  counts, and gap-like N-run counts.

Keep the module compact. MultiQC should summarize many FastaGuard reports, not
replicate every field from the full FastaGuard HTML report.

## Publication Boundary

This phase validates the package locally only. It does not upload a distribution
to TestPyPI or PyPI and does not open or update a MultiQC issue or pull request.

After separate publication approval, the release owner can rebuild from a clean
checkout, inspect and check the wheel and source distribution, test the intended
index upload, publish version `0.1.0`, and verify the installed package from the
index. Any upstream MultiQC issue or pull request should be prepared only after
the public package location and installation instructions are stable.
