# Packaging

## Recommendation

Treat packaging as part of the product, not as a later chore. For bioinformatics adoption, the order should be:

```text
GitHub release binaries -> Docker image -> Bioconda -> Homebrew later
```

The repository currently supports local release builds and Docker builds. Bioconda should be prepared once the first public tag exists.

## Local Binary

Build the optimized CLI:

```bash
cargo build --release --locked
```

Run it:

```bash
./target/release/fastaguard testdata/valid_assembly.fa \
  --profile assembly \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_mqc.json
```

## Docker

Build the image:

```bash
docker build -t fastaguard:local .
```

Run against a FASTA in the current directory:

```bash
docker run --rm \
  -v "$PWD:/data" \
  fastaguard:local \
  /data/sample.fa \
  --profile assembly \
  --out /data/fastaguard_report.html \
  --json /data/fastaguard.json \
  --tsv /data/fastaguard.tsv \
  --multiqc /data/fastaguard_mqc.json
```

## GitHub Release Binaries

For the first public release:

1. Tag the release:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. Push the tag to trigger `.github/workflows/release.yml`.
3. Build release binaries for Linux x86_64, macOS Intel, and macOS Apple Silicon.
4. Attach `SHA256SUMS` and release archives to the GitHub release.
5. Keep the JSON Schema and finding catalog in the source archive and binary archives.

## Bioconda

Bioconda should be added after the first public source archive is available. The recipe should expose one executable:

```text
fastaguard
```

Recommended recipe checks:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

Do not block the MVP on Bioconda, but design for it now:

- keep a single static-ish CLI binary target
- keep deterministic tests and tiny fixtures
- avoid runtime databases for v0.1
- maintain stable exit codes
- maintain a versioned JSON Schema

## Container Strategy

The Docker image should stay boring:

- no bundled reference databases
- no background services
- no network requirement at runtime
- one entrypoint: `fastaguard`

That makes it easy to run in Nextflow, Snakemake, Galaxy, and CI systems.
