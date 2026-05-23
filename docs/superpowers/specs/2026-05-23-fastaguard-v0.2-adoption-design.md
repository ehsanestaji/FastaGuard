# FastaGuard v0.2 Design: Assembly Trust And Pipeline Adoption

## Summary

FastaGuard v0.2 should make the tool easier to adopt in real bioinformatics
pipelines while adding one focused assembly QC improvement.

Release theme:

```text
FastaGuard v0.2: Assembly Trust + Pipeline Adoption
```

Product promise:

```text
Install it easily, aggregate it in MultiQC, trust its JSON contract, and catch
assembly-level outliers before heavier QC tools run.
```

v0.2 should keep `assembly` as the main profile. It should not rush
transcriptome, protein, or reference-panel profiles before the assembly
preflight contract is trusted by pipeline users.

## Goals

- Make FastaGuard feel natural in workflow engines and reporting stacks.
- Harden the MultiQC path so many samples can be scanned quickly.
- Confirm and document the distribution path after Bioconda publication.
- Add explainable assembly outlier findings for composition and length.
- Strengthen machine-readable provenance for pipelines and future tool agents.
- Preserve the v0.1 contract shape wherever possible.

## Non-Goals

- Do not claim contamination detection from GC outliers alone.
- Do not add external database dependencies in v0.2.
- Do not build transcriptome, protein, or reference-panel profiles yet.
- Do not make an LLM/chat feature.
- Do not replace QUAST, BUSCO, BlobToolKit, CheckM, sourmash, Kraken, or
  MultiQC.

## Product Position

v0.1 proved the core FASTA preflight contract:

```text
Is this assembly FASTA valid, sane, interpretable, and ready for downstream
tools?
```

v0.2 should prove pipeline value:

```text
Can a pipeline author install FastaGuard, aggregate it, trust its output schema,
and route suspicious assemblies to the right downstream tools?
```

Recommended public message:

```text
FastaGuard catches FASTA-level assembly problems and composition outliers before
expensive downstream QC.
```

## Feature Scope

v0.2 should add one biological feature family:

```text
Assembly outlier findings
```

### GC Outlier Findings

The current GC-vs-length plot can flag GC outliers, but those flags should be
promoted into explainable findings.

Finding ID:

```text
gc_outliers
```

Suggested severity:

```text
Major
```

The finding should trigger when one or more records have GC composition far from
the assembly background according to the profile z-score threshold.

Example interpretation:

```text
Major finding: 8 contigs have GC composition far from the assembly background.
Why it matters: unusual GC can indicate contamination, cobionts, plasmids,
assembly artifacts, or real biological variation.
Suggested next step: inspect flagged contigs and consider BlobToolKit,
sourmash, Kraken, or related taxonomic checks if the pattern is strong.
```

The report must avoid overclaiming. It should say "composition anomaly", not
"contamination", unless a future database-backed mode exists.

### Length Outlier Findings

Add careful length outlier evidence for ultra-short and ultra-long records
relative to the assembly distribution.

Finding ID:

```text
length_outliers
```

Suggested severity:

```text
Minor
```

Length outliers should be presented as review evidence, not automatic failure.
A very long scaffold can be excellent; a very short contig can be expected in
some assemblies. The value is in making the records visible and machine-readable.

### Composite Anomaly Priority

Add a composite anomaly indicator when a record has multiple suspicious signals,
for example:

- GC outlier plus high N
- GC outlier plus long gap run
- GC outlier plus tiny contig
- duplicate sequence plus unusual composition

This should not be a broad new statistical engine in v0.2. It should be a
compact prioritization layer that helps humans and pipeline agents focus on the
records most worth inspecting.

Possible finding ID:

```text
composite_anomalies
```

Suggested severity:

```text
Major
```

### Evidence Records

New findings should follow the existing bounded evidence pattern:

```json
{
  "id": "gc_outliers",
  "evidence": {
    "total_records": 8,
    "truncated": false,
    "records": [
      {
        "id": "contig_42",
        "length": 18004,
        "reason": "GC z-score exceeded profile threshold",
        "gc_percent": 72.4
      }
    ]
  }
}
```

