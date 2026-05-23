# MultiQC FastaGuard Module Starter

This directory contains a dedicated MultiQC plugin starter for FastaGuard.

FastaGuard already emits MultiQC custom-content JSON as `fastaguard_mqc.json`.
This plugin is the next step: a native module that can add FastaGuard verdicts
and key assembly preflight metrics directly to MultiQC reports.

## Local Install

From this directory:

```bash
python -m pip install -e .
cd path/to/fastaguard/results
multiqc .
```

The plugin looks for `*fastaguard_mqc.json` files and reads the same custom
content contract emitted by the CLI.

## Verification

Run the plugin against example reports in strict mode:

```bash
cd integrations/multiqc
python -m pip install -e .
cd ../..
multiqc --strict --module fastaguard examples/reports
```

## Current Scope

- Parse FastaGuard custom-content JSON.
- Add verdict and summary metrics to the MultiQC general stats table.
- Add one FastaGuard summary table section.

Keep the module compact. MultiQC should summarize many FastaGuard reports, not
replicate every field from the full FastaGuard HTML report.
