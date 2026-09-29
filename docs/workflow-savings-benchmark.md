# Workflow Savings Benchmark

## Purpose

Measure the work avoided when FastaGuard detects a reference declaration
mismatch before a workflow starts mapping and sorting. This is a controlled
synthetic bacterial-scale experiment, not a universal performance claim.

## Comparison

Run the same input three times in each condition:

- **Preflight gate:** run `fastaguard reference` first. A blocking report
  starts no downstream task.
- **Late validation baseline:** run the mapping and sorting tasks first, then
  validate the same reference declaration mismatch. This captures work that an
  early gate avoids.

The baseline must use a strict safety time limit. A timed-out run is reported as
a lower bound, never as an exact saving.

## Required observation fields

The inputs are JSON arrays of observation objects. Each observation requires:

| Field | Meaning |
| --- | --- |
| `run_id` | Repeat identifier, unique within each input file and present in both files. Array order does not determine pairing. |
| `engine` | Workflow engine used for this comparison. |
| `case_id` | One fixed reference, declaration, policy and downstream-workload condition. Preserve its full configuration and companion checksums beside the observations. |
| `input_sha256` | Lowercase SHA-256 digest of the canonical reference input bytes. |
| `binary_sha256` | Lowercase SHA-256 digest of the measured FastaGuard executable. |
| `runner_id` | Non-identifying label for the pinned platform, workflow-engine version and resource configuration. Preserve those details with the raw observations. |
| `status` | `completed` for the gate; `completed` or `timeout` for the late-validation baseline. |

All observations in one summary must share the same engine, case, input digest,
binary digest and runner. Use separate summaries for different conditions.
Run IDs must match exactly, with no duplicates or missing partners. These
fields identify the recorded comparison; they do not independently attest
that a measurement was performed correctly.

Each gated observation also records `gate_can_continue: false`,
`preflight_wall_seconds`, `preflight_cpu_seconds`, `preflight_peak_rss_kib`, and
an empty `downstream_tasks` list. A completed report with a blocking decision is
not a timed-out gate. A gate that permits continuation or starts downstream
work is not a valid observation for this blocked-work experiment.

Each late-validation observation contains a non-empty `downstream_tasks` list.
Each task records `name`, `wall_seconds`, `cpu_seconds`, `requested_cpus`, and
`peak_rss_kib`. CPU allocations are positive integers; all measured times and
RSS values are finite, non-negative numbers, not booleans or numeric strings.
Normalise RSS to KiB before recording it. Missing measurements must not be
replaced with zero. Non-finite values, arithmetic overflow, malformed JSON and
duplicate JSON keys are rejected.

Optionally, record `workflow_wall_seconds` for both members of every pair.
This is measured whole-workflow elapsed time, including scheduling and other
overheads. Partial coverage is rejected rather than silently summarising a
subset of repeats.

Summarise matched observations with:

```bash
python3 scripts/benchmark_workflow_savings.py \
  --gated gated-observations.json \
  --ungated late-validation-observations.json \
  --out workflow-savings-summary.json
```

The output path must not already exist; the command will not replace an earlier
summary or an observation file. Invalid inputs exit with code 2 without
publishing a summary.

## Summary contract

The utility writes `summary_schema_version: 2`, the comparison context, sorted
run IDs, repeat counts and timeout IDs. This is a benchmark utility format,
not a change to FastaGuard's report schemas.

Metrics are medians of per-pair observations or differences:

- `median_allocated_cpu_hours_avoided`: gross downstream allocation,
  `sum(task.wall_seconds * task.requested_cpus) / 3600`. This does not subtract
  the gate's scheduled CPU allocation and is not a net compute-cost estimate.
- `median_actual_cpu_seconds_avoided`: summed baseline task CPU seconds minus
  preflight CPU seconds, retaining negative results.
- `median_task_summed_wall_seconds_avoided`: summed baseline task wall seconds
  minus preflight wall seconds, also retaining negative results. Parallel tasks
  contribute separately; this is not elapsed workflow time.
- `median_workflow_wall_seconds_avoided`: measured baseline workflow elapsed
  time minus measured gated workflow elapsed time, or `null` if neither was
  supplied. It is never inferred from task durations.
