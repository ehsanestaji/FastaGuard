# FastaGuard v0.3 Evidence Pack

This page records the evidence workflow for FastaGuard v0.3. The purpose is to
make the assembly gate inspectable before expanding into broader biological
profiles.

FastaGuard is FASTA preflight QC. It is not biological completeness analysis,
not assembly correctness analysis, and not contamination confirmation. Passing
the v0.3 gate means the FASTA-level contract is sane enough to continue into
downstream tools such as QUAST, BUSCO, BlobToolKit, CheckM, seqkit, or
annotation.

## Local Evidence Run

Build the release binary:

```bash
cargo build --release --locked
```

Run the CI-safe local evidence path:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3-local \
  --local-only
```

Local-only mode does not require network access or the NCBI Datasets CLI. It
runs:

- a deterministic synthetic FASTA
- `testdata/problem_assembly.fa`
- a gzipped copy of `testdata/valid_assembly.fa`

The evidence command runs FastaGuard with `--profile assembly --gate pipeline`
and keeps `--min-contig-length 1` so tiny local fixtures remain useful for
contract testing.

## Public NCBI Evidence Run

Install the NCBI Datasets CLI, then run:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3
```

The public workflow downloads genomic FASTA packages with commands shaped like:

```bash
datasets download genome accession GCF_000005845.2 --include genome --filename target/evidence/v0.3/ecoli_k12_mg1655/ncbi_dataset.zip
```

If `datasets` is not installed, use `--local-only` for offline smoke tests. The
default public manifest is:

```text
docs/evidence/public_assemblies.json
```

It currently includes:

- `GCF_000005845.2`: Escherichia coli K-12 MG1655
- `GCF_000182925.2`: Neurospora crassa OR74A

## Outputs

Each case writes FastaGuard artifacts under the selected output directory, for
example `target/evidence/v0.3-local/<case>/` or `target/evidence/v0.3/<case>/`:

- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_report.html`
- `fastaguard_mqc.json`

The workflow also writes compact summaries:

- `evidence_summary.json`
- `evidence_summary.tsv`

The summaries include verdict, gate status, blocking findings, top findings,
runtime, input size, sequence counts, and `input_sha256`. Commit compact
summaries when useful. Do not commit downloaded FASTA files, NCBI zip archives,
or full generated per-case reports.

## Interpretation

The v0.3 gate means the FASTA-level contract is sane enough to continue through
the pipeline. It checks validity, duplicate identifiers, invalid characters,
composition red flags, gap signals, and related FASTA-level evidence.

The gate does not prove biological completeness, does not prove assembly
correctness, and does not rule out contamination. Use QUAST, BUSCO,
BlobToolKit, CheckM, sourmash, Kraken, or other tools for deeper biological
interpretation after FastaGuard has checked the FASTA-level contract.

## References

- [NCBI Datasets genome download reference](https://www.ncbi.nlm.nih.gov/datasets/docs/v2/reference-docs/command-line/datasets/download/genome/)
- [NCBI Datasets genome download guide](https://www.ncbi.nlm.nih.gov/datasets/docs/v2/how-tos/genomes/download-genome/)
- [Neurospora crassa OR74A BioProject](https://www.ncbi.nlm.nih.gov/bioproject/132)