Evidence should be deterministic and sorted in a stable way, such as by
severity signal and then length or ID.

## Integration Scope

### MultiQC

The existing MultiQC plugin starter should become usable enough for real
projects.

Scope:

- Parse many `fastaguard_mqc.json` files.
- Add key metrics to MultiQC general stats.
- Add a FastaGuard summary section.
- Show verdict, sequence count, total length, N50, GC%, N%, duplicate IDs,
  invalid sequences, high-N sequences, and outlier counts.
- Use clear color rules so WARN/FAIL and high-risk metrics are easy to scan.
- Verify with MultiQC strict mode.
- Keep FastaGuard HTML/JSON as the detailed evidence source.

Rationale from MultiQC documentation:

- MultiQC custom content is useful but more limited than real modules.
- Public tools benefit from proper modules or plugins.
- MultiQC modules should avoid embedding many images and should recreate plots
  from raw data where needed.

References:

- https://docs.seqera.io/multiqc/custom_content
- https://docs.seqera.io/multiqc/development/modules/

### BioContainers

Bioconda is live for FastaGuard v0.1.1. v0.2 should confirm whether the
generated BioContainers image and tags are available.

If confirmed:

- Document the image name and version tag.
- Add container examples for Nextflow/nf-core and Snakemake.

If not confirmed:

- Document the status plainly.
- Keep the local Dockerfile path as the temporary container path.

### nf-core Local Module

Polish the local nf-core-style module starter.

Scope:

- Keep input shape: `tuple val(meta), path(fasta)`.
- Keep outputs: HTML, JSON, TSV, `_mqc.json`, and `versions.yml`.
- Document Bioconda install.
- Add a pinned container directive only after BioContainers is confirmed.
- Keep the example close to nf-core module conventions.

Rationale from nf-core documentation:

- `nf-core modules create` is the standard path for local or shared modules.
- If Bioconda metadata exists, nf-core tooling can use it to populate container
  information.

Reference:

- https://nf-co.re/docs/nf-core-tools/cli/modules/create

### Snakemake Wrapper Starter

Polish the wrapper starter so it is closer to official Snakemake wrapper style.

Scope:

- Add `environment.yaml` using the Bioconda package.
- Keep `params.extra` for optional flags.
- Document local usage.
- Document the future path to an upstream Snakemake wrapper.

Rationale from Snakemake documentation:

- Wrappers are the most fine-grained Snakemake modularization layer.
- Wrapper versions improve reproducibility by pinning behavior.
- Wrappers can use Conda integration to deploy software dependencies.

Reference:

- https://snakemake.readthedocs.io/en/v9.1.8/snakefiles/modularization.html

### Benchmark And Evidence Page

Add a concise evidence document for users and maintainers.

Scope:

- Synthetic fixtures showing duplicate IDs, invalid characters, high N, and GC
  outliers.
- Runtime and memory table for tiny and medium fixtures.
- A section explaining what FastaGuard catches before QUAST, BUSCO, and
  BlobToolKit.
- At least one real public FASTA benchmark later if a stable source is chosen.

This page should turn product claims into inspectable evidence.

## Data Contract

v0.2 should strengthen the JSON contract without breaking v0.1 consumers.

### Schema Version

Keep tool version and schema version separate.

Suggested values:

```text
tool version: 0.2.0
schema version: 0.2.0
```

### Provenance

Add richer provenance for workflow engines and future tool agents.

Recommended new fields:

```json
{
  "provenance": {
    "command": "fastaguard sample.fa --profile assembly ...",
    "started_at": "2026-05-23T12:34:56Z",
    "completed_at": "2026-05-23T12:34:57Z",
    "duration_ms": 842,
    "input_size_bytes": 1842912
  }
}
```

`input_sha256` should be considered but may remain optional or behind a flag if
checksum cost is a concern for huge FASTA files.

### Finding Taxonomy

Add fields that make findings easier for pipelines and agents to interpret:

```json
{
  "category": "composition",
  "confidence": "moderate",
  "requires_followup_tool": true
}
```

These fields distinguish hard validation failures from biological suspicion.

