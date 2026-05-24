# Bioconda Recipe

Upstream Bioconda currently publishes FastaGuard v0.1.1.

The recipe has been merged into `bioconda/bioconda-recipes` as
`recipes/fastaguard/`, and the current published package is available from
Bioconda:

```bash
mamba install -c conda-forge -c bioconda fastaguard
```

BioContainers image/tag availability should be confirmed separately after
Bioconda publication.

This local recipe directory is staged for FastaGuard v0.2.0. Do not submit it
to Bioconda or copy it into `bioconda-recipes` until the v0.2.0 GitHub source
archive is published and the placeholder `sha256` in `meta.yaml` is replaced
with the real archive SHA256.

## Local Checks

From a clone of `bioconda-recipes`, copy this directory to
`recipes/fastaguard/` only after the v0.2.0 source archive SHA is filled in,
then run the standard Bioconda recipe lint/build workflow.

Minimum contract checks in the recipe:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

The recipe uses `cargo-bundle-licenses` so Rust dependency licenses can be
included in the package as `THIRDPARTY.yml`.
