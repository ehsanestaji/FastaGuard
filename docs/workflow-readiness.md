# Workflow Readiness

## Current State

FastaGuard is ready for local workflow use through the published v0.5.0
Bioconda package and BioContainers image:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.5.0
docker pull quay.io/biocontainers/fastaguard:0.5.0--hfa8f182_0
```

The repository includes local starters for:

- nf-core-style Nextflow modules in `examples/nf-core/`
- Snakemake wrapper-style usage in `examples/snakemake/wrapper/`
- MultiQC custom-content aggregation through `fastaguard_mqc.json`
- evidence-preserving gate checks through
  `examples/workflows/check_fastaguard_gate.py`

These are workflow adoption starters. They are not yet an upstream nf-core module.
They are not yet an official Snakemake wrapper.

External upstream-style validation was run on 2026-07-03 in dedicated checkouts:

- `nf-core modules lint fastaguard`: 47 tests passed, 0 warnings, 0 failures.
- `nf-core modules test fastaguard --profile conda --once --no-prompts`: all
  nf-test cases passed against Bioconda v0.5.0.
- Snakemake wrapper formatting: `black --check` and `snakefmt --check` passed.
- Snakemake wrapper lint: `snakemake --lint --snakefile test/Snakefile` passed.
- Snakemake wrapper pytest: `test_wrappers.py::test_fastaguard` passed with
  PASS, WARN, FAIL, and invalid FASTA fixtures plus captured `exit_code`
  outputs.

The v0.6 source contract removes QC-derived process failures: successfully
written PASS, WARN, and FAIL reports all return exit code `0`. The pinned v0.5
starters keep their compatibility capture until v0.6 packages and containers
are published. After publication, upstream wrappers can remove that workaround
and route directly from JSON or TSV status fields.

## Safe Order

1. Run local repository tests first.
2. Harden the nf-core starter with topic-aware versions, fixtures, and nf-test
   starter coverage.
3. Add collect-then-gate examples with `examples/workflows/check_fastaguard_gate.py`.
4. Harden the Snakemake wrapper starter with metadata, fixture tests, and a
   starter pin file.
5. Run external `nf-core modules lint`, `nf-core modules test`, and Snakemake
   wrapper tests in dedicated upstream checkouts.
6. Open external upstream PRs last.

## Integration Pattern

FastaGuard should run before QUAST, BUSCO, BlobToolKit, CheckM, annotation, or
official submission validators. The default workflow pattern is:

```text
collect FASTA-level evidence -> apply stop/go policy -> route downstream tools
```

With the published v0.5 runtime, `--gate pipeline` and `--gate submission` write
JSON, TSV, HTML, and MultiQC-compatible evidence before returning a QC-derived
exit code. The pinned local starters capture that code so the evidence remains
available, then leave stop/go enforcement to a downstream gate step. The
important contract fields are:

- `verdict.status`
- `gate.mode`
- `gate.status`
- `gate.blocking_findings`
- `readiness.categories`
- `provenance.input_sha256`

## nf-core Readiness

The local module already carries the expected interface shape:

- input channel: `tuple val(meta), path(fasta)`
- outputs: HTML, JSON, TSV, MultiQC custom-content JSON, captured exit code,
  and versions metadata
- runtime: `bioconda::fastaguard=0.5.0`
- container: `quay.io/biocontainers/fastaguard:0.5.0--hfa8f182_0`
- starter nf-test fixture layout for PASS, WARN, FAIL, and invalid FASTA cases
- topic-aware `versions.yml` output for current nf-core version collection

Before an upstream nf-core module submission, repeat this checklist in a fresh
upstream checkout:

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

The upstream submission should keep the module boring: database-free, pinned to
Bioconda/BioContainers, and focused on stable machine-readable outputs.

## Snakemake Readiness

The local wrapper starter already provides:

- `wrapper.py`
- `environment.yaml`
- `meta.yaml`
- `environment.linux-64.pin.txt` as a local starter pin file
- a copy-pasteable `Snakefile`
- a `test/Snakefile` starter with PASS, WARN, FAIL, and invalid FASTA fixtures
- outputs for HTML, JSON, TSV, MultiQC custom-content JSON, and captured exit
  code

Before an official Snakemake wrapper submission, complete this checklist:

- regenerate `environment.linux-64.pin.txt` from the wrapper environment if
  the upstream repository requires a solver-produced pin file
- adapt the local `test/Snakefile` and tiny FASTA fixtures
- update `test_wrappers.py` so wrapper tests run in the upstream repository
- test PASS, WARN, FAIL, and invalid FASTA behavior
- ensure the wrapper can handle arbitrary input and output paths
- preserve evidence on blocking FASTA results, either through workflow-specific
  output handling or a collect-then-gate wrapper pattern

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
