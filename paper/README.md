# FastaGuard paper (LaTeX)

**Venue (user-confirmed):** [*Bioinformatics*](https://academic.oup.com/bioinformatics) — **Application Note**

**PR:** [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) · `cursor/paper-prep-venues-198e`

Paper-prep only. Cold-start: [`HANDOFF.md`](HANDOFF.md). Readiness checklist: [`../docs/paper-readiness.md`](../docs/paper-readiness.md).

## Build & check

```bash
cd paper
make check      # submission control (must pass)
make            # → main.pdf
make biorxiv    # → build/fastaguard-biorxiv.pdf
```

Requires TeX Live (`pdflatex`, `bibtex`, `latexmk`, TikZ, `microtype`) and Python 3.

## Submission package files

| File | Role |
| --- | --- |
| `main.tex` / `references.bib` | Manuscript |
| `submission.yml` | Control manifest (versions, freezes, blockers) |
| `check_submission.py` | Machine gate |
| `cover-letter.md` | ScholarOne cover letter draft |
| `ZENODO.md` | USER-ACTION DOI mint from `v1.0.0` |
| `biorxiv-checklist.md` | Optional preprint steps |
| `../.zenodo.json` | Zenodo metadata (no DOI invented) |

## Freezes

- Availability software: **`v1.0.0`**
- Table 1 evidence: **v0.6.0** pack under `docs/evidence/results/v0.6/`
- Format-free first submission OK; OUP General Template deferred to revision

## Framing

Never pitch as “FastQC for FASTA.”
