# Bioconda Recipe

Upstream Bioconda currently publishes FastaGuard v1.0.0, updated in
[PR #68744](https://github.com/bioconda/bioconda-recipes/pull/68744).

The recipe has been merged into `bioconda/bioconda-recipes` as
`recipes/fastaguard/`, and the current published package is available from
Bioconda:

```bash
mamba install -c conda-forge -c bioconda fastaguard=1.0.0
```

BioContainers publishes the pinned workflow image:

```bash
docker pull quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0
```

This local recipe directory mirrors the FastaGuard v1.1.0 Bioconda recipe
submitted in [PR #70692](https://github.com/bioconda/bioconda-recipes/pull/70692).
The v1.1.0 GitHub source archive is published and `meta.yaml` includes the
verified archive SHA256.

## Local Checks

From a clone of `bioconda-recipes`, copy this directory to
`recipes/fastaguard/`, then run the standard Bioconda recipe lint/build
workflow.

Minimum contract checks in the recipe:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

The recipe uses `cargo-bundle-licenses` so Rust dependency licenses can be
included in the package as `THIRDPARTY.yml`.
