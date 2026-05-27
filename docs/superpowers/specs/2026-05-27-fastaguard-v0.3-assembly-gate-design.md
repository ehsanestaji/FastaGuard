# FastaGuard v0.3 Design: Evidence And Assembly Gate

## Summary

FastaGuard v0.3 should turn the current assembly preflight checker into a
pipeline-ready gate that workflow authors can add before QUAST, BUSCO,
BlobToolKit, CheckM, annotation, or submission.

Release theme:

```text
FastaGuard v0.3: Evidence And Assembly Gate
```

Product promise:

```text
FastaGuard gives assembly pipelines a fast, explainable PASS/WARN/FAIL gate
before expensive QC.
```

The release should stay assembly-first. It should not add transcriptome,
protein, or reference-panel profiles yet. v0.3 should make the assembly
contract more credible, easier to enforce, and easier to cite.

## Goals

- Add a pipeline gate preset for common assembly preflight behavior.
- Add input checksum provenance so reports can be tied to exact FASTA bytes.
- Make gate decisions machine-readable without requiring log or HTML parsing.
- Improve report language that separates blocking failures from follow-up
  recommendations.
- Run and document public assembly evidence using the existing evidence script.
- Keep the default product fast, deterministic, database-free, and easy to run
  through Bioconda and BioContainers.

## Non-Goals

- Do not add external databases, taxonomy calls, coverage analysis, aligners, or
  internet requirements to the CLI.
- Do not claim biological completeness, assembly correctness, or contamination
  confirmation.
- Do not add transcriptome, protein, reference-panel, or compare mode in v0.3.
- Do not make an LLM summary feature.
- Do not break the existing `--fail-on` mechanism; the gate preset should be a
  convenience layer over explicit behavior.

## Product Position

v0.1 proved the basic assembly preflight contract. v0.2 made the reports more
trustworthy and pipeline-friendly. v0.3 should answer the pipeline author's
practical question:

```text
Can I add this as the first assembly QC gate and trust what it blocks?
```

Recommended public message:

```text
The assembly FASTA gate before expensive QC.
```

## Feature Scope

### Assembly Gate Preset

Add a new CLI option:

```bash
fastaguard sample.fa --profile assembly --gate pipeline
```

Supported values:

```text
none
pipeline
```

Default behavior:

```text
--gate none
```

`--gate pipeline` should encode conservative defaults for workflow engines. It
should fail the run for findings that make downstream assembly QC unreliable:

```text
duplicate_ids
invalid_chars
invalid_fasta_structure
high_n_rate
```

It should not fail on `gc_outliers`, `length_outliers`, or
`composite_anomalies` by default. Those remain follow-up and prioritization
signals unless the user explicitly includes them with `--fail-on`.

If the user supplies both `--gate pipeline` and `--fail-on`, the final failure
set should be the union of the pipeline preset and the explicit finding IDs.
This keeps the preset easy to understand and avoids surprising overrides.

The CLI should reject unknown gate values with a clear tool error.

### Machine-Readable Gate Decision

Add a compact gate decision to the JSON report:

```json
"gate": {
  "mode": "pipeline",
  "status": "FAIL",
  "blocking_findings": ["duplicate_ids", "invalid_chars"],
  "advisory_findings": ["gc_outliers"],
  "fail_on": ["duplicate_ids", "high_n_rate", "invalid_chars", "invalid_fasta_structure"]
}
```

Rules:

- `mode` is `none` or `pipeline`.
- `status` matches the report verdict.
- `blocking_findings` lists triggered finding IDs that are in the active
  failure set.
- `advisory_findings` lists triggered finding IDs that are not in the active
  failure set.
- `fail_on` records the final active failure set after applying the gate preset
  and user-provided `--fail-on`.

This field is intentionally small. Workflow engines and future tool agents
should be able to route from it without reading human prose.

### Provenance Checksums

Add input checksum metadata to provenance:

```json
"input_sha256": "..."
```

The checksum should be computed over the exact input bytes on disk, not the
decompressed FASTA stream. For `.fa.gz` inputs, this means the checksum
identifies the compressed file that was passed to FastaGuard.

The checksum should be enabled by default in v0.3. The implementation should
stream file bytes and must not load the whole FASTA into memory. If the input
cannot be read, normal input error handling should already fail the run before a
report is emitted.

### Threshold Metadata

Keep the existing provenance threshold fields, and add enough gate context for
machines to understand why a finding blocked:

