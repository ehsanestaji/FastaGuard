# Paper track handoff (cold-start)

Paper-only track. Do not change product CLI/schema/packaging unless asked.
Venue: ***Bioinformatics* (OUP) Application Note** — **user-confirmed**.
PR: [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) · branch `cursor/paper-prep-venues-198e`.

## Machine-ready vs USER-ACTION

| Status | Items |
| --- | --- |
| **DONE / submit-package ready** | Full draft; front-matter; evidence freeze; `submission.yml`; `check_submission.py`; cover letter; `.zenodo.json`; bioRxiv build target; `docs/paper-readiness.md` |
| **USER-ACTION only** | Co-author final OK; Zenodo DOI mint (`paper/ZENODO.md`); optional bioRxiv; ScholarOne upload; APC/R&P/ISCB |

Do **not** claim ScholarOne submitted from this agent track.

## Commands

```bash
cd paper
make check     # must exit 0
make           # main.pdf
make biorxiv   # build/fastaguard-biorxiv.pdf
```

## Freezes

- **Software Availability cite:** GitHub **`v1.0.0`** (Bioconda `fastaguard=1.0.0`)
- **Table 1 evidence:** `docs/evidence/results/v0.6/evidence_summary.tsv` (fastaguard **0.6.0**, commit `cf27295…`)
- **Forbidden phrase:** never use “FastQC for FASTA” affirmatively

## Copied from VariantFlow (front-matter only)

Authors/ORCID/Umeå/emails; WIFORCE funding; COI; AI disclosure; CRediT; SNIC ack; sectioning conventions.

## Not copied

VariantFlow VCF claims, benchmarks, tables, Zenodo/bioRxiv DOIs.

## Key paths

- Manifest: `paper/submission.yml`
- Readiness: `docs/paper-readiness.md`
- Store context: `/cursor/stores/bc-53e26eb2-1f92-41f8-9fa2-384c56a6ad53/docs/project-context.md`

## Zenodo DOI (2026-10-24)

- Version DOI: `10.5281/zenodo.23622311` (https://zenodo.org/records/23622311)
- Concept DOI: `10.5281/zenodo.23622310`
- Source: GitHub release `v1.0.0` via Zenodo GitHub integration
