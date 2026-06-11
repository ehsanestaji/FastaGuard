# FastaGuard v0.5 Design: Submission Readiness Gate

## Summary

FastaGuard v0.5 should make the product's preflight position more concrete:

```text
FastaGuard catches FASTA problems that break pipelines and delay submissions.
```

v0.4 added readiness categories and compare mode. v0.5 should turn the
submission part of readiness into a deliberate gate, focused on FASTA-level
issues that users can fix before they spend time on official validators,
annotation, QUAST, BUSCO, BlobToolKit, CheckM, NCBI FCS, or submission portals.

Release theme:

```text
FastaGuard v0.5: Submission Readiness Gate
```

Product promise:

```text
Check whether an assembly FASTA is structurally safe, identifier-safe, and
submission-ready enough to continue into official validation and downstream QC.
```

This release should remain assembly-first and database-free by default. It
should not add transcriptome, protein, or reference-panel profiles yet.

## Why This Matters

The current bioinformatics landscape already has strong tools, but their roles
start later or solve broader problems:

- SeqKit, SeqFu, pyfastx, and BBTools provide fast FASTA/FASTQ manipulation or
  statistics.
- QUAST evaluates assembly quality and can compare assemblies.
- BUSCO estimates biological completeness.
- BlobToolKit and NCBI FCS help investigate contamination or foreign sequence
  signals with supporting data or databases.
- MultiQC aggregates outputs, but custom content remains more limited than a
  native module.
- NCBI, ENA, and DDBJ submission systems have their own validation rules and
  submission workflows.

FastaGuard's useful gap is the layer before those tools:

```text
Is this FASTA safe to hand to other tools and validators?
```

v0.5 should turn that into a practical user workflow.

## Goals

- Add a `submission` gate preset for assembly FASTA preflight.
- Add a `--submission-target` option with `generic` and `ncbi` as the first
  supported targets.
- Add stricter identifier and definition-line checks without changing default
  `--gate pipeline` behavior.
- Add structured submission-readiness findings with stable IDs, evidence,
  thresholds, and recommended next steps.
- Add report fields that tell pipelines and agents whether the FASTA is ready
  for official validation, annotation, and downstream QC.
- Preserve v0.4 compare mode and single-file behavior for users who do not opt
  into the submission gate.
- Update docs so users understand FastaGuard is a pre-submission preflight, not
  an official substitute for NCBI, ENA, DDBJ, FCS, QUAST, BUSCO, or BlobToolKit.

## Non-Goals

- Do not implement official NCBI, ENA, or DDBJ validation.
- Do not claim that passing FastaGuard guarantees repository acceptance.
- Do not add taxonomy databases, marker databases, aligners, read mapping, or
  internet requirements.
- Do not run NCBI FCS, BlobToolKit, QUAST, BUSCO, CheckM, or annotation tools.
- Do not infer biological completeness or confirm contamination.
- Do not add transcriptome, protein, or reference-panel profiles in v0.5.
- Do not add an LLM-facing chat feature.

## Product Position

Recommended public message:

```text
Preflight your FASTA before official validators and expensive QC.
```

Short slogan:

```text
Validate the FASTA before the pipeline pays for it.
```

Avoid:

```text
FastaGuard replaces submission validators.
```

The correct boundary is:

```text
FastaGuard finds FASTA-level risks early. Official validators and downstream
tools still decide biological, taxonomic, annotation, and submission acceptance.
```

## User Workflows

### Generic Submission Readiness

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate submission \
  --submission-target generic \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_mqc.json
```

This mode should answer:

- Are FASTA records valid and non-empty?
- Are identifiers safe for common parsers and indexes?
- Are sequence characters valid for assembly FASTA?
- Are ambiguity and gap signals likely to need explanation before submission?
- Should the user fix the FASTA before running official validation?

### NCBI-Oriented Preflight

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate submission \
  --submission-target ncbi
```

This mode should be stricter about SeqID-like concerns and gap reporting, while
remaining honest that it is not an official NCBI validator.

