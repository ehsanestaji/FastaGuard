# Five-minute quickstart

This path uses the published v1.0.0 CLI, available through Bioconda,
BioContainers, and GitHub release binaries. It includes assembly preflight and
the Reference Contract Gate.

## 1. Install the published CLI

With Bioconda:

```bash
mamba create -n fastaguard -c conda-forge -c bioconda fastaguard=1.0.0
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

## Bundle mode

The published v1.0.0 CLI also supports the shorter bundle command:

```bash
fastaguard sample.fa --outdir reports --prefix sample-01
```

It writes the same four names listed above. Existing outputs are protected
unless `--force` is supplied.

## Reference quickstart

This example needs the installed v1.0.0 CLI, Bash, Python 3, and a repository
checkout containing the fixtures and gate helper. Run it from the repository
root; the optional JSON inspection below also uses `jq`:

```bash
bash examples/reference/quickstart.sh fastaguard target/reference-quickstart
```

The four-base reference is synthetic demonstration data, not biological
validation evidence. `reference.dict` declares its correct length of four;
`copied-length-mismatch.dict` differs only in declaring five bases. The example
checks both dictionaries against the same FASTA using the coordinate policy.

Both report commands exit `0` and produce these files under `pass/` and
`mismatch/` inside the output directory:

- `reference.fastaguard.html`
- `reference.fastaguard.json`
- `reference.fastaguard.tsv`
- `reference.fastaguard_mqc.json`

The matching case also writes `pass/reference.fastaguard.lock.json`. Inspect
the blocking decision:

```bash
jq '{verdict: .verdict.status, can_continue: .gate.can_continue}' \
  target/reference-quickstart/mismatch/reference.fastaguard.json
python3 examples/workflows/check_fastaguard_gate.py \
  target/reference-quickstart/mismatch/reference.fastaguard.json
```

The matching report is PASS with `gate.can_continue: true`. The mismatched
report is FAIL with `gate.can_continue: false`; the separate helper exits `2`
without removing the reports. Missing or malformed gate information makes the
helper exit `3`.

The demonstration runner exits `0` only when both expected outcomes occur.
It is not a reusable workflow gate. Use a new output directory to repeat the
example while retaining previous evidence; existing files are not overwritten.
The first argument can be a quoted executable path instead of `fastaguard`.

### Write a reference lockfile

`--write-lock` is a boolean switch. With `--outdir`, the lockfile is named
`<prefix>.fastaguard.lock.json` alongside the reports. For an explicit lock
path, use `--lock PATH` without `--outdir`:

```bash
mkdir -p target/reference-explicit-lock
fastaguard reference examples/reference/fixtures/reference.fa \
  --dict examples/reference/fixtures/reference.dict --require dict \
  --format json --json target/reference-explicit-lock/report.json \
  --write-lock --lock target/reference-explicit-lock/baseline.json
```

Lock writing requires JSON output. The v1.0 lock records semantic reference
identity and declarations; it is not an approval or signed attestation. This
version writes lockfiles but does not verify a later run against one.

## Container alternatives for published v1.0.0

Docker:

```bash
docker run --rm -v "$PWD:/work" \
  quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0 \
  fastaguard /work/sample.fa --json /work/reports/sample-01.fastaguard.json
```

Apptainer can execute the same public OCI image from Quay:

```bash
apptainer exec --bind "$PWD:/work" \
  docker://quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0 \
  fastaguard /work/sample.fa --json /work/reports/sample-01.fastaguard.json
```

The exact tag is visible in the
[BioContainers registry](https://quay.io/repository/biocontainers/fastaguard?tab=tags).
Apptainer documents `docker://` images from Quay in its
[OCI container guide](https://apptainer.org/docs/user/latest/docker_and_oci.html).
