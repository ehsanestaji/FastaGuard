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

Each gated observation records `preflight_wall_seconds`,
`preflight_cpu_seconds`, `preflight_peak_rss_kib`, and an empty
`downstream_tasks` list. Each late-validation task records `name`,
`wall_seconds`, `cpu_seconds`, `requested_cpus`, and `peak_rss_kib`.

Summarise matched observations with:

```bash
python3 scripts/benchmark_workflow_savings.py \
  --gated gated-observations.json \
  --ungated late-validation-observations.json \
  --out workflow-savings-summary.json
```

The summary reports medians for preflight overhead, downstream tasks started,
allocated CPU-hours avoided, actual CPU seconds avoided after subtracting
preflight CPU, wall time avoided, and peak RSS in both conditions.

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
