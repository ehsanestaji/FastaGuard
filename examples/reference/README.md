# Reference workflows

These single-bundle examples run the published FastaGuard v1.0.0 reference
command. Run them from the repository root with FastaGuard and the chosen
workflow engine on `PATH`. Each example uses the same synthetic reference and
dictionary as the [reference quickstart](../../docs/quickstart.md#reference-quickstart).

Nextflow (DSL2):

```bash
nextflow run examples/reference/nextflow/main.nf \
  -params-file examples/reference/nextflow/params.json
```

Snakemake:

```bash
snakemake --snakefile examples/reference/snakemake/Snakefile --cores 1 \
  --configfile examples/reference/snakemake/config.json
```

The two checked-in configurations use different output roots. Each writes a
run directory named `exact` containing HTML, JSON, TSV, MultiQC JSON, a
reference lockfile, and `downstream.started` after the gate accepts the bundle.
The lockfile is optional and has its own output declaration. The marker is a
demonstration of a downstream task, not a biological analysis.

Use a native JSON parameter/configuration file for a real bundle. Both engines
accept these keys:

| Key | Meaning |
| --- | --- |
| `fasta` | Required canonical FASTA path. |
| `output_dir` | Required local or shared-filesystem output root. |
| `run_id` | Unique name for this input set; letters, digits, `_` and `-`, starting with a letter or digit. Defaults to `reference`. |
| `mode` | `report` (default) or `enforce`. |
| `fai`, `dict`, `alias_map` | Optional explicit file paths. |
| `alignments`, `variants`, `annotations` | Lists of explicit file paths; each item is a workflow file dependency. |
| `policy` | `strict`, `coordinate` (default), or `advisory`. |
| `require` | List drawn from `fai`, `dict`, `alignment`, `variants`, `annotation`. |
| `write_lock` | Boolean; when true, request a lockfile alongside the reports. |
| `binary` | Optional executable path or command; defaults to `fastaguard`. |

An omitted optional file differs from a supplied unreadable path. All four
reports are requested together. Use a new `run_id` when any input or report
setting changes; both examples reject reuse that would contradict an existing
run. Both examples record a hidden `.reference-inputs.sha256` file and hash
the supplied inputs before reporting. Large references add a second streaming
read for that protection. Snakemake also records
`.reference-report-digests.json` and checks the published report files on
each run; missing or modified evidence is an error.

`report` mode stops after saving the reports, including a completed FAIL
report. In `enforce` mode, a separate gate reads the JSON boolean
`gate.can_continue`. A blocked gate fails the workflow while leaving all
completed reports in `output_dir/run_id/`; no downstream marker is written.
The original FASTA and companions are never modified.

Nextflow stages repeated companion filenames in numbered subdirectories. Its
persistence task synchronously copies the bundle to `output_dir/run_id/` and
emits a receipt before the gate can run. It accepts `-resume` for unchanged
inputs only after confirming the published bytes match. Snakemake writes
engine-managed reports to a staging directory, then publishes them without
replacing earlier completed evidence. Its gate rule owns only the acceptance
marker. These examples target one bundle on a local or shared filesystem.
Object-store publication and multiple bundles are outside their scope.

The small [fixture manifest](fixtures/workflow-cases.json) defines eight
expected decision cases used by the engine integration tests. It is test data,
not biological validation evidence.

## Testing

The dedicated GitHub CI job uses Nextflow 26.04.6, Java 17, Snakemake 9.25.2,
and Python 3.12. It runs both workflow linters and the full reference test
file, including the decision matrix, report retention, and retry checks.

With both engines on `PATH` and the Python test dependencies installed:

```bash
cargo build --locked --bin fastaguard
FASTAGUARD_REQUIRE_WORKFLOW_ENGINES=1 \
  python3 -m pytest -q -ra tests/python/test_reference_workflows.py
```

Required mode fails when an engine is unavailable and rejects a
`FASTAGUARD_WORKFLOW_CASE` filter. Without required mode, local tests may
skip an unavailable engine.
