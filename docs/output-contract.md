# Output Contract

## Principle

FastaGuard should be pipeline-native from the first release.

The output contract is as important as the HTML report. Pipeline authors need stable field names, deterministic behavior, documented exit codes, and versioned schemas.

## Artifacts

```text
fastaguard.json
fastaguard.tsv
fastaguard_report.html
fastaguard_mqc.json
schema/fastaguard.schema.json
schema/finding-catalog.json
```

## JSON Contract

Example v0.1 shape:

```json
{
  "schema_version": "0.1.0",
  "tool": {
    "name": "FastaGuard",
    "version": "0.1.0"
  },
  "input": {
    "path": "sample.fa",
    "profile": "assembly",
    "compressed": false
  },
  "verdict": {
    "status": "WARN",
    "reasons": ["high_n_rate", "duplicate_ids"]
  },
  "machine_summary": {
    "verdict": "WARN",
    "safe_for_downstream": false,
    "top_findings": ["high_n_rate", "duplicate_ids"],
    "recommended_next_tools": [
      {
        "tool": "QUAST",
        "reason": "Assembly-level evaluation can show whether ambiguity affects broader assembly quality."
      }
    ]
  },
  "scope": {
    "level": "fasta_preflight",
    "can_conclude": ["FASTA parse validity", "duplicate identifiers"],
    "cannot_conclude": ["biological completeness", "taxonomic contamination"]
  },
  "provenance": {
    "profile": "assembly",
    "threads": 1,
    "fail_on": [],
    "thresholds": {
      "high_n_sequence_fraction": 0.2,
      "high_global_n_fraction": 0.05,
      "min_contig_length": 200,
      "max_gap_run": 100,
      "gc_outlier_zscore": 3.0
    }
  },
  "summary": {
    "sequence_count": 481,
    "total_length": 5042301,
    "min_length": 203,
    "max_length": 512044,
    "mean_length": 10483.0,
    "median_length": 6012.0,
    "n50": 128003,
    "n90": 24013,
    "l50": 12,
    "l90": 81,
    "gc_percent": 51.8,
    "at_percent": 44.8,
    "n_percent": 3.4,
    "ambiguity_percent": 3.7,
    "duplicate_id_count": 1,
    "duplicate_sequence_count": 0,
    "invalid_sequence_count": 0,
    "high_n_sequence_count": 62,
    "tiny_contig_count": 4,
    "max_gap_run": 25
  },
  "plots": {
    "length_histogram": [
      {
        "min_length": 203,
        "max_length": 50400,
        "sequence_count": 41,
        "total_length": 993002
      }
    ],
    "gc_length_plot": [
      {
        "id": "scaffold_42",
        "length": 18004,
        "gc_percent": 51.2,
        "n_percent": 37.0,
        "gc_zscore": 0.8,
        "flags": ["high_n"]
      }
    ]
  },
  "findings": [
    {
      "id": "high_n_rate",
      "severity": "major",
      "profile": "assembly",
      "affected_count": 62,
      "affected_fraction": 0.128,
      "message": "12.8% of sequences contain more than 20% Ns.",
      "why_it_matters": "High ambiguity can reduce annotation and mapping quality.",
      "suggested_next_step": "Inspect high-N scaffolds or run gap closing/polishing.",
      "evidence": {
        "total_records": 62,
        "truncated": true,
        "records": [
          {
            "id": "scaffold_42",
            "length": 18004,
            "reason": "per-sequence N fraction exceeded threshold",
            "n_fraction": 0.37,
            "n_percent": 37.0
          }
        ]
      },
      "actions": [
        {
          "action_type": "inspect_records",
          "target": "high-N scaffolds",
          "reason": "High ambiguity may indicate unresolved assembly regions or masking problems.",
          "recommended_tool": "seqkit",
          "requires_external_database": false
        }
      ]
    }
  ],
  "artifacts": {
    "html": "fastaguard_report.html",
    "tsv": "fastaguard.tsv",
    "multiqc": "fastaguard_mqc.json"
  }
}
```

## Stability Rules

Stable from v0.1:

- `schema_version`
- `tool.name`
- `tool.version`
- `input.profile`
- `verdict.status`
- `verdict.reasons`
- `machine_summary.verdict`
- `machine_summary.safe_for_downstream`
- `machine_summary.top_findings`
- `scope.level`
- `scope.can_conclude`
- `scope.cannot_conclude`
- `provenance.profile`
- `provenance.thresholds`
- `summary.sequence_count`
- `summary.total_length`
- `summary.n50`
- `summary.n90`
- `summary.l50`
- `summary.l90`
- `summary.gc_percent`
- `summary.n_percent`
- `plots.length_histogram`
- `plots.gc_length_plot`
- `findings[].id`
- `findings[].severity`
- `findings[].message`
- `findings[].why_it_matters`
- `findings[].suggested_next_step`
- `findings[].evidence`
- `findings[].actions`

Fields can be added in later schema versions, but existing meanings should not drift casually.

## Machine-Actionable Contract

The JSON output should become the source of truth for humans, workflow engines, dashboards, and future tool-using LLM agents.

An agent should be able to answer these questions without scraping HTML or logs:

- what was the verdict?
- which findings drove that verdict?
- what evidence and thresholds support each finding?
- which records triggered each finding?
- what next action is safe?
- what is outside FastaGuard's scope?

Current schema versions include structured `actions`, bounded per-record `evidence`, `provenance`, explicit `scope`, and a compact `machine_summary`. Future schema versions can add richer evidence fields while preserving the stable fields above.

## Contract Discovery Commands

FastaGuard exposes its machine-readable contract without requiring an input FASTA:

```bash
fastaguard --schema
fastaguard --finding-catalog
fastaguard --explain-finding high_n_rate
```

These commands are intended for workflow engines, agentic tools, documentation generators, and validation layers. They provide the current report schema, the complete finding catalog, and a single finding definition by stable ID.

## Conformance Fixtures

Golden JSON fixtures live in `tests/golden/` and cover:

- a passing assembly FASTA
- an assembly FASTA with QC failures
- a structurally invalid FASTA

These fixtures are part of the machine-readable contract. If they change, the change should be intentional and reviewed as schema or semantics drift.

## TSV Contract

The TSV should be easy to parse with shell tools and workflow engines.

Recommended first rows:

```text
metric	value
schema_version	0.1.0
profile	assembly
verdict	WARN
sequence_count	481
total_length	5042301
n50	128003
n90	24013
l50	12
l90	81
gc_percent	51.8
n_percent	3.4
finding_count	2
```

## MultiQC Contract

The MultiQC JSON is emitted as MultiQC custom content and should use the default filename:

```text
fastaguard_mqc.json
```

The top-level shape should be deliberately boring:

- `id`
- `section_name`
- `description`
- `plot_type`
- `pconfig`
- `data`

This keeps FastaGuard easy to integrate before a custom MultiQC module exists.

## HTML Report

The HTML report should be:

- self-contained
- static
- shareable
- readable without a server
- backed by the same JSON data model as the machine-readable output

Report layers:

```text
1. Verdict
   PASS / WARN / FAIL with profile-aware reasons

2. Evidence
   summary tables, finding details, length histogram, GC-vs-length plot, and embedded JSON

3. Actions
   suggested next tools and remediation steps
```

Outlier findings and richer plot interactivity are planned beyond the v0.1 report contract.
