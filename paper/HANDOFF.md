# Paper track handoff (cold-start)

Paper-only track. Do not change product CLI/schema/packaging unless asked.
Venue: ***Bioinformatics* (OUP) Application Note** — **user-confirmed**.
PR: [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) · branch `cursor/paper-prep-venues-198e`.

## Machine-ready vs USER-ACTION

| Status | Items |
| --- | --- |
| **DONE / submit-package ready** | Full draft; front-matter; evidence freeze; `submission.yml`; `check_submission.py`; cover letter; `.zenodo.json`; bioRxiv build target; `docs/paper-readiness.md` |
| **DONE (2026-10-24)** | Zenodo DOI minted and pasted into Availability + manifest; OUP guideline review (AI + preprint disclosure added to cover letter); external claims verified (see below); PDF metadata + Fig. 1 label overlap fixed |
| **USER-ACTION only** | Co-author final OK (Jian-Feng Mao); bioRxiv deposit; ScholarOne upload; APC/R&P/ISCB |

Do **not** claim bioRxiv or ScholarOne submitted from this agent track until the
confirmation page / DOI is seen. Never click final Submit without explicit user OK.
Order: co-author OK -> bioRxiv -> ScholarOne (cover letter says the preprint was submitted).

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

## Verified 2026-10-24 (do not re-derive)

- `fastaguard=1.0.0` is Bioconda latest; BioContainers `quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0` exists.
- nf-core module `modules/nf-core/fastaguard` and `snakemake-wrappers/bio/fastaguard` exist upstream.
- Zenodo DOI resolves (HTTP 200 -> zenodo.org/records/23622311).
- Body ~2,160 words excl. references (OUP limit ~2,600 words, or ~2,000 + one figure).
- GitHub "Latest" is v1.1.0 but the paper intentionally cites v1.0.0 (Bioconda latest).

## Pre-submission review (2026-10-24) - applied

Two independent reviews (claims vs a binary built from `git archive v1.0.0`; related work and
references) plus an editorial pass. Applied to `main.tex` / `references.bib`:

- Table 1 caption now states runs used `--min-contig-length 1` (default => synthetic fixture WARNs on
  `tiny_contigs`; verified), the N. crassa count includes mitochondrion NC_026614.1 (64,840 bp; verified
  against NCBI Datasets), and the v1.0.0 binary reproduces both local fixtures (verdict, gate, SHA-256, exit 0).
- Removed the unsupported "value-benchmark" sentence (source was v0.3.0 in `docs/value-benchmark.md`, cited to v1.0.0).
- Gate text lists first-token duplicate IDs; binaries claim fixed (no Linux aarch64 GitHub binary; Bioconda has it);
  "deterministic bundle" narrowed (timestamps/durations vary); NCBI policy described as conservative (stricter than
  table2asn on contig_199 / seqid_50 boundary cases) with the NCBI FASTA-format citation.
- Related work added (py_fasta_validator, fa-lint, AssemblyQC, SeqFu, gfastats, SeqKit2) with hedged "to our
  knowledge" wording; missing citations added (CheckM, Nextflow, Snakemake, nf-core, Bioconda, BioContainers, FCS-GX,
  RefSeq and genome papers). All DOIs checked against Crossref/DataCite; 24 bib entries, 0 BibTeX warnings.
- Removed: "fast", "product thesis", "future tool-using agents", "falsifiable", SNIC/HPC2N acknowledgement (all
  reported runs were macOS arm64), and the `paper/` supplementary pointer (not in the v1.0.0 tree).
- Zenodo record 23622311 creators fixed on zenodo.org (Estaji + Mao, ORCIDs, UPSC affiliation); DOI unchanged.
- PDF: `--flag` ligature fixed (microtype `\DisableLigatures`), PDF metadata filled, Fig. 1 label overlap fixed.

Open items (user decisions, NOT done): AI-disclosure wording vs OUP rule; funding line confirmation;
restore SNIC ack only if HPC2N was really used; optional feature-matrix + differential test vs
py_fasta_validator/fa-lint/table2asn (referee will likely ask); optional re-run of the two NCBI rows with v1.0.0;
`CITATION.cff` lists Estaji only (product repo); Zenodo description still says Bioconda "remains at v0.7.0".
Length: body ~2,270 words (limit ~2,600) but 24 references push the draft PDF to 7 pages.

## OUP policy points that shaped the cover letter

- Preprints allowed; must be disclosed in the cover letter; update with published DOI after acceptance;
  do not submit the preprint to more than one journal at once.
- AI use must be disclosed in the cover letter and in the manuscript. OUP lists "drafting papers from a
  prompt" as unacceptable; the disclosure must stay an accurate description of what was done.
- Software must be archived (Zenodo) with the URL in the Availability abstract, and stay available 2 years.
- ScholarOne asks for ORCID, COI, CRediT roles; reviewer suggestions optional.

## Zenodo DOI (2026-10-24)

- Version DOI: `10.5281/zenodo.23622311` (https://zenodo.org/records/23622311)
- Concept DOI: `10.5281/zenodo.23622310`
- Source: GitHub release `v1.0.0` via Zenodo GitHub integration
