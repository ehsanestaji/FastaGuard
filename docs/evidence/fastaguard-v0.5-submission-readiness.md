# FastaGuard v0.5 Submission Readiness Evidence

This page records tiny local evidence cases for the v0.5 submission-readiness
gate. The goal is to show FASTA-level hazards before official validators and
expensive QC.

## Commands

```bash
mkdir -p target/evidence/v0.5

fastaguard testdata/submission_ids.fa \
  --gate submission \
  --submission-target ncbi \
  --json target/evidence/v0.5/submission_ids.json

fastaguard testdata/submission_warnings.fa \
  --gate submission \
  --submission-target generic \
  --json target/evidence/v0.5/submission_warnings.json
```

## Scope

FastaGuard can report parse validity, identifier safety, duplicate first-token
IDs, invalid sequence symbols, gap-like N runs, high ambiguity, and tiny-record
advisories. It cannot guarantee repository acceptance, biological completeness,
annotation correctness, or contamination status.

Passing `--gate submission` means the FASTA passed FastaGuard's local
FASTA-level checks for the selected `--submission-target`. It does not mean
NCBI, ENA, DDBJ, or other official validators will accept the submission.

The optional v0.7 table2asn differential described in
[`fastaguard-v0.7-ncbi-genome-policy.md`](fastaguard-v0.7-ncbi-genome-policy.md)
checks only the explicitly shared FASTA-level boundaries. It does not expand
this evidence into a repository-acceptance or submission-readiness guarantee.

## Expected Follow-Up

After FASTA-level blockers are fixed, users should continue to official
validators, NCBI FCS, QUAST, BUSCO, BlobToolKit, CheckM, annotation, or the
next workflow step named in the report.
