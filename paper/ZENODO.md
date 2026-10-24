# Zenodo archive steps (USER-ACTION)

No Zenodo DOI is invented in the manuscript. GitHub release **`v1.0.0`** already
exists and is the cited Availability / Bioconda line
(<https://github.com/ehsanestaji/FastaGuard/releases/tag/v1.0.0>).

## Mint a DOI from the existing release

1. Sign in to [Zenodo](https://zenodo.org/) with an account that can access the
   `ehsanestaji/FastaGuard` GitHub repository (enable the Zenodo GitHub
   integration under Zenodo → GitHub if needed).
2. Enable the **FastaGuard** repository in Zenodo’s GitHub listing.
3. Prefer depositing the **existing** `v1.0.0` release (do not invent a new
   version number for the paper). Options:
   - If Zenodo is linked: create a *new* GitHub release only if you intentionally
     want a fresh archive tag; otherwise use Zenodo’s UI to refresh/archive the
     `v1.0.0` release asset / tag already published.
   - Or upload the `v1.0.0` source tarball from GitHub releases manually and
     apply metadata from repo-root [`.zenodo.json`](../.zenodo.json).
4. Publish the Zenodo record. Copy:
   - **Concept DOI** (optional, always resolves to latest)
   - **Version DOI** for `v1.0.0` (preferred for Availability)
5. Paste into:
   - `paper/submission.yml` → `release.archive_doi` / `release.version_doi`
   - `paper/main.tex` Availability sentence (add
     `\href{https://doi.org/10.5281/zenodo.XXXX}{doi:10.5281/zenodo.XXXX}`)
6. Rebuild: `cd paper && make && make check`

## Do not

- Invent or placeholder a `10.5281/zenodo.…` DOI in the submit-ready PDF.
- Retag a different software version unless the manuscript Availability is
  updated to match.
