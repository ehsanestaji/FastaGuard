# Paper track handoff (cold-start)

For any agent or human resuming FastaGuard **paper preparation** after a pause.
This track is **paper-only**. Do not change product CLI/schema/packaging unless
the user explicitly expands scope. Product/dev work may be on a separate track
— do not conflict with it.

## Locked decisions

| Item | Value |
| --- | --- |
| Venue | ***Bioinformatics* (OUP) — Application Note** (**user-confirmed**) |
| Working title | `FastaGuard: FASTA preflight QC for modern bioinformatics pipelines` |
| Framing | FASTA **preflight** contract before QUAST/BUSCO/BlobToolKit/etc. |
| Forbidden phrase | Do **not** use “FastQC for FASTA” |
| Repo branch / PR | `cursor/paper-prep-venues-198e` / [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) |
| LaTeX root | `paper/main.tex` |
| Author / funding template source | [VariantFlow](https://github.com/ehsanestaji/VariantFlow) `paper/bioinformatics-application-note/` (**user-directed reuse**) |

## What was copied from VariantFlow (front-matter only)

Reuse these from VariantFlow’s Application Note — **not** its VCF science:

- Author list: Ehsan Estaji + Jian-Feng Mao, ORCIDs, Umeå Plant Science Centre affiliation
- Correspondence emails: `ehsan.estaji@umu.se`, `jianfeng.mao@umu.se`
- CRediT **Author contributions** wording pattern
- **Funding:** WIFORCE / Knut and Alice Wallenberg Foundation
- **Conflict of interest:** no competing interests
- **AI usage disclosure** pattern (adapted to FastaGuard / Cursor agents)
- **Acknowledgements:** SNIC/HPC2N at Umeå University (+ FastaGuard packaging thanks)
- Sectioning: Introduction / Implementation / Results / Discussion + end-matter blocks
- LaTeX front-matter conventions (margin, ORCID hyperlinks, `apalike`, keywords line)

## What must stay FastaGuard-specific

- All scientific claims, comparison framing, gates/contract description
- Evidence table and timings from `docs/evidence/` (v0.6 public pack) — **never** VariantFlow VCF benchmarks/tables
- Availability URLs, Bioconda/BioContainers/nf-core/Snakemake packaging story
- TikZ preflight figure and FastaGuard `references.bib` (QUAST/BUSCO/etc.)

## What is done

- Full Application Note draft with VariantFlow-aligned author/funding/disclosures
- Structured Bioinformatics Application Note abstract (Summary / Availability / Contact / SI)
- TikZ overview figure; v0.6 evidence table
- ~2.0k words (within ~2.0–2.6k Application Note target); PDF builds via `make`
- Store context + venue brief updated

## Draft vs final

| Piece | Status |
| --- | --- |
| Author / ORCID / affiliation / emails | **Filled from VariantFlow** (same author block by user default) |
| Funding / COI / AI disclosure / CRediT | **Filled** (VariantFlow patterns) |
| Scientific body + evidence | **FastaGuard draft-complete** |
| Word count | ~2.0k — OK; trim if journal typesetting overruns |
| Figure | Usable TikZ |
| Table | v0.6 evidence (contextual timings) |
| Zenodo / preprint DOI | **Not yet** (VariantFlow had these; add for FastaGuard when archived) |
| OUP camera-ready template | **Not yet** — format-free OK for first submission |
| ScholarOne submit | **Not yet** |

## How to build

```bash
cd paper
make          # → main.pdf
make clean
```

Requires TeX Live with `pdflatex`, `bibtex`, `latexmk`, TikZ (`texlive-pictures`), `microtype`.

## Store docs

- `/cursor/stores/bc-53e26eb2-1f92-41f8-9fa2-384c56a6ad53/docs/publication-venues.md`
- `/cursor/stores/bc-53e26eb2-1f92-41f8-9fa2-384c56a6ad53/docs/project-context.md`

## Next concrete steps

1. Confirm Jian-Feng Mao co-authorship for FastaGuard is intentional (defaulted yes per user).
2. Optional: archive a FastaGuard release on Zenodo and add DOI to Availability.
3. Optional preprint (bioRxiv); recount words after any edits.
4. Submit format-free PDF; on revision use OUP General Template.
5. Confirm APC / Read & Publish / ISCB discount before acceptance.

## Key in-repo sources for claims

- `README.md`, `docs/product-thesis.md`, `docs/tool-landscape.md`, `docs/output-contract.md`
- `docs/evidence/fastaguard-v0.6-public-evidence.md` + `docs/evidence/results/v0.6/`
- VariantFlow front-matter only: `https://github.com/ehsanestaji/VariantFlow/tree/main/paper/bioinformatics-application-note`
