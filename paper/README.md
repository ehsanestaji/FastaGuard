# FastaGuard paper (LaTeX)

Draft manuscript targeting **Bioinformatics (Oxford Academic) — Application Note**.

This folder is paper preparation only. It does not change the FastaGuard product.

## Target venue (primary)

| Item | Choice |
| --- | --- |
| Journal | [*Bioinformatics*](https://academic.oup.com/bioinformatics) (OUP) |
| Article type | Application Note |
| Length budget | Up to ~4 pages (~2,600 words, or ~2,000 words + one figure) |
| Abstract shape | Summary / Availability and Implementation / Contact / Supplementary information |
| Software rules | Freely available to non-commercial users; state availability + implementation; keep available ≥2 years after publication |

Format-free initial submission is allowed. For revision / camera-ready, switch to the [OUP General Template](https://www.overleaf.com/latex/templates/oup-general-template/fqkhysbcbpwv) with Bioinformatics options (`numsec,webpdf,modern,large`) and author-date bibliography style as documented by OUP.

Venue ranking and framing notes for authors live in the Project store doc `docs/publication-venues.md` (not in this repo path).

## Layout of this folder

```text
paper/
  main.tex           Application Note skeleton (claims + section outline)
  references.bib     BibTeX database
  Makefile           latexmk / pdflatex build
  README.md          this file
  figures/           figure placeholders (add PDF/PNG before submission)
  tables/            optional table source / CSV exports
```

## Build the PDF

Requires a standard TeX Live (or MacTeX / MiKTeX) install with:

- `pdflatex`
- `bibtex` (or `biber` if you later switch the bibliography backend)
- `latexmk` (recommended)
- packages used by `main.tex`: `geometry`, `times`, `graphicx`, `booktabs`, `hyperref`, `xcolor`, `natbib`, `setspace`, `lineno`, `url`

### Quick build

```bash
cd paper
make
```

Output: `paper/main.pdf`.

### Without latexmk

```bash
cd paper
make pdflatex-only
```

### Clean aux files

```bash
make clean        # keep PDF
make distclean    # remove PDF too
```

### Overleaf

Upload `main.tex`, `references.bib`, and `figures/` to a new Overleaf project. Set the compiler to pdfLaTeX. For final OUP formatting, replace the document class with the OUP General Template as linked above.

## Word / page budget tips

- Keep the body short: Motivation → Implementation → Results/use → brief Discussion.
- Prefer **one** main figure (pipeline placement or evidence summary).
- Put finding catalogs, long tables, and extra screenshots in Supplementary Information.
- Count words before submission; manuscripts that overrun the Application Note limit by a large margin are often returned without review.

## Before submission checklist

- [ ] Replace TBD emails, affiliation, funding, and benchmark table cells
- [ ] Drop real figure(s) into `figures/` and uncomment `\includegraphics`
- [ ] Freeze software version + DOI/archive (GitHub release / Zenodo) in Availability
- [ ] Align claims with `docs/product-thesis.md` and `docs/tool-landscape.md` (no “FastQC for FASTA”)
- [ ] Verify Bioconda / BioContainers / module links still resolve
- [ ] Optionally deposit a preprint (bioRxiv / arXiv q-bio) — OUP is preprint-friendly with update-on-accept obligations
