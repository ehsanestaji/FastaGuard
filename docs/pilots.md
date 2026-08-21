# Report-only pilot guide

FastaGuard pilots start in report-only mode. Run the tool locally, keep the
FASTA under the data owner's controls, and evaluate whether its structured
findings improve preflight decisions before changing workflow gates.

## Privacy boundary

Do not share FASTA files, raw sequences, or input paths. Do not paste commands
that expose paths, sample names, account names, storage locations, or project
identifiers. A FastaGuard report can contain those values as well as record IDs
inside finding evidence, so reports are not automatically safe to share.

Share only the minimum redacted report fields needed to discuss a finding.
Sequence data and local or remote paths are never part of pilot intake.

## Redaction process for support reports

1. Work on a copy; keep the original report inside the approved environment.
2. Remove `input.path`, `provenance.command`, `provenance.input_sha256`, sample
   names, filenames, timestamps, user names, host names, and storage locations.
3. Remove or replace every record `id` under finding evidence. Review free-text
   fields for identifiers or paths rather than assuming field names catch all
   sensitive values.
4. Search the copy for project identifiers, path separators, home-directory
   names, accession-like internal IDs, and command fragments.
5. Share aggregate statuses, finding IDs, counts, thresholds, and a minimal
   synthetic description where possible. Have the data owner approve the exact
   redacted extract before it leaves the approved environment.

If adequate redaction is uncertain, do not share the report. Describe the
finding category and desired help without attaching an artifact.

## Pilot intake template

Copy this template into the agreed private support channel:

```text
FastaGuard version:
Installation method:
Report schema version:
Selected profile and gate:
Redacted findings (IDs, severities, counts, thresholds only):
Workflow context (no sequence data, sample names, commands, or paths):
Observed decision or friction:
Desired follow-up:

Quoted case study consent (choose exactly one):
[ ] Yes — I give explicit consent for the approved redacted text to be quoted.
[ ] No — do not quote or publish this pilot.
```

No selection means no consent. Consent applies only to the exact reviewed,
redacted wording; it does not authorize sharing reports, paths, sequence data,
or additional context. Consent may be withdrawn before publication.

## Pilot sequence

1. Run locally and collect reports without enforcing a new gate.
2. Compare the reported findings with the team's existing preflight decisions.
3. Record false positives, missing explanations, and useful routing actions.
4. Agree on policy separately before enabling `gate.can_continue` as a workflow
   control.

The pilot evaluates FASTA-level preflight usefulness. It is not a completeness,
contamination, taxonomy, annotation, or repository-acceptance study.
