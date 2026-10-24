# Paper track handoff (cold-start)

For any agent or human resuming FastaGuard **paper preparation** after a pause.
This track is **paper-only**. Do not change product CLI/schema/packaging unless
the user explicitly expands scope. Product/dev work may be on a separate track
(e.g. Aragon / other agents) — do not conflict with it.

## Locked decisions

| Item | Value |
| --- | --- |
| Venue | ***Bioinformatics* (OUP) — Application Note** (**user-confirmed**) |
| Working title | `FastaGuard: FASTA preflight QC for modern bioinformatics pipelines` |
| Framing | FASTA **preflight** contract before QUAST/BUSCO/BlobToolKit/etc. |
| Forbidden phrase | Do **not** use “FastQC for FASTA” |
| Repo branch / PR | `cursor/paper-prep-venues-198e` / [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) |
| LaTeX root | `paper/main.tex` |

## What is done

- Full Application Note **first draft** in `main.tex` (Motivation, Implementation, Results, Discussion).
- Structured abstract (Summary / Availability and Implementation / Contact / Supplementary).
- TikZ main figure (preflight placement story).
- Evidence table filled from committed v0.6 public-evidence pack (`docs/evidence/`).
- BibTeX `references.bib` with core ecosystem citations.
- `Makefile` + build README; PDF builds with TeX Live (`pdflatex` + `bibtex` via `latexmk`).
- Project store venue brief + project context (paths below).

## Draft vs final

| Piece | Status |
| --- | --- |
| Scientific framing / claims | Draft-complete; aligned with `docs/product-thesis.md` |
| Author email / affiliation | **TBD placeholders** |
| Funding / COI details | **TBD** (COI currently “none declared”) |
| Figure | **Usable TikZ**; optional polish / export to PDF art later |
| Table numbers | From **v0.6** evidence (contextual timings); re-freeze if citing newer release |
| Word count | Aim ~2,000–2,600; recount after edits (`paper/HANDOFF.md` workflow) |
| OUP camera-ready template | **Not yet** — format-free OK for first submission |
| Preprint / Zenodo DOI | **Not yet** |
| Submission to ScholarOne | **Not yet** |

## How to build

```bash
cd paper
make          # latexmk -pdf main.tex → main.pdf
make clean    # aux only
```

Requires: `pdflatex`, `bibtex`, `latexmk`, and packages used in `main.tex`
(including `tikz` from `texlive-pictures`). PDF is gitignored.

## Store docs (durable Project Context)

- Venue + framing: `/cursor/stores/bc-53e26eb2-1f92-41f8-9fa2-384c56a6ad53/docs/publication-venues.md`
- Project context / resume: `/cursor/stores/bc-53e26eb2-1f92-41f8-9fa2-384c56a6ad53/docs/project-context.md`

(Also via symlink `/cursor/stores/self/docs/...` when that alias is mounted.)

## Next concrete manuscript steps

1. Replace `[institutional-email@example.org]` and affiliation; complete funding.
2. Recount words; trim or expand to stay near Application Note budget (~2k–2.6k words / ~4 typeset pages). Enable `\doublespacing` before ScholarOne upload if desired.
3. Optionally re-run evidence for the exact software version named in Availability (v1.0.0 packaging vs newer source tag) and update Table 1.
4. Deposit preprint (bioRxiv) and/or archive DOI (Zenodo) if desired before or with submission.
5. Submit format-free PDF + SI; on revision, switch to OUP General Template.
6. Confirm APC / Read & Publish / ISCB discount before acceptance.

## Key in-repo sources for claims

- `README.md`, `docs/product-thesis.md`, `docs/tool-landscape.md`, `docs/output-contract.md`
- `docs/evidence/fastaguard-v0.6-public-evidence.md` + `docs/evidence/results/v0.6/`
- `CITATION.cff`
