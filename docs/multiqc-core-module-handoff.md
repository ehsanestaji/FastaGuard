# MultiQC Core-Module Handoff Preparation

This document prepares the technical inputs for a possible future upstream
submission of a FastaGuard module to MultiQC. It is a handoff checklist, not an
upstream issue, pull request, review outcome, or acceptance claim. The local
plugin remains unpublished at version `0.1.0` until a separate publication
decision is made. Release policy: upstream review and merge timing do not block
the v0.7 binary release.

## Supported Runtime Floor

The candidate integration keeps the local package contract unchanged:

- Python 3.10 or newer
- MultiQC 1.28 or newer
- current compatibility targets: MultiQC 1.28 and 1.35 on Python 3.10 and the
  current supported Python release

The existing package entry points remain the reference for local testing. A
future core module would use the upstream discovery and registration mechanisms
required by the MultiQC version targeted at contribution time.

## Input Discovery

The plugin registers filename-first searches for both supported output names:

```text
fastaguard_mqc.json
*.fastaguard_mqc.json
```

The first is FastaGuard's default MultiQC custom-content filename. The prefixed
form is produced by deterministic report bundles such as
`sample-01.fastaguard_mqc.json`. Discovery should read file paths, not load
entire JSON files during the search phase.

## Minimal Parser Input

The parser accepts MultiQC custom-content JSON with `id: "fastaguard"`,
`plot_type: "table"`, and a non-empty `data` object. Each key in `data` is the
sample name and each value is one summary row. This is a minimal current input:

```json
{
  "id": "fastaguard",
  "plot_type": "table",
  "data": {
    "sample": {
      "verdict": "WARN",
      "sequence_count": 3,
      "total_length": 47,
      "n50": 16,
      "n90": 15,
      "gc_percent": 34.04,
      "n_percent": 2.13,
      "finding_count": 1,
      "gate_can_continue": true,
      "submission_policy_id": "ncbi_genome"
    }
  }
}
```

The required row fields are `verdict`, `sequence_count`, `total_length`, `n50`,
`n90`, `gc_percent`, `n_percent`, and `finding_count`. All gate, readiness,
submission, and detailed aggregate fields are optional so that custom-content
reports created before v0.7 still parse. In particular, `gate_can_continue` and
`submission_policy_id` must remain optional additions. When present,
`gate_can_continue` is the workflow-continuation boolean selected by the gate,
and `submission_policy_id` identifies the policy snapshot used for a submission
gate.

FastaGuard derives the `data` key from the input filename: it removes a trailing
`.gz`, removes the remaining filename extension, and falls back to `sample` for
an empty stem. The parser preserves that key as the MultiQC sample name. A core
module should retain normal MultiQC sample-name cleaning and collision behavior
when adapting this boundary upstream.

## Compact Display Contract

The rendered module is an aggregation view, not another copy of the evidence
report. General stats contain only:

```text
verdict
gate_can_continue
sequence_count
total_length
finding_count
n50
n_percent
```

The FastaGuard summary section contains only:

```text
verdict
gate_can_continue
gate_status
readiness_status
submission_target
submission_policy_id
submission_status
sequence_count
total_length
n50
gc_percent
n_percent
finding_count
```

The saved `multiqc_fastaguard` data keeps the full parsed aggregate row for
export and debugging. The rendered section must not reproduce `finding_ids`,
blocking-finding strings, per-record evidence, sequence data, or the larger set
of individual finding-count columns. Users should inspect FastaGuard JSON or
HTML for that evidence.

## Test Reports and Contribution Evidence

The committed strict-mode inputs are:

- `examples/reports/assembly_pass/fastaguard_mqc.json`
- `examples/reports/assembly_fail/fastaguard_mqc.json`

They exercise `gate_can_continue: true` and `gate_can_continue: false`.
Parser tests also create `submission_policy_id: "ncbi_genome"` rows and a
pre-v0.7 row that omits both new optional fields. The wheel test installs the
exact built artifact into a disposable environment, runs `multiqc --strict
--module fastaguard`, verifies the installed artifact location and versions,
and asserts the compact exported section header.

A future upstream submission package should include:

- the two small, non-sensitive test reports above or equivalent minimized
  fixtures accepted by the upstream test suite;
- expected parsed data for both samples;
- screenshots of the general-stats columns and compact FastaGuard section;
- the strict command output and compatibility results for the supported floor;
- confirmation that no raw FASTA sequence or per-record evidence is embedded in
  the screenshots or fixture additions.

## Proposed Core-Module Scope

A future upstream contribution should be limited to filename discovery, parsing
the stable custom-content summary contract, compact general stats, the compact
summary section, and focused fixtures/tests. Changes required by upstream
module conventions should remain isolated from FastaGuard's JSON source-of-truth
contract.

Non-goals are:

- publishing the local plugin or opening an upstream contribution in this phase;
- copying the complete FastaGuard HTML report or raw finding evidence into
  MultiQC;
- changing FastaGuard process exit semantics or workflow gate policy;
- replacing QUAST, BUSCO, BlobToolKit, CheckM, seqkit, MultiQC, or official
  submission validators;
- claiming biological completeness, contamination status, annotation validity,
  metadata validity, taxonomy correctness, or repository acceptance.

The FastaGuard submission profile is FASTA preflight only. It does not establish
repository acceptance, and upstream acceptance of a MultiQC module would not
change that product boundary.
