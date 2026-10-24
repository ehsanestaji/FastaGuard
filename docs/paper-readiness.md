# FastaGuard paper readiness (*Bioinformatics* Application Note)

Venue: **Bioinformatics (OUP) — Application Note** (user-confirmed).  
Branch/PR: `cursor/paper-prep-venues-198e` / [#21](https://github.com/ehsanestaji/FastaGuard/pull/21).  
Control manifest: [`paper/submission.yml`](../paper/submission.yml).  
Machine check: `cd paper && make check`.

## DONE (machine / repo complete)

| Item | Status |
| --- | --- |
| Full Application Note draft (`paper/main.tex`) | DONE |
| Structured abstract (Summary / Availability / Contact / SI) | DONE |
| Authors, ORCID, Umeå affiliation, emails (from VariantFlow front-matter) | DONE |
| Funding (WIFORCE/KAW), COI, AI disclosure, CRediT, acknowledgements | DONE |
| Evidence freeze: Table 1 numbers match `docs/evidence/results/v0.6/evidence_summary.tsv` | DONE |
| Cited software release for Availability: GitHub **`v1.0.0`** (Bioconda-published) | DONE |
| Forbidden phrase guard (“FastQC for FASTA”) in `check_submission.py` | DONE |
| `.zenodo.json` metadata prepared (no fake DOI) | DONE |
| Zenodo DOI minted from `v1.0.0` (`10.5281/zenodo.23622311`) and cited in Availability + manifest | DONE |
| Cover letter draft (`paper/cover-letter.md`) incl. preprint + AI disclosure + archive DOI (OUP policy) | DONE |
| Upstream claims verified (Bioconda, BioContainers, nf-core module, Snakemake wrapper) | DONE |
| Pre-submission review: claims re-tested against a v1.0.0 build; related work + references audited; fixes applied (see `paper/HANDOFF.md`) | DONE |
| Zenodo record creators corrected (Estaji + Mao, ORCIDs, affiliation) | DONE |
| bioRxiv build target + checklist | DONE |
| PDF builds (`make`) | DONE |
| Word count ~2.0–2.6k band | DONE (~2.0k+) |

## USER-ACTION (requires Ehsan / credentials / co-author)

| Item | Notes |
| --- | --- |
| Co-author final OK (Jian-Feng Mao) | Confirm authorship/order/CRediT |
| Review decisions | AI-disclosure wording, funding line, SNIC ack, optional feature matrix / differential test, optional NCBI re-run at v1.0.0 (see `paper/HANDOFF.md`) |
| bioRxiv deposit | Follow [`paper/biorxiv-checklist.md`](../paper/biorxiv-checklist.md); all authors must consent; cover letter discloses it |
| ScholarOne upload | Format-free first submission OK; upload PDF + cover letter; OUP General Template deferred to **revision** |
| APC / Read & Publish / ISCB discount | Check before acceptance |
| Suggested editors (optional) | Cover letter suggests assembly/sequence-analysis tools area |

## Pre-submit command checklist

```bash
cd paper
make check    # must exit 0
make          # builds main.pdf
make biorxiv  # optional preprint PDF copy
```

## Evidence vs software versions

- **Availability / packaging cite:** FastaGuard **`v1.0.0`**
  (<https://github.com/ehsanestaji/FastaGuard/releases/tag/v1.0.0>).
- **Table 1 evidence freeze:** committed **v0.6.0** public-evidence pack
  (`docs/evidence/…`); timings are contextual contract illustrations, not a
  claim that v1.0.0 was re-benchmarked on that host.
- Do **not** invent a new tag solely for the paper; `v1.0.0` already exists.
  Latest GitHub “Latest” may be newer (e.g. v1.1.0) — paper cites v1.0.0 as the
  Bioconda reference-contract release unless Availability is intentionally retargeted.