### Compare Mode With Submission Gate

```bash
fastaguard compare assemblies/*.fa \
  --profile assembly \
  --gate submission \
  --submission-target ncbi \
  --json submission_cohort.json \
  --tsv submission_cohort.tsv \
  --out submission_cohort.html
```

This should make a cohort-level table of which FASTA files are ready for
official validation and which should be fixed first.

## CLI Design

Extend the existing gate enum:

```text
--gate <none|pipeline|submission>
```

Add:

```text
--submission-target <generic|ncbi>
```

Default behavior:

- `--gate none`: no blocking gate, same as existing behavior.
- `--gate pipeline`: existing v0.3/v0.4 behavior.
- `--gate submission`: stricter FASTA-level blocking for submission readiness.
- If `--gate submission` is used without `--submission-target`, default to
  `generic`.
- If `--submission-target` is provided without `--gate submission`, include
  target-aware advisories in readiness output but do not change exit behavior.

Exit codes remain unchanged:

```text
0 = pass
1 = warnings above configured threshold
2 = hard QC failure
3 = invalid input / tool error
```

## Submission Targets

### Generic

The generic target should encode broad, conservative FASTA hygiene:

- no empty identifiers
- no duplicate IDs
- no duplicate first-token IDs
- no unsafe whitespace ambiguity in identifiers
- no control characters
- no invalid nucleotide/IUPAC symbols
- no empty records
- bounded identifier length advisories
- gap-run and ambiguity summaries

### NCBI

The NCBI target should add stricter SeqID-oriented checks inspired by public
NCBI submission guidance:

- warn or fail on SeqID characters that are risky for submission and downstream
  tools
- flag identifiers longer than the configured SeqID threshold
- flag spaces and pipe characters in first-token IDs
- flag definition lines with missing first-token IDs
- report long `N` runs as gap-like evidence
- route users to NCBI FCS when FASTA-level signals suggest contamination
  follow-up, while making clear that FastaGuard does not run FCS

The exact thresholds should be documented in provenance and schema.

## Finding Scope

Add or promote stable finding IDs for submission readiness:

```text
unsafe_identifier_chars
long_identifier
duplicate_first_token_ids
empty_identifier
control_characters
gap_run_summary
submission_gap_like_ns
submission_high_ambiguity
submission_tiny_records
submission_target_scope
```

Some of these may reuse existing low-level evidence if a finding already exists.
Do not create duplicate concepts if a current finding ID can be extended safely.

## Gate Behavior

`--gate submission` should fail on problems that make the FASTA structurally
unsafe for common tooling:

```text
invalid_fasta_structure
empty_records
empty_identifier
duplicate_ids
duplicate_first_token_ids
invalid_chars
control_characters
unsafe_identifier_chars
```

It should warn, not fail by default, on issues that may be legitimate but need
review or explanation:

```text
long_identifier
submission_gap_like_ns
submission_high_ambiguity
submission_tiny_records
gap_run_summary
gc_outliers
length_outliers
```

Users can still make advisory findings blocking with `--fail-on`.

## JSON Contract

Extend the existing `gate` and `readiness` fields rather than inventing a second
submission report type.

Recommended shape:

```json
{
  "gate": {
    "mode": "submission",
    "submission_target": "ncbi",
    "status": "FAIL",
    "blocking_findings": ["duplicate_first_token_ids", "unsafe_identifier_chars"],
    "advisory_findings": ["submission_gap_like_ns", "long_identifier"],
    "fail_on": [
      "duplicate_ids",
      "duplicate_first_token_ids",
      "empty_identifier",
      "empty_records",
      "invalid_chars",
      "invalid_fasta_structure",
      "unsafe_identifier_chars"
    ]
  },
  "readiness": {
    "overall": {
      "status": "FAIL",
      "summary": "FASTA should be fixed before official submission validation."
    },
    "categories": [
      {
        "id": "submission",
        "status": "FAIL",
        "target": "ncbi",
        "blocking_findings": ["unsafe_identifier_chars"],
        "advisory_findings": ["submission_gap_like_ns"]
      }
    ]
  },
  "scope": {
    "can_conclude": [
      "FASTA parse validity",
      "identifier safety",
      "assembly alphabet validity",
      "FASTA-level submission-readiness risks"
    ],
    "cannot_conclude": [
      "repository acceptance",
      "taxonomic contamination",
      "biological completeness",
      "annotation correctness"
    ]
  }
}
```

