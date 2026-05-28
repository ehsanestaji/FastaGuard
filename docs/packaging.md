# Packaging

## Recommendation

Treat packaging as part of the product, not as a later chore. For bioinformatics adoption, the order should be:

```text
Bioconda -> BioContainers -> GitHub release binaries -> Docker image -> Homebrew later
```

FastaGuard v0.3.0 is published on GitHub with Linux and macOS release binaries.
Bioconda serves v0.3.0 on Linux and macOS x86_64/ARM64 platforms.
BioContainers provides the pinned v0.3 workflow image generated from the
Bioconda package. Docker remains useful for local smoke tests.

## Bioconda

Recommended install:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.3.0
```

Conda equivalent:

```bash
conda install -c conda-forge -c bioconda fastaguard=0.3.0
```

Verify the installed package:

```bash
fastaguard --version
fastaguard --schema
fastaguard --finding-catalog
```

Current published package:

- Version: `0.3.0`
- Platforms: `linux-64`, `linux-aarch64`, `osx-64`, `osx-arm64`
- Package page: [anaconda.org/bioconda/fastaguard](https://anaconda.org/bioconda/fastaguard)

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

For a public release:

1. Tag the release:

   ```bash
   git tag v0.3.0
   git push origin v0.3.0
   ```

2. Push the tag to trigger `.github/workflows/release.yml`.
3. Build release binaries for Linux x86_64, macOS Intel, and macOS Apple Silicon.
4. Attach `SHA256SUMS` and release archives to the GitHub release.
5. Keep the JSON Schema and finding catalog in the source archive and binary archives.

## Upstream Recipe

The upstream Bioconda recipe was merged from `packaging/bioconda/` as
`recipes/fastaguard/`. The recipe exposes one executable:

```text
fastaguard
```

Recommended recipe checks:

```bash
fastaguard --help
fastaguard --schema
fastaguard --finding-catalog
```

Keep future releases compatible with Bioconda expectations:

- keep a single static-ish CLI binary target
- keep deterministic tests and tiny fixtures
- avoid runtime databases for early releases
- maintain stable exit codes
- maintain a versioned JSON Schema

Bioconda recipe guidance checked for this setup:

- Bioconda hosts bioinformatics-specific packages.
- Rust dependencies should have license metadata bundled, so the starter recipe uses `cargo-bundle-licenses`.
- Tests in `meta.yaml` must rely only on runtime dependencies, so the starter tests use FastaGuard contract discovery commands.

## Container Strategy

The Docker image should stay boring:

- no bundled reference databases
- no background services
- no network requirement at runtime
- one entrypoint: `fastaguard`

That makes it easy to run in Nextflow, Snakemake, Galaxy, and CI systems.

The Bioconda recipe has merged upstream and generated a BioContainers image.
Use the pinned tag in workflow examples:

```bash
docker pull quay.io/biocontainers/fastaguard:0.3.0--hfa8f182_0
```

That path is preferable to maintaining a separate BioContainers Dockerfile.

## MultiQC

FastaGuard emits MultiQC custom content as `fastaguard_mqc.json`.

A native MultiQC plugin starter now lives in:

```text
integrations/multiqc/
```

Local development:

```bash
cd integrations/multiqc
python -m pip install -e .
cd ../../examples/reports
multiqc .
```

This is intentionally compact: it parses `fastaguard_mqc.json`, adds key metrics to MultiQC general stats, and adds a FastaGuard summary section. The full evidence remains in FastaGuard's own HTML and JSON reports.
