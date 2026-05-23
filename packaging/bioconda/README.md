# Bioconda Recipe

This directory mirrors the upstream Bioconda recipe for FastaGuard v0.1.1.

The recipe has been merged into `bioconda/bioconda-recipes` as
`recipes/fastaguard/`, and the package is available from Bioconda:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

BioContainers image/tag availability should be confirmed separately after
Bioconda publication.

## Local Checks

From a clone of `bioconda-recipes`, copy this directory to
`recipes/fastaguard/` when preparing an update, then run the standard Bioconda
recipe lint/build workflow.

Minimum contract checks in the recipe:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

The recipe uses `cargo-bundle-licenses` so Rust dependency licenses can be
included in the package as `THIRDPARTY.yml`.
