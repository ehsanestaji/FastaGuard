# Five-minute quickstart

This path uses the current published FastaGuard distribution, v0.7.0. The
v1.0.0 source release candidate prepares the Reference Contract Gate, but no
v1.0.0 GitHub release, Bioconda package, or BioContainers image is claimed here.

## 1. Install the published CLI

With Bioconda:

```bash
mamba create -n fastaguard -c conda-forge -c bioconda fastaguard=0.7.0
mamba activate fastaguard
fastaguard --version
```

The package record is at
[anaconda.org/bioconda/fastaguard](https://anaconda.org/bioconda/fastaguard).

## 2. Produce four local reports

From a directory containing `sample.fa`, run:

```bash
mkdir -p reports
fastaguard sample.fa \
  --profile assembly \
  --gate pipeline \
  --out reports/sample-01.fastaguard.html \
  --json reports/sample-01.fastaguard.json \
  --tsv reports/sample-01.fastaguard.tsv \
  --multiqc reports/sample-01.fastaguard_mqc.json
```

This creates:

- `sample-01.fastaguard.html` for local review
- `sample-01.fastaguard.json` as the complete machine-readable contract
- `sample-01.fastaguard.tsv` for flat tables and shell-oriented workflows
- `sample-01.fastaguard_mqc.json` for MultiQC custom-content discovery

MultiQC recognises filenames ending in `_mqc.json`; see its
[custom-content documentation](https://docs.seqera.io/multiqc/custom_content).

## 3. Interpret JSON and TSV downstream

Read overall status and the selected gate independently:

```bash
jq '{verdict: .verdict.status, can_continue: .gate.can_continue, blockers: .gate.blocking_findings}' \
  reports/sample-01.fastaguard.json
```

Continue only when the selected gate allows it:

```bash
if jq -e '.gate.can_continue == true' reports/sample-01.fastaguard.json >/dev/null; then
  run_downstream_qc sample.fa
fi
```

The single-file TSV is a two-column `metric`/`value` table, not a columnar
per-sample table. Names such as `input_path`, `verdict`, and `gate_status`
appear as values in the `metric` rows:

```bash
python3 -c 'import csv; rows = {row["metric"]: row["value"] for row in csv.DictReader(open("reports/sample-01.fastaguard.tsv"), delimiter="\t")}; print({key: rows[key] for key in ("input_path", "verdict", "gate_status", "gate_can_continue")})'
```

See [report interpretation](report-interpretation.md) before turning findings
into workflow policy.

## v1.0.0 source candidate: bundle mode

The shorter bundle command belongs to the current v1.0.0 source tree. It is not
evidence of a published v1.0.0 tag, package, or container:

```bash
cargo install --path . --locked
fastaguard sample.fa --outdir reports --prefix sample-01
```

It writes the same four names listed above. Existing outputs are protected
unless `--force` is supplied.

## Container alternatives for published v0.7.0

Docker:

```bash
docker run --rm -v "$PWD:/work" \
  quay.io/biocontainers/fastaguard:0.7.0--hfa8f182_0 \
  fastaguard /work/sample.fa --json /work/reports/sample-01.fastaguard.json
```

Apptainer can execute the same public OCI image from Quay:

```bash
apptainer exec --bind "$PWD:/work" \
  docker://quay.io/biocontainers/fastaguard:0.7.0--hfa8f182_0 \
  fastaguard /work/sample.fa --json /work/reports/sample-01.fastaguard.json
```

The exact tag is visible in the
[BioContainers registry](https://quay.io/repository/biocontainers/fastaguard?tab=tags).
Apptainer documents `docker://` images from Quay in its
[OCI container guide](https://apptainer.org/docs/user/latest/docker_and_oci.html).
