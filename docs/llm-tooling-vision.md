# LLM And Tooling Vision

## Principle

FastaGuard should prepare FASTA QC for humans, workflow engines, and future tool-using LLM agents.

The long-term idea is simple:

```text
Do not make machines scrape reports.
Give them a stable QC contract they can reason over safely.
```

The HTML report is for people. The JSON contract is for pipelines, orchestrators, notebooks, dashboards, and future agents that need to decide what should happen next.

## Why This Matters

Bioinformatics workflows are becoming more automated. Soon, an LLM or workflow agent may need to answer:

- did this FASTA pass preflight QC?
- which findings matter most?
- what evidence supports each finding?
- which downstream tools are appropriate next?
- what should not be inferred from this report?

FastaGuard should make those answers available as structured data, not hidden inside prose.

## Product Position

FastaGuard is not trying to replace QUAST, BUSCO, BlobToolKit, CheckM, seqkit, or MultiQC.

It should become the machine-readable preflight layer before those tools:

```text
Validate FASTA first.
Expose findings as stable structured data.
Let humans, pipelines, and agents decide the next step confidently.
```

## Design Rules

- JSON is the source of truth; HTML, TSV, and MultiQC outputs are derived views.
- Every important human-readable finding should map to a stable machine-readable field.
- Finding IDs should be stable, documented, and safe for workflow conditions.
- Verdicts, severity, evidence, thresholds, and suggested actions should be explicit fields.
- Reports should include provenance: tool version, schema version, profile, thresholds, command context, and input metadata.
- LLM-facing summaries must only summarize local FastaGuard metrics and findings.
- FastaGuard should clearly state scope limits, especially when a next tool is needed for biological completeness, assembly quality, or contamination analysis.
- Agents should never need to scrape HTML or parse log text to understand a FastaGuard run.

## Recommended Todo List

1. Publish a formal JSON Schema, for example `schema/fastaguard.schema.json`.
2. Add a documented finding catalog that maps each `finding_id` to severity, meaning, thresholds, evidence fields, and suggested actions.
3. Add structured `actions[]` records to findings, with fields such as `action_type`, `target`, `recommended_tool`, `reason`, and `requires_external_database`.
4. Add a `machine_summary` block with verdict, top findings, downstream readiness, and recommended next tools.
5. Add a richer `provenance` block with command, arguments, profile, thresholds, timestamps, input size, and eventually input checksum.
6. Add explicit `scope` fields so agents know what FastaGuard can and cannot conclude.
7. Add `--schema` and `--explain-finding <id>` commands so other tools can discover the contract programmatically.
8. Add a stable schema migration policy before the first widely distributed release.
9. Add conformance tests with tiny FASTA fixtures and golden JSON outputs.
10. Add an MCP or tool-server layer later, after the CLI contract is stable.
11. Add compare-mode outputs that make many FASTA files easy for agents to rank, filter, and route.
12. Keep generated LLM summaries optional, local-metrics-only, and traceable back to structured fields.

## Future JSON Shape

Example fields that would make FastaGuard easier for machines and LLM agents to consume:

```json
{
  "machine_summary": {
    "verdict": "WARN",
    "safe_for_downstream": false,
    "top_findings": ["high_n_rate"],
    "recommended_next_tools": [
      {
        "tool": "QUAST",
        "reason": "assembly-level evaluation after FASTA validity checks"
      },
      {
        "tool": "BUSCO",
        "reason": "biological completeness after structural issues are handled"
      }
    ]
  },
  "actions": [
    {
      "finding_id": "high_n_rate",
      "action_type": "inspect_records",
      "target": "high_n_sequences",
      "human_text": "Inspect high-N scaffolds before annotation.",
      "machine_hint": "filter findings where id == 'high_n_rate'",
      "requires_external_database": false
    }
  ],
  "scope": {
    "level": "fasta_preflight",
    "can_conclude": [
      "FASTA parse validity",
      "duplicate identifiers",
      "sequence composition red flags"
    ],
    "cannot_conclude": [
      "biological completeness",
      "taxonomic contamination",
      "whole-assembly accuracy"
    ]
  }
}
```

## Recommendation

Make this a core product principle, but keep the first implementation boring:

```text
v0.1: stable JSON and finding IDs
v0.2: JSON Schema and finding catalog
v0.3: structured actions and provenance
later: MCP/tool-agent interface and optional local summaries
```

That lets FastaGuard become useful to machines without weakening the current promise: fast, explainable FASTA preflight QC.

## Current Contract Commands

The first contract-discovery commands are:

```bash
fastaguard --schema
fastaguard --finding-catalog
fastaguard --explain-finding high_n_rate
```

These are the first practical bridge from "nice report" to "tool-readable QC contract."
