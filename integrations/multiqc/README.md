# MultiQC FastaGuard Module

This directory contains the unpublished `multiqc-fastaguard` plugin package.
Its package version remains `0.1.0` until the initial release is approved and
published.

FastaGuard already emits MultiQC custom-content JSON as `fastaguard_mqc.json`.
The native module adds FastaGuard verdicts, gate continuation, readiness and
submission status, policy provenance, and a compact core metric set directly to
MultiQC reports.

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
committed pass and fail examples. The full parsed data keeps all supported
aggregate fields, including optional `gate_can_continue` and
`submission_policy_id` values when they are present. Reports that predate v0.7
and omit those fields remain valid plugin inputs.

The rendered summary is deliberately compact. Its columns are `verdict`,
`gate_can_continue`, `gate_status`, `readiness_status`, `submission_target`,
`submission_policy_id`, `submission_status`, `sequence_count`, `total_length`,
`n50`, `gc_percent`, `n_percent`, and `finding_count`. Blocker strings, raw
finding IDs, per-record evidence, and detailed finding-count columns remain in
FastaGuard's own outputs rather than being copied into this table.

The release test builds the distributions, installs that exact wheel into its
own disposable environment, and runs the same strict command. Set
`FASTAGUARD_MULTIQC_VERSION` to exercise a specific supported MultiQC version:

```bash
FASTAGUARD_MULTIQC_VERSION=1.28 python3 -m pytest -q \
  tests/python/test_multiqc_plugin.py
FASTAGUARD_MULTIQC_VERSION=1.35 python3 -m pytest -q \
  tests/python/test_multiqc_plugin.py
```

The compatibility gate covers the declared Python 3.10 floor and the current
Python 3.14 release against both MultiQC 1.28 and 1.35. The wheel also installs
this README under `share/doc/multiqc-fastaguard/README.md`; the source
distribution includes it at the archive root.

After reviewing the output, remove the exact temporary directory and the
untracked local build artifacts:

```bash
test -n "$validation_dir" && rm -rf -- "$validation_dir"
rm -rf -- integrations/multiqc/dist
```

## Scope

- Parse FastaGuard custom-content JSON.
- Add verdict, `gate_can_continue`, record count, total length, finding count,
  N50, and N percentage to the MultiQC general stats table.
- Add one compact FastaGuard summary section with gate, readiness, policy,
  submission, and core FASTA metrics.
- Preserve the full parsed data in `multiqc_fastaguard` for downstream export
  without expanding the rendered table.

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

The preparation checklist for a possible future core-module contribution is in
`docs/multiqc-core-module-handoff.md`. Upstream review and merge timing do not
block the FastaGuard v0.7 binary release.
