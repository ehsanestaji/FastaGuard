# Paper track handoff (cold-start)

Paper-only track. Do not change product CLI/schema/packaging unless asked.
Venue: ***Bioinformatics* (OUP) Application Note** — **user-confirmed**.
PR: [#21](https://github.com/ehsanestaji/FastaGuard/pull/21) · branch `cursor/paper-prep-venues-198e`.

## Machine-ready vs USER-ACTION

| Status | Items |
| --- | --- |
| **DONE / submit-package ready** | Full draft; front-matter; evidence freeze; `submission.yml`; `check_submission.py`; cover letter; `.zenodo.json`; bioRxiv build target; `docs/paper-readiness.md` |
| **DONE (2026-10-24)** | Zenodo DOI minted and pasted into Availability + manifest; OUP guideline review (AI + preprint disclosure added to cover letter); external claims verified (see below); PDF metadata + Fig. 1 label overlap fixed |
| **DONE (2026-10-24)** | bioRxiv preprint submitted as `BIORXIV/2026/761563` (acknowledgement email received); in screening, DOI pending |
| **USER-ACTION only** | Record the bioRxiv DOI when issued; ScholarOne upload; APC/R&P/ISCB; merge PR #21 |

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

Resolved after the user's answers (same day):
- AI disclosure rewritten (paper + cover letter): software, code base and benchmarking results came first; the draft
  was written solely from them, not from a free-form prompt; tools named (Cursor-based agents, Anthropic Claude Code).
- Funding unchanged: GraphPop, GraphMana and VariantFlow preprints carry the identical WIFORCE/KAW sentence.
- SNIC/HPC2N not used for FastaGuard: acknowledgement stays removed.
- NCBI rows re-run with a v1.0.0 build (clean clone at the tag + NCBI Datasets 18.39.0): all five evidence-pack rows
  match the frozen v0.6.0 values on verdict, gate, blocking findings, lengths, sequence counts, SHA-256, exit code
  (timings differ). Portable summary: `docs/evidence/results/v1.0.0-rerun/`. Binary was built from the v1.0.0 tag, not
  the release asset.
- Validator comparison added (Table 2, `paper/comparison/`): py_fasta_validator 0.6, fa-lint 1.2.0, table2asn 1.29.324,
  SeqKit 2.14.0, FastaGuard 1.0.0 on 30 constructed files (expectations fixed first) + the two RefSeq assemblies.
  Reproduce with `python3 -I paper/comparison/run_comparison.py --help`; tool SHA-256s in `tool_versions.json`.
  Author-constructed corpus favours FastaGuard; capability, not accuracy. A first real-data run was discarded (label
  collision + stale report read); the script was fixed and the 30-file results were byte-identical afterwards.

Co-author OK: reported by Ehsan in chat 2026-10-24 (no message from Jian-Feng seen by the agent).
Still open: merge PR #21 so `paper/comparison/` resolves on main (it is not in the v1.0.0
Zenodo archive; consider a small separate deposit for test data); `CITATION.cff` lists Estaji only (product repo); Zenodo
description still says Bioconda "remains at v0.7.0"; length (Introduction-Discussion ~2,260 words; ~2,700 incl.
abstract, captions and declarations; ~850 more in references; draft PDF is 8 pages).

## bioRxiv submission record (2026-10-24)

- ID `BIORXIV/2026/761563`; New Results; Bioinformatics; CC BY 4.0; funder Knut and Alice Wallenberg Foundation (no award number).
- The uploaded PDF is `paper/build/fastaguard-biorxiv.pdf` (SHA-256 `8eb58ec7...2269`); bioRxiv served it back byte-identical.
- Form notes: one corresponding author only (Ehsan); accented letters must be HTML entities (`Ume&aring;`); the abstract field takes the full
  abstract; the built-in browser has no file-upload tool, so the file is picked by the user in the native dialog.
- Next: when bioRxiv issues the DOI, record it in `paper/submission.yml`, the cover letter and the academic dashboard
  (`research-monitor`, entry SUB-03), then upload to ScholarOne.

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
