# Workflow Readiness

## Current State

FastaGuard is ready for workflow use through the published v0.6.0 Bioconda
package and BioContainers image:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.6.0
docker pull quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0
```

The repository includes local starters for:

- nf-core-style Nextflow modules in `examples/nf-core/`
- Snakemake wrapper-style usage in `examples/snakemake/wrapper/`
- MultiQC custom-content aggregation through `fastaguard_mqc.json`
- evidence-preserving gate checks through
  `examples/workflows/check_fastaguard_gate.py`

The nf-core module PR [#12239](https://github.com/nf-core/modules/pull/12239)
merged 2026-08-21. The Snakemake wrapper PR
[#5436](https://github.com/snakemake/snakemake-wrappers/pull/5436) merged
2026-07-27, and autobump PR [#5737](https://github.com/snakemake/snakemake-wrappers/pull/5737)
merged 2026-07-31. The repository starters remain local compatibility
references.

Historical v0.5 upstream-style validation was run on 2026-07-03 in dedicated
checkouts:

- `nf-core modules lint fastaguard`: 47 tests passed, 0 warnings, 0 failures.
- `nf-core modules test fastaguard --profile conda --once --no-prompts`: all
  nf-test cases passed against the package available at that time.
- Snakemake wrapper formatting: `black --check` and `snakefmt --check` passed.
- Snakemake wrapper lint: `snakemake --lint --snakefile test/Snakefile` passed.
- Snakemake wrapper pytest: `test_wrappers.py::test_fastaguard` passed with
  PASS, WARN, FAIL, and invalid FASTA fixtures.

The v0.6 contract removes QC-derived process failures: successfully written
PASS, WARN, and FAIL reports all return exit code `0`. Future releases update
existing integrations after GitHub and package publication; workflow routing
should continue to use JSON or TSV status fields.

## Safe Order

1. Run local repository tests first.
2. Preserve collect-then-gate examples with
   `examples/workflows/check_fastaguard_gate.py`.
3. Verify integration updates against the current upstream test suites after a
   GitHub and package publication.

## Integration Pattern

FastaGuard should run before QUAST, BUSCO, BlobToolKit, CheckM, annotation, or
official submission validators. The default workflow pattern is:

```text
collect FASTA-level evidence -> apply stop/go policy -> route downstream tools
```

With the published v0.6 runtime, `--gate pipeline` and `--gate submission`
write JSON, TSV, HTML, and MultiQC-compatible evidence while successful report
generation returns exit code `0`. Stop/go enforcement belongs to a downstream
gate step. The important contract fields are:

- `verdict.status`
- `gate.mode`
- `gate.status`
- `gate.blocking_findings`
- `readiness.categories`
- `provenance.input_sha256`

## nf-core Integration

The merged nf-core module is tracked by
[#12239](https://github.com/nf-core/modules/pull/12239). The local reference
module carries the expected interface shape:

- input channel: `tuple val(meta), path(fasta)`
- reports: HTML, JSON, TSV, and MultiQC custom-content JSON
- version metadata on the current nf-core versions topic
- optional CLI arguments supplied by callers through `task.ext.args`
- output prefixes supplied through `task.ext.prefix ?: "${meta.id}"`
- starter nf-test fixture layout for PASS, WARN, FAIL, and invalid FASTA cases
- topic-aware `versions.yml` output for current nf-core version collection

For future updates, repeat this checklist in a fresh upstream checkout after
GitHub and package publication:

- regenerate or validate the module against the current `nf-core/tools`
  template
- run `nf-core modules lint fastaguard`
- run `nf-core modules test fastaguard --profile conda --once --no-prompts`
- adapt the local nf-test starter into the upstream repository layout
- assert that `.fastaguard.json`, `.fastaguard.tsv`, `.fastaguard.html`,
  `.fastaguard_mqc.json`, and version outputs are produced
- align `meta.yml` with current nf-core channel metadata expectations
- check current nf-core topic channels guidance for version outputs
- document that FastaGuard remains a FASTA preflight gate, not a replacement
  for downstream interpretive QC

The module should remain database-free, pinned to Bioconda/BioContainers, and
focused on stable machine-readable outputs.
It should not choose a profile or gate by default. Callers select policy through
`task.ext.args`, and downstream processes enforce it by parsing JSON or TSV.

## Snakemake Integration

The local wrapper starter already provides:

- `wrapper.py`
- `environment.yaml`
- `meta.yaml`
- `environment.linux-64.pin.txt` as a local starter pin file
- a copy-pasteable `Snakefile`
- a `test/Snakefile` starter with PASS, WARN, FAIL, and invalid FASTA fixtures
- outputs for HTML, JSON, TSV, and MultiQC custom-content JSON

The merged wrapper is tracked by
[#5436](https://github.com/snakemake/snakemake-wrappers/pull/5436). For future
updates, complete this checklist after GitHub and package publication:

- regenerate `environment.linux-64.pin.txt` from the wrapper environment if
  the upstream repository requires a solver-produced pin file
- adapt the local `test/Snakefile` and tiny FASTA fixtures
- update `test_wrappers.py` so wrapper tests run in the upstream repository
- test PASS, WARN, FAIL, and invalid FASTA behavior
- ensure the wrapper can handle arbitrary input and output paths
- keep `wrapper.py` thin, consume optional arguments only through
  `params.extra`, and avoid a default profile or gate
- preserve evidence on blocking FASTA results and enforce policy downstream by
  parsing JSON or TSV

## Submission Gate Usage

For repository-preflight workflows, use:

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate submission \
  --submission-target ncbi \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --out fastaguard_report.html \
  --multiqc fastaguard_mqc.json
```

This is FASTA-level readiness only. It can identify identifier hazards,
duplicate first-token IDs, high ambiguity, gap-like N runs, and tiny-record
advisories. It does not guarantee repository acceptance and does not replace
NCBI, ENA, DDBJ, NCBI FCS, annotation validation, QUAST, BUSCO, BlobToolKit, or
CheckM.

## References Checked

Current upstream expectations were checked on 2026-07-03 against:

- nf-core modules repository and `nf-core modules lint` workflow:
  https://github.com/nf-core/modules
- nf-core topic channels migration guidance:
  https://nf-co.re/docs/tutorials/migrate_to_topics/update_modules
- Snakemake wrappers contributing guidance:
  https://snakemake-wrappers.readthedocs.io/en/stable/contributing.html
