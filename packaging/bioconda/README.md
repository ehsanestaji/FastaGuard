# Bioconda Recipe Starter

This directory is a staging recipe for upstream Bioconda submission.

Bioconda should receive the recipe after the source archive is public and the
`sha256` value in `meta.yaml` has been replaced with the source archive hash.
Once the Bioconda PR is merged, BioContainers infrastructure can build the
corresponding container from the conda recipe.

## Local Checks

From a clone of `bioconda-recipes`, copy this directory to
`recipes/fastaguard/`, replace the placeholder SHA256, then run the standard
Bioconda recipe lint/build workflow.

Minimum contract checks in the recipe:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

The recipe uses `cargo-bundle-licenses` so Rust dependency licenses can be
included in the package as `THIRDPARTY.yml`.
