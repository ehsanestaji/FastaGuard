# FastaGuard v0.2 Evidence Pack

This page records the evidence workflow for FastaGuard v0.2. It is intended to
make the preflight claim inspectable before adding new biological profiles.

FastaGuard is a FASTA preflight tool. It is not biological completeness
analysis, not assembly correctness analysis, and not contamination confirmation.
Passing FastaGuard means the FASTA-level contract is sane enough to route into
downstream tools such as QUAST, BUSCO, BlobToolKit, CheckM, or annotation.

## Local Evidence Run

Build the release binary:

```bash
cargo build --release --locked
```

Run the CI-safe local evidence path:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/local-smoke \
  --local-only
```

Local-only mode does not require network access or the NCBI Datasets CLI. It
runs:

- a deterministic synthetic FASTA
- `testdata/problem_assembly.fa`
- a gzipped copy of `testdata/valid_assembly.fa`

## Public NCBI Evidence Run

Install the NCBI Datasets CLI, then run:

```bash
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.2
```

The public workflow downloads genomic FASTA packages with commands shaped like:

```bash
datasets download genome accession GCF_000005845.2 --include genome --filename target/evidence/v0.2/ecoli_k12_mg1655/ncbi_dataset.zip
```

If `datasets` is not installed, the script exits before running public cases.
Use `--local-only` for offline smoke tests.

The default public manifest is:

```text
docs/evidence/public_assemblies.json
```

It currently includes:

- `GCF_000005845.2`: Escherichia coli K-12 MG1655
- `GCF_000182925.2`: Neurospora crassa OR74A

## Outputs

Each case writes FastaGuard artifacts under `target/evidence/<case>/`:

- `fastaguard.json`
- `fastaguard.tsv`
- `fastaguard_report.html`
- `fastaguard_mqc.json`

The workflow also writes compact summaries:

- `evidence_summary.json`
- `evidence_summary.tsv`

The summary records the command used, FastaGuard version, git commit, platform,
date, input size, sequence count, elapsed seconds, verdict, and top findings.
Commit the evidence page and compact summaries when useful. Do not commit
downloaded FASTA files, NCBI zip archives, or full generated reports.

## Interpretation

Use this evidence to answer practical adoption questions:

- how quickly does FastaGuard produce a preflight report?
- does it catch duplicate IDs, invalid symbols, high-N records, and composition outliers?
- does it produce JSON, TSV, HTML, and MultiQC-ready output before heavier tools run?

Use QUAST, BUSCO, BlobToolKit, CheckM, sourmash, Kraken, or other tools for
deeper biological interpretation after FastaGuard has checked the FASTA-level
contract.

## References

- [NCBI Datasets genome download reference](https://www.ncbi.nlm.nih.gov/datasets/docs/v2/reference-docs/command-line/datasets/download/genome/)
- [NCBI Datasets genome download guide](https://www.ncbi.nlm.nih.gov/datasets/docs/v2/how-tos/genomes/download-genome/)
- [Neurospora crassa OR74A BioProject](https://www.ncbi.nlm.nih.gov/bioproject/132)