- Preflight wall time and peak RSS, downstream task count, and the maximum
  individual downstream task RSS in each repeat. The latter is not the combined
  peak memory of concurrent tasks.

Machine-readable metrics retain floating-point precision without display
rounding, so small negative savings are not hidden and timeout lower bounds
are not rounded upwards. Apply appropriate formatting separately in tables.

A summary containing any timed-out baseline has `estimate_kind: "lower_bound"`
and lists its censored repeats separately from completed pairs. Task costs
observed before the timeout are lower bounds on completed-run costs; their
derived median savings remain lower bounds even when mixed with completed
repeats. All required measurements must still be available. Ordinary failures
and cancellations are rejected, not reclassified as timeouts. A summary with
no timeouts has `estimate_kind: "observed"`, not a guarantee about future runs.

The old ambiguous `median_wall_seconds_avoided` key is replaced by the explicit
task-summed metric. Historical observations lacking identity or status fields
must be reconstructed from retained raw evidence or rerun. Do not guess those
fields from array order or a published median. Use three repeats per named
condition for the evidence campaign; the utility reports the actual count
without claiming that fewer repeats meet that campaign requirement.

## Workflow-native sources

For Nextflow, take task duration, CPU, requested CPUs, and peak RSS from the
trace report. For Snakemake, take wall time, CPU time, requested threads, and
maximum RSS from its benchmark output. Preserve the original trace or benchmark
files beside local results; publish only the compact summary after reviewing it
for paths, commands, and input sequence data.

## Interpretation

State the result only in this form: the measured reference mismatch was stopped
before mapping and sorting, avoiding the reported median work on the named
runner and fixture. Do not generalise the value to every workflow, input, or
platform. Some downstream tools accept incompatible reference declarations, so
the experiment measures preflight placement rather than claiming a universal
native failure mode.

## Local results

The tables below record earlier experiments, not results of the current
reference-evidence campaign. Their original measurements have not been
recomputed under the version 2 observation contract.

Three matched runs were completed in a Linux container using a deterministic
5 Mbp reference and 20,000 150 bp reads. The late-validation task was limited
to two minutes. Values below are medians from workflow-native reports.

| Engine | Preflight wall time | Wall time avoided | Allocated CPU-hours avoided | Actual CPU time avoided | Downstream tasks avoided | Peak RSS: gate / late task |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Snakemake | 0.05 s | 0.74 s | 0.0004 | 0.75 s | 1 | 1.8 / 63.5 MiB |
| Nextflow | 0.161 s | 0.819 s | 0.0005 | 1.244 s | 1 | 15.9 / 33.0 MiB |

These values show that the mismatch was stopped before mapping and sorting in
this fixture. They are evidence for the gate's placement and overhead, not a
claim that every workflow will save the same amount of time or compute.

## Real-data local results

Three matched runs used ENA run `SRR27793280`: 4,420,110 paired-end Illumina
read pairs from *Escherichia coli* K-12 MG1655. The reference was NCBI
`NC_000913.3` (4,641,652 bp). The original reads and reference sequence were
unchanged; the test increased the declared length of the first reference entry
in a copied `.fai` file by one base. Each late-validation task had a 15-minute
safety limit.

| Engine | Preflight wall time | Wall time avoided | Allocated CPU-hours avoided | Actual CPU time avoided | Downstream tasks avoided | Peak RSS: gate / late task |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Snakemake | 0.03 s | 45.53 s | 0.0253 | 41.07 s | 1 | 0.9 MiB / 2.14 GiB |
| Nextflow | 0.035 s | 44.966 s | 0.0250 | 108.873 s | 1 | 15.2 MiB / 2.3 GiB |

The late task performed paired-end mapping, coordinate sorting, indexing, and
then the same reference check. Snakemake's actual CPU time is its native
`cpu_time` field. Nextflow's is calculated from the trace report's `%cpu` and
`realtime` fields. The distinct CPU values are therefore not directly
interchangeable across engines; the allocation and wall-time measurements are
the clearest comparison.

This is a controlled late-validation baseline. It demonstrates that this
specific reference declaration mismatch can be rejected before a substantial
mapping task begins. It does not claim that every laboratory workflow will
save these exact amounts.
