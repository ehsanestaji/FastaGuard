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

v0.4 compare mode adds cohort-level artifacts:

```text
cohort.json
cohort.tsv
cohort_report.html
fastaguard_compare_mqc.json
```

## JSON Contract

Example v0.3 shape:

```json
{
  "schema_version": "0.3.0",
  "tool": {
    "name": "FastaGuard",
    "version": "0.3.0"
  },
  "input": {
    "path": "sample.fa",
    "profile": "assembly",
    "compressed": false
  },
  "verdict": {
    "status": "FAIL",
    "reasons": ["duplicate_ids", "invalid_chars", "high_n_rate"]
  },
  "gate": {
    "mode": "pipeline",
    "status": "FAIL",
    "blocking_findings": ["duplicate_ids", "invalid_chars", "high_n_rate"],
    "advisory_findings": ["tiny_contigs", "gc_outliers"],
    "fail_on": ["duplicate_ids", "high_n_rate", "invalid_chars", "invalid_fasta_structure"]
  },
  "machine_summary": {
    "verdict": "FAIL",
    "safe_for_downstream": false,
    "top_findings": ["high_n_rate", "tiny_contigs"],
    "recommended_next_tools": [
      {
        "tool": "seqkit",
        "reason": "High ambiguity may indicate unresolved assembly regions or masking problems."
      },
      {
        "tool": "QUAST",
        "reason": "Assembly-level evaluation can show whether ambiguity affects broader assembly quality."
      }
    ],
    "routing_hints": [
      {
        "condition": "assembly_ambiguity",
        "suggested_route": "gap_closing_or_polishing_review",
        "requires_external_database": false
      },
      {
        "condition": "small_record_review",
        "suggested_route": "review_or_filter_short_records",
        "requires_external_database": false
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
    "fail_on": ["duplicate_ids", "high_n_rate", "invalid_chars", "invalid_fasta_structure"],
    "command": "fastaguard sample.fa --profile assembly --gate pipeline",
    "started_at": "2026-05-23T00:00:00Z",
    "completed_at": "2026-05-23T00:00:01Z",
    "duration_ms": 842,
    "input_size_bytes": 5120340,
    "input_sha256": "3f786850e387550fdab836ed7e6dc881de23001b3a28c9f1f4b2d0a4c6e7f8aa",
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
    "min_length": 120,
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
    "invalid_sequence_count": 1,
    "high_n_sequence_count": 62,
    "tiny_contig_count": 4,
    "max_gap_run": 25
  },
  "plots": {
    "length_histogram": [
      {
        "min_length": 120,
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
      "category": "composition",
      "severity": "major",
      "confidence": "high",
      "requires_followup_tool": false,
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
    },
    {
      "id": "tiny_contigs",
      "category": "structure",
      "severity": "minor",
      "confidence": "moderate",
      "requires_followup_tool": false,
      "profile": "assembly",
      "affected_count": 4,
      "affected_fraction": 0.008,
      "message": "4 contigs are shorter than the 200 bp profile minimum.",
      "why_it_matters": "Very short contigs often add noise to assembly statistics and downstream annotation.",
      "suggested_next_step": "Filter or review tiny contigs before using the assembly in production workflows.",
      "evidence": {
        "total_records": 4,
        "truncated": false,
        "records": [
          {
            "id": "contig_17",
            "length": 120,
            "reason": "shorter than profile minimum contig length",
            "gc_percent": 49.2
          }
        ]
      },
      "actions": [
        {
          "action_type": "filter_or_review_records",
          "target": "tiny contigs",
          "reason": "Short records may be noise, but should be reviewed before automatic removal.",
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

Stable in the v0.3 contract:

- `schema_version`
- `tool.name`
- `tool.version`
- `input.profile`
- `verdict.status`
- `verdict.reasons`
- `gate.mode`
- `gate.status`
- `gate.blocking_findings`
- `gate.advisory_findings`
- `gate.fail_on`
- `machine_summary.verdict`
- `machine_summary.safe_for_downstream`
- `machine_summary.top_findings`
- `machine_summary.recommended_next_tools`
- `machine_summary.routing_hints`
- `scope.level`
- `scope.can_conclude`
- `scope.cannot_conclude`
- `provenance.profile`
- `provenance.command`
- `provenance.started_at`
- `provenance.completed_at`
- `provenance.duration_ms`
- `provenance.input_size_bytes`
- `provenance.input_sha256`
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
- `findings[].category`
- `findings[].severity`
- `findings[].confidence`
- `findings[].requires_followup_tool`
- `findings[].message`
- `findings[].why_it_matters`
- `findings[].suggested_next_step`
- `findings[].evidence`
- `findings[].actions`

Fields can be added in later schema versions, but existing meanings should not drift casually.

## Gate Contract

The v0.3 assembly gate makes workflow stop/go behavior explicit in JSON:

```json
"gate": {
  "mode": "pipeline",
  "status": "FAIL",
  "blocking_findings": ["duplicate_ids", "invalid_chars", "high_n_rate"],
  "advisory_findings": ["tiny_contigs", "gc_outliers"],
  "fail_on": ["duplicate_ids", "high_n_rate", "invalid_chars", "invalid_fasta_structure"]
}
```

Machines should use `gate.blocking_findings` for workflow stop/go decisions.
This list is the stable explanation of why a gated run blocked downstream work.

Humans should inspect the HTML evidence before deciding how to repair or route
the assembly. Advisory findings such as GC or length outliers can indicate
records worth reviewing, but they are not blocking unless the user explicitly
adds them with `--fail-on`.

`provenance.input_sha256` identifies the exact input bytes used for the report.
That checksum lets workflow engines, reviewers, and future audit tools connect a
gate decision to one immutable FASTA input.

## Readiness And Compare Contract

The v0.4 contract adds preflight readiness categories without changing
FastaGuard's boundary. Readiness tells workflow engines whether a FASTA is ready
for file consumption, parsing, symbol validation, indexing, assembly triage,
submission review, and machine-readable routing. It does not prove biological
completeness, assembly correctness, or contamination.

Readiness categories:

- file
- structure
- alphabet
- index
- assembly
- submission
- machine

Compare mode wraps single-file reports into a cohort triage layer:

```bash
fastaguard compare assemblies/*.fa \
  --profile assembly \
  --gate pipeline \
  --out cohort_report.html \
  --json cohort.json \
  --tsv cohort.tsv \
  --multiqc fastaguard_compare_mqc.json
```

Machines should treat `cohort.json` as the source of truth. The TSV is for
filtering and spreadsheet review, the HTML report is for human triage, and
`fastaguard_compare_mqc.json` is for MultiQC-compatible cohort summaries.
Compare mode ranks and routes FASTA files before QUAST, BUSCO, BlobToolKit,
CheckM, official validators, annotation, or other interpretive QC tools; it does
not replace them.

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
schema_version	0.3.0
profile	assembly
verdict	FAIL
gate_mode	pipeline
gate_status	FAIL
gate_blocking_findings	duplicate_ids,invalid_chars,high_n_rate
gate_advisory_findings	tiny_contigs,gc_outliers
input_sha256	3f786850e387550fdab836ed7e6dc881de23001b3a28c9f1f4b2d0a4c6e7f8aa
sequence_count	481
total_length	5042301
n50	128003
n90	24013
l50	12
l90	81
gc_percent	51.8
n_percent	3.4
gc_outlier_count	1
length_outlier_count	1
composite_anomaly_count	1
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

This keeps FastaGuard easy to integrate as MultiQC custom content and as input
to the native FastaGuard MultiQC plugin.

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

Outlier findings are part of the v0.3 report contract. They are preflight
triage signals only; GC and composite anomalies do not by themselves prove
contamination, cobionts, plasmids, or misassembly.
