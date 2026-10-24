# FastaGuard paper (LaTeX)

**Venue (user-confirmed):** [*Bioinformatics*](https://academic.oup.com/bioinformatics) (Oxford Academic) — **Application Note**.

**PR / branch:** [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) · `cursor/paper-prep-venues-198e`

This folder is **paper preparation only**. It does not change the FastaGuard product.
Cold-start resume details: see [`HANDOFF.md`](HANDOFF.md).

## Article constraints

| Item | Target |
| --- | --- |
| Type | Application Note |
| Length | Up to ~4 pages (~2,600 words, or ~2,000 words + one figure) |
| Title | Must include the application name (`FastaGuard: …`) |
| Abstract | Summary / Availability and Implementation / Contact / Supplementary information |
| Software | Freely available; state availability + implementation; keep available ≥2 years |

Format-free initial submission is allowed. For revision / camera-ready, switch to the [OUP General Template](https://www.overleaf.com/latex/templates/oup-general-template/fqkhysbcbpwv) with Bioinformatics options (`numsec,webpdf,modern,large`) and author-date bibliography style as documented by OUP.

## Layout

```text
paper/
  main.tex           Full Application Note draft
  references.bib     BibTeX database
  Makefile           latexmk / pdflatex build
  README.md          this file
  HANDOFF.md         cold-start status + next steps
  figures/           notes for optional external artwork
  tables/            optional CSV / working tables
```

## Build the PDF

Requires TeX Live (or MacTeX / MiKTeX) with `pdflatex`, `bibtex`, `latexmk`, plus
`geometry`, `times`, `graphicx`, `booktabs`, `hyperref`, `xcolor`, `natbib`,
`setspace`, `lineno`, `url`, and **TikZ** (`texlive-pictures`).

```bash
cd paper
make              # → main.pdf
make pdflatex-only
make clean        # keep PDF
make distclean    # remove PDF too
```

Overleaf: upload `main.tex`, `references.bib`, and compile with pdfLaTeX.

## Draft status (short)

- **Done:** full draft body; TikZ overview figure; v0.6 evidence table; BibTeX; build path.
- **Front-matter:** author/ORCID/Umeå/funding/COI/AI/CRediT reused from VariantFlow Application Note (see `HANDOFF.md`) — scientific content stays FastaGuard-only.
- **Still TBD:** FastaGuard Zenodo/preprint DOI, OUP template on revision, ScholarOne submit.
- **Framing rule:** never pitch as “FastQC for FASTA.”

Durable venue + project context for multi-agent handoff live in the Project store:

- `docs/publication-venues.md`
- `docs/project-context.md`