Compare reports should surface the same gate status per sample and add cohort
counts:

```text
submission_ready_count
submission_warn_count
submission_fail_count
```

## HTML Report

Add a concise "Submission Readiness" section near the top when submission
signals are present.

The section should show:

- target: `generic` or `ncbi`
- status: PASS / WARN / FAIL
- blocking problems
- advisory risks
- recommended next step
- scope note that official validators are still required

Avoid long prose. The HTML should explain the result, not become a submission
manual.

## TSV And MultiQC

Add stable summary columns:

```text
submission_target
submission_status
submission_blocking_findings
submission_advisory_findings
unsafe_identifier_count
long_identifier_count
duplicate_first_token_id_count
gap_like_n_run_count
```

The MultiQC custom-content output should include the same high-level fields so
workflow reports can sort samples by submission status.

## Evidence And Documentation

Add a small committed evidence page for v0.5:

```text
docs/evidence/fastaguard-v0.5-submission-readiness.md
```

It should include tiny synthetic cases:

- clean assembly FASTA
- duplicate first-token IDs
- unsafe identifier characters
- long identifier
- long N runs
- invalid sequence character

Do not commit large public FASTA files.

Docs to update:

- `README.md`
- `docs/roadmap.md`
- `docs/vision-plan.md`
- `docs/tool-landscape.md`
- `docs/output-contract.md`
- `docs/packaging.md`
- `docs/releases/v0.5.0.md`
- `examples/nf-core/README.md`
- `examples/snakemake/README.md`

## Tests

Add focused tests for:

- CLI accepts `--gate submission`
- CLI accepts `--submission-target generic`
- CLI accepts `--submission-target ncbi`
- unknown submission target exits with code `3`
- submission gate fails duplicate first-token IDs
- submission gate fails unsafe identifier characters
- submission gate warns on long identifiers
- submission gate warns on long N runs
- `--fail-on long_identifier` can make long identifiers blocking
- JSON schema validates new fields
- golden JSON fixtures include submission pass/warn/fail cases
- HTML contains "Submission Readiness"
- TSV includes submission columns
- MultiQC output includes submission fields
- compare mode aggregates submission status deterministically

Run the usual gates:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

## Release Strategy

Before tagging v0.5.0:

1. Merge v0.4.0 into Bioconda or document clearly that Bioconda remains behind.
2. Implement submission readiness behind explicit `--gate submission`.
3. Preserve `--gate pipeline` behavior unless tests intentionally prove an
   unchanged report contract.
4. Regenerate schema, examples, and golden reports.
5. Add v0.5 release notes with clear boundaries.
6. Tag and publish GitHub release.
7. Update Bioconda after the public source archive exists.

## Success Criteria

v0.5 is successful if:

- a user can run one command before official validation and see fixable FASTA
  risks immediately
- pipeline authors can route on `gate.mode = submission` and
  `readiness.categories[id=submission]`
- no downstream tool claims are overstated
- the report tells users when to continue to NCBI FCS, QUAST, BUSCO,
  BlobToolKit, CheckM, annotation, or official validators
- all outputs remain deterministic and schema-validated

## Recommended Implementation Order

1. Add CLI enums and no-op serialization support for `submission_target`.
2. Add tests for target parsing and unchanged default behavior.
3. Add identifier-safety analyzer functions with focused unit tests.
4. Add submission gate failure/advisory mapping.
5. Extend JSON schema and golden fixtures.
6. Extend TSV, MultiQC, and HTML outputs.
7. Add compare-mode aggregation fields.
8. Update docs, examples, and release notes.
9. Run full verification gates.

This order keeps the contract clear before touching report presentation.
