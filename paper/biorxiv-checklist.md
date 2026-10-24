# bioRxiv upload checklist (USER-ACTION, optional)

Prepare a preprint **before or with** ScholarOne submission. OUP allows
preprints; update the preprint with the published DOI after acceptance.

## Build the bioRxiv-ready PDF

```bash
cd paper
make biorxiv
# → paper/build/fastaguard-biorxiv.pdf
```

This is the same Application Note PDF with a stable filename for upload.

## Upload steps

1. Create/sign in to a bioRxiv account.
2. Start a new preprint in **Bioinformatics** / computational biology category
   (choose the closest bioRxiv subject collection).
3. Upload `paper/build/fastaguard-biorxiv.pdf`.
4. Title: `FastaGuard: FASTA preflight QC for modern bioinformatics pipelines`
5. Authors / affiliations / ORCIDs as in `paper/main.tex`.
6. Abstract: paste the structured Summary paragraph (bioRxiv may want a single
   abstract block — use the Summary text; keep Availability as a separate
   statement in the PDF).
7. Confirm license compatible with later OUP CC BY publication.
8. After posting, record the bioRxiv DOI in `paper/submission.yml` notes and
   optionally in Acknowledgements / Data availability.

## Do not

- Claim the preprint is posted until the bioRxiv confirmation page shows a DOI.
- Put a fake bioRxiv DOI in `main.tex`.