```json
"thresholds": {
  "high_n_sequence_fraction": 0.2,
  "high_global_n_fraction": 0.05,
  "min_contig_length": 200,
  "max_gap_run": 100,
  "gc_outlier_zscore": 3.0
}
```

No new threshold schema is required for v0.3. The key improvement is that the
active `gate.fail_on` set makes threshold-backed blocking behavior explicit.

### Report Language

Update HTML and release-facing docs so users see three classes of outcome:

```text
Blocking: fix before downstream QC.
Advisory: safe to continue, but inspect.
Routing: run a deeper downstream tool if the question matters.
```

Examples:

- duplicate IDs and invalid characters are blocking for `--gate pipeline`
- GC outliers are advisory and may route to BlobToolKit, sourmash, Kraken, or
  related tools
- high N content can be blocking when it exceeds the configured gate threshold

This wording should preserve the core boundary: FastaGuard is preflight QC, not
biological confirmation.

### Public Evidence Pack

Use the existing evidence workflow as the v0.3 proof layer. The default public
manifest should remain small and fast enough to be rerun by maintainers.

v0.3 should commit compact evidence summaries, not downloaded FASTA files,
archives, or full generated reports.

Commit these files when a public run is available:

```text
docs/evidence/v0.3/evidence_summary.json
docs/evidence/v0.3/evidence_summary.tsv
docs/evidence/fastaguard-v0.3-evidence.md
```

The evidence page should include:

- command used
- FastaGuard version and git commit
- platform and date
- public assembly accessions
- input size, sequence count, elapsed seconds
- verdict and top findings
- reminder that FastaGuard is preflight QC, not completeness or contamination
  confirmation

If NCBI Datasets CLI or network access is unavailable during implementation,
the local-only evidence workflow should still be tested and documented. Public
evidence summaries should only be committed after a real public run succeeds.

## CLI And Contract

New CLI:

```text
--gate <none|pipeline>
```

Schema version should become:

```text
0.3.0
```

Cargo package version should become:

```text
0.3.0
```

JSON report additions:

```text
gate
provenance.input_sha256
```

TSV additions:

```text
gate_mode
gate_status
gate_blocking_findings
gate_advisory_findings
input_sha256
```

MultiQC custom content should include compact `gate_mode`, `gate_status`, and
`gate_blocking_findings` values in the existing custom-content table.

HTML report should show the gate decision near the verdict.

## Architecture

The implementation should keep gate policy separate from finding generation.

Recommended units:

- CLI parsing records the requested gate mode.
- A small gate-policy module maps gate mode to default failure IDs.
- Run configuration stores the final `fail_on` set and gate mode.
- Finding generation remains responsible only for detecting findings.
- Report assembly derives the gate decision from triggered findings and active
  failure IDs.
- Provenance computes `input_sha256` with streaming file reads.

This avoids hiding gate behavior inside individual findings and keeps future
gate presets possible.

## Testing

Add focused tests for:

- `--gate pipeline` adds the expected failure IDs.
- `--gate none` preserves existing default behavior.
- `--gate pipeline --fail-on gc_outliers` unions preset and explicit rules.
- unknown gate values are rejected.
- problem fixture reports include a `gate` object with blocking and advisory
  findings.
- valid fixture reports include `gate.mode`, `gate.status`, empty blocking
  findings, and `provenance.input_sha256`.
- gzipped input checksum is computed from the compressed bytes.
- JSON schema validates updated golden reports.
- TSV includes gate and checksum rows.
- HTML includes gate decision language.
- MultiQC output includes gate mode, gate status, and blocking findings.
- evidence script local-only path continues to pass without network access.

Run release gates:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

Optional evidence checks:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3-local \
  --local-only
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.3
```

The public evidence command requires NCBI Datasets CLI and network access.

## Release And Adoption

v0.3 should ship with:

- GitHub release notes
- updated README quickstart for `--gate pipeline`
- updated Nextflow/nf-core and Snakemake examples
- updated output contract documentation
- updated schema and golden fixtures
- local evidence summary
- public evidence summary when available

After the GitHub `v0.3.0` release exists, update the Bioconda recipe and let
the Bioconda update path produce the next BioContainers image. Do not open a
Bioconda update before the public GitHub source archive exists.

## Success Criteria

v0.3 is successful if:

- a pipeline author can copy one command and get a conservative assembly gate
- the JSON report makes the gate decision obvious to machines
- provenance identifies the exact input file with SHA256
- the report still routes to downstream tools without claiming to replace them
- evidence summaries show FastaGuard running on local and public FASTA cases
- all existing v0.2 outputs remain understandable with a clear schema version
  bump
