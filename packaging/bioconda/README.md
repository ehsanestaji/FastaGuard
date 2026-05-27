# Bioconda Recipe

Upstream Bioconda currently publishes FastaGuard v0.2.0.

The recipe has been merged into `bioconda/bioconda-recipes` as
`recipes/fastaguard/`, and the current published package is available from
Bioconda:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

BioContainers publishes the pinned workflow image:

```bash
docker pull quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0
```

This local recipe directory mirrors the FastaGuard v0.2.0 Bioconda recipe. The
v0.2.0 GitHub source archive is published and `meta.yaml` includes the real
archive SHA256.

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
