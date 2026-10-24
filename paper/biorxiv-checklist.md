# bioRxiv upload checklist (USER-ACTION)

Deposit the preprint **before** the ScholarOne submission: the cover letter
states that the same manuscript has been submitted to bioRxiv (OUP requires
preprint disclosure in the cover letter). OUP allows preprints; update the
preprint with the published DOI after acceptance.

## Build the bioRxiv-ready PDF

```bash
cd paper
make biorxiv
# -> paper/build/fastaguard-biorxiv.pdf
```

This is the same Application Note PDF with a stable filename for upload.
bioRxiv takes a single PDF with text, figures, and tables; no separate figure
files are needed.

## Gate before starting

- Jian-Feng Mao has confirmed in writing that he consents to deposition and
  submission (bioRxiv requires all authors' consent).
- Ehsan is logged in to bioRxiv himself (registration needs an affiliation:
  Umeå Plant Science Centre, Umeå University). Agents never enter passwords or
  create accounts.

## Fields (paste-ready)

| Field | Value |
| --- | --- |
| Title | `FastaGuard: FASTA preflight QC for modern bioinformatics pipelines` |
| Authors | Ehsan Estaji (ORCID 0009-0005-0838-228X); Jian-Feng Mao (ORCID 0000-0001-9735-8516) |
| Affiliation | Umeå Plant Science Centre, Department of Plant Physiology, Umeå University, SE-901 87 Umeå, Sweden |
| Corresponding | ehsan.estaji@umu.se, jianfeng.mao@umu.se |
| Article type | New Results |
| Subject area | Bioinformatics (confirm the name in bioRxiv's dropdown) |
| Abstract | Summary paragraph from `main.tex` (single block); Availability stays in the PDF |
| License | CC BY (recommended: matches the open-access CC BY licence OUP offers) |
| Funding | WIFORCE (Wallenberg Initiatives in Forest Research), Knut and Alice Wallenberg Foundation |
| Competing interests | None |
| Code / data | https://github.com/ehsanestaji/FastaGuard ; Zenodo doi:10.5281/zenodo.23622311 |

## Upload steps

1. Start a new submission, upload `paper/build/fastaguard-biorxiv.pdf`.
2. Fill the fields above; review the PDF as bioRxiv renders it.
3. Stop at the final confirm screen and get Ehsan's explicit OK before clicking
   it: a posted preprint is citable and cannot be removed.
4. After bioRxiv screening, record the bioRxiv DOI in `paper/submission.yml`
   (`biorxiv_status`) and add it to the ScholarOne record / revision.

## Do not

- Claim the preprint is posted until the bioRxiv confirmation shows a DOI.
- Put a fake bioRxiv DOI in `main.tex`.
- Submit the preprint to more than one journal at once (OUP policy).
