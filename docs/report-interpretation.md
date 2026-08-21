# Report interpretation

JSON is the source of truth. HTML is the human view, TSV is a flat handoff, and
the `_mqc.json` file is a compact aggregation view. Do not scrape HTML or infer
workflow policy from presentation text.

## Three different decisions

| Field | Question answered | Recommended use |
| --- | --- | --- |
| `verdict.status` | What is the overall PASS/WARN/FAIL assessment? | Human triage and broad status summaries |
| `machine_summary.safe_for_downstream` | Is the conservative overall verdict PASS? | Strict consumers that accept only PASS |
| `gate.can_continue` | Do the findings block the selected gate? | Workflow continuation under an explicit gate policy |

These fields are intentionally not synonyms. For example, a WARN report can
have `gate.can_continue = true` when all warnings are advisory under the
selected gate. Conversely, consumers should inspect `gate.blocking_findings`
when `gate.can_continue` is false rather than guessing from a finding count.

## Process status is not QC policy

A process exit code records report generation status, not the QC verdict.
Starting with the published v0.6.0 contract, exit code `0` covers completed
PASS, WARN, and FAIL reports. Exit code `2` indicates argument parsing failure,
and exit code `3` indicates a configuration, input-access/I/O, runtime, or
output-write error.

Collect the report first, then route on JSON:

```bash
fastaguard sample.fa \
  --gate pipeline \
  --json fastaguard.json \
  --out fastaguard.html

jq -e '.gate.can_continue == true' fastaguard.json >/dev/null
```

## Practical reading order

1. Confirm `tool.version`, `schema_version`, `input.path`, and provenance.
2. Read `verdict.status` and the short `machine_summary`.
3. Check the selected `gate.mode`, `gate.can_continue`, and blocking finding IDs.
4. Review each finding's severity, evidence, thresholds, and suggested actions.
5. Use `scope` to identify conclusions that require another tool or official
   validator.

Stable finding IDs are safer workflow inputs than message wording. Thresholds
and affected-record evidence explain why a finding fired; they do not establish
biological completeness, contamination, taxonomy, annotation correctness, or
repository acceptance.

## TSV and MultiQC

The single-file TSV is a two-column `metric`/`value` table for simple
row-oriented handoffs. `verdict`, `gate_status`, and `gate_can_continue` are
metric-row names, not TSV column headers; retrieve their values by selecting
the corresponding `metric` rows:

```bash
python3 -c 'import csv; rows = {row["metric"]: row["value"] for row in csv.DictReader(open("reports/sample-01.fastaguard.tsv"), delimiter="\t")}; print({key: rows[key] for key in ("verdict", "gate_status", "gate_can_continue")})'
```

Use JSON for nested evidence and the actual continuation boolean. The
MultiQC-compatible file is for cohort visibility; the detailed FastaGuard JSON
and HTML remain the evidence to inspect.