### Routing Hints

Upgrade `machine_summary` with compact routing hints:

```json
{
  "routing_hints": [
    {
      "condition": "composition_anomaly",
      "suggested_route": "contamination_or_cobiont_triage",
      "requires_external_database": true
    }
  ]
}
```

Routing hints are not commands. They are structured suggestions that workflow
engines and agents can inspect safely.

### Compatibility Rule

v0.2 may add fields, but should not rename or remove existing v0.1 fields unless
there is a clear contract reason. Existing JSON Schema and golden-report tests
should protect this rule.

## Architecture Notes

The current Rust modules already have good boundaries:

- `parser.rs`: streaming FASTA parsing
- `metrics.rs`: record-level and assembly-level metrics
- `findings.rs`: findings and verdict logic
- `models.rs`: JSON report contract
- `report/*`: rendered outputs

v0.2 should preserve these boundaries.

Suggested implementation shape:

- Add record-level outlier signal fields to metrics or a focused stats helper.
- Keep z-score and length distribution logic isolated in `src/stats/outliers.rs`.
- Promote selected plot flags into findings in `findings.rs`.
- Extend `models.rs` for provenance, finding taxonomy, and routing hints.
- Update schema, finding catalog, golden JSON, HTML, TSV, and MultiQC output as
  derived views.

## Testing Strategy

### Rust Core Tests

- GC outlier detection on controlled fixtures.
- Length outlier detection on skewed fixtures.
- Composite anomaly priority when multiple signals affect the same record.
- No false FAIL from outlier warnings unless `--fail-on` requests it.
- HTML contains the new outlier evidence.
- TSV and MultiQC summaries expose outlier counts where useful.

### Contract Tests

- New schema validates all golden JSON reports.
- Existing v0.1 fields remain present.
- `--schema`, `--finding-catalog`, and `--explain-finding` include new findings.
- Provenance fields are present and deterministic enough for tests.
- New finding taxonomy fields validate against schema.

### Integration Tests

- MultiQC plugin parses multiple FastaGuard reports.
- MultiQC strict mode works.
- Snakemake wrapper includes a Bioconda environment file.
- nf-core starter docs match current Bioconda and container status.

### Packaging And Release Checks

Run the usual gates before release:

```bash
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
python3 -m unittest discover tests/python -v
git diff --check
```

Also run targeted checks:

```bash
multiqc --strict examples/reports
docker build -t fastaguard:local .
mamba install -c conda-forge -c bioconda fastaguard
```

## Release Criteria

v0.2 is ready when:

- Assembly outlier findings are implemented and explainable.
- JSON Schema and finding catalog are updated.
- Golden reports validate against the new schema.
- MultiQC plugin works on multiple reports.
- Docs clearly state where FastaGuard fits before QUAST, BUSCO, BlobToolKit,
  and related tools.
- Benchmark/evidence page exists.
- Bioconda install remains the recommended install path.
- BioContainers status is confirmed or honestly documented as pending.

## Recommended Next Implementation Order

1. Commit the current Bioconda-live documentation update.
2. Confirm BioContainers status.
3. Add Snakemake `environment.yaml` and polish nf-core/Snakemake docs.
4. Harden MultiQC plugin and strict-mode verification.
5. Add outlier finding tests.
6. Implement GC, length, and composite anomaly findings.
7. Extend provenance, finding taxonomy, routing hints, schema, and golden
   reports.
8. Add benchmark/evidence page.
9. Run full release checks and prepare v0.2.0 release notes.

## Implementation Defaults

Use these defaults for the v0.2 implementation plan:

- Do not compute `input_sha256` by default in v0.2. Add command, timestamps,
  duration, and input size first. Revisit checksums later behind an explicit
  flag if needed.
- Use a robust percentile/IQR-style approach for `length_outliers` instead of a
  plain z-score. Assembly length distributions are often skewed.
- Implement `composite_anomalies` as a separate finding when a record has two or
  more suspicious signals. Also keep record-level flags in plot data.
- Keep the MultiQC work as a packaged plugin for v0.2. Consider upstream
  MultiQC submission after more users test the module.
