# FastaGuard v0.4 Design: Preflight Readiness And Compare Mode

## Summary

FastaGuard v0.4 should make the product's pre-QC position unmistakable:

```text
FastaGuard is the FASTA readiness gate before interpretive QC tools run.
```

v0.3 made single assembly FASTA checks credible for pipelines. v0.4 should make
FastaGuard useful in the daily workflow where bioinformaticians handle many
FASTA files, not just one. The release should add compare mode and a preflight
readiness matrix that tells users whether each FASTA is ready for indexing,
mapping, BLAST database creation, annotation, submission, and deeper assembly QC.

Release theme:

```text
FastaGuard v0.4: Preflight Readiness + Compare Mode
```

Product promise:

```text
Rank, gate, and route many assembly FASTA files before QUAST, BUSCO,
BlobToolKit, CheckM, annotation, or submission.
```

This release should remain assembly-first and database-free by default. It
should not add transcriptome, protein, or reference-panel profiles yet.

## Goals

- Add a `compare` command for many assembly FASTA files.
- Add a preflight readiness matrix to single-file and compare outputs.
- Make readiness decisions machine-readable with stable IDs and evidence.
- Add checks that catch common pre-QC failures before indexing, BLAST database
  creation, annotation, submission, or heavier assembly QC.
- Preserve v0.3 report behavior for existing single-file users.
- Produce a value benchmark document with measured runtime, memory, and
  downstream-work-avoided scenarios.

## Non-Goals

- Do not add taxonomy databases, marker-gene databases, aligners, read mapping,
  or internet requirements to default runs.
- Do not claim biological completeness, assembly correctness, or contamination
  confirmation.
- Do not replace official NCBI, ENA, QUAST, BUSCO, BlobToolKit, CheckM,
  sourmash, Kraken, samtools, BLAST, or annotation validators.
- Do not add transcriptome, protein, or reference-panel profiles in v0.4.
- Do not add an LLM/chat feature.
- Do not introduce a workflow engine dependency.

## Product Position

The useful distinction is:

```text
FastaGuard checks whether a FASTA is ready for tools.
Downstream QC tools interpret what the assembly means biologically.
```

FastaGuard should own analysis readiness. That includes file parsing, FASTA
structure, sequence alphabet, identifier safety, indexing safety, basic assembly
composition signals, submission-style advisories, and cohort triage.

Recommended public message:

```text
The FASTA readiness gate before assembly QC.
```

Recommended slogan:

```text
Validate the FASTA. Explain the risk. Route the workflow.
```

Avoid:

```text
FastQC for FASTA
```

That phrase makes the tool sound smaller than the machine-readable preflight
contract it is trying to become.

## User Workflows

### Single Assembly Gate

Existing v0.3 behavior should continue:

```bash
fastaguard sample.fa \
  --profile assembly \
  --gate pipeline \
  --out fastaguard_report.html \
  --json fastaguard.json \
  --tsv fastaguard.tsv \
  --multiqc fastaguard_mqc.json
```

v0.4 should add readiness evidence to that report without changing the basic
command shape.

### Cohort Triage

Add a compare command:

```bash
fastaguard compare assemblies/*.fa \
  --profile assembly \
  --gate pipeline \
  --out cohort_report.html \
  --json cohort.json \
  --tsv cohort.tsv \
  --multiqc fastaguard_compare_mqc.json
```

The compare command should answer:

- Which FASTA files fail the preflight gate?
- Which pass but deserve follow-up inspection?
- Which samples are outliers compared with the cohort?
- Which downstream tools are reasonable next steps?
- Which failures are likely to waste expensive jobs if ignored?

### Pipeline Use

Workflow authors should be able to run FastaGuard before expensive processes and
route on stable fields:

```text
readiness.overall.status
readiness.categories[].status
gate.status
gate.blocking_findings
machine_summary.recommended_next_tools
```

Pipelines should not parse logs or HTML.

## Feature Scope

### Compare Command

Add a subcommand:

```text
fastaguard compare <FASTA>...
```

Minimum behavior:

- Accept two or more input FASTA paths.
- Support uncompressed and gzipped FASTA inputs using the existing parser path.
- Run the same assembly analysis as single-file mode for each input.
- Preserve deterministic ordering based on input path order unless an explicit
  sort is requested in a future release.
- Produce combined JSON, TSV, HTML, and MultiQC-compatible outputs.
- Return an exit code based on the worst per-sample status:
  - `0` when all samples pass
  - `1` when one or more samples warn but none fail
  - `2` when one or more samples fail the active gate
  - `3` for invalid command, input, or tool errors that prevent comparison

The compare command should not require all inputs to share the same number of
records, total length, or organism. It is a triage view, not a formal benchmark.

### Compare JSON Contract

Add a new compare report shape:

```json
{
  "schema_version": "0.4.0",
  "report_type": "compare",
  "tool": {
    "name": "fastaguard",
    "version": "0.4.0"
  },
  "input": {
    "profile": "assembly",
    "sample_count": 3
  },
  "summary": {
    "sample_count": 3,
    "pass_count": 1,
    "warn_count": 1,
    "fail_count": 1
  },
  "samples": [
    {
      "sample_id": "assembly_a",
      "input_path": "assemblies/a.fa",
      "verdict": "PASS",
      "gate_status": "PASS",
      "readiness_status": "PASS",
      "sequence_count": 42,
      "total_length": 5123456,
      "n50": 240000,
      "gc_percent": 50.8,
      "n_percent": 0.1,
      "finding_ids": [],
      "readiness_blockers": [],
      "recommended_next_tools": ["QUAST", "BUSCO"]
    }
  ],
  "cohort_findings": [
    {
      "id": "cohort_total_length_outliers",
      "severity": "major",
      "affected_count": 1,
      "evidence": {
        "samples": [
          {
            "sample_id": "assembly_c",
            "total_length": 8723456,
            "reason": "total length is high relative to cohort distribution"
          }
        ]
      }
    }
  ]
}
```

Rules:

- `report_type` distinguishes single-file reports from compare reports.
- `samples[]` should contain compact per-sample summaries, not full nested
  copies of every single-file report.
- Full per-sample JSON files should not be generated by default in v0.4. The
  compare report should stay compact. A later `--write-sample-reports` option
  can add full per-sample artifacts if users need them.
- Cohort findings should be deterministic and based only on local metrics.

### Compare TSV

The compare TSV should be a table with one row per sample.

Minimum columns:

```text
sample_id
input_path
verdict
gate_status
readiness_status
sequence_count
total_length
n50
n90
gc_percent
n_percent
duplicate_id_count
invalid_sequence_count
high_n_sequence_count
tiny_contig_count
max_gap_run
gc_outlier_count
length_outlier_count
finding_count
readiness_blockers
recommended_next_tools
input_sha256
```

This table is a major adoption surface. It should be stable, simple, and easy to
join with workflow metadata.

### Compare HTML

The compare HTML should be self-contained and should not rely on external
JavaScript or CDNs.

Required sections:

1. Verdict summary
2. Sample table
3. Readiness matrix
4. Cohort metric plots
5. Cohort findings
6. Suggested next tools

Minimum plots:

- total length by sample
- N50 by sample
- GC% by sample
- N% by sample
- sequence count by sample

Inline SVG is enough for v0.4. The goal is scanability, not rich interactivity.

### MultiQC Output

The compare command should produce standard MultiQC custom content compatible
with the existing FastaGuard approach.

The MultiQC data should be keyed by sample ID and include:

- verdict
- gate status
- readiness status
- sequence count
- total length
- N50
- GC%
- N%
- duplicate ID count
- invalid sequence count
- high-N sequence count
- finding count
- readiness blockers

The default compare MultiQC filename should be:

```text
fastaguard_compare_mqc.json
```

Single-file default remains:

```text
fastaguard_mqc.json
```

## Preflight Readiness Matrix

Add a readiness layer to single-file and compare reports.

The readiness layer should answer:

```text
Which downstream surfaces is this FASTA ready for?
```

Proposed JSON shape:

```json
"readiness": {
  "overall": {
    "status": "FAIL",
    "blockers": ["index.duplicate_first_token_ids", "alphabet.invalid_chars"]
  },
  "categories": [
    {
      "id": "file",
      "label": "File readiness",
      "status": "PASS",
      "findings": []
    },
    {
      "id": "index",
      "label": "Index readiness",
      "status": "FAIL",
      "findings": ["duplicate_ids", "duplicate_first_token_ids"]
    }
  ]
}
```

Statuses:

```text
PASS
WARN
FAIL
```

Categories:

```text
file
structure
alphabet
index
assembly
submission
cohort
machine
```

`cohort` applies only to compare reports. `machine` describes whether the
report contract is complete enough for workflow routing.

Readiness is not a second verdict system. It is a routing view over findings,
thresholds, and output-contract availability. The single report verdict and gate
status remain the authoritative pass/warn/fail decision.

## New Findings

### Duplicate First-Token IDs

Finding ID:

```text
duplicate_first_token_ids
```

Severity:

```text
critical
```

Rationale:

Many tools treat the first whitespace-delimited token in a FASTA header as the
record name. Two headers can look different to humans but collide in downstream
indexes if their first token is the same.

Example:

```text
>contig1 length=1000
>contig1 length=2000
```

This should be blocking for `--gate pipeline`.

### Blank Or Unsafe IDs

Finding ID:

```text
unsafe_ids
```

Severity:

```text
major
```

Scope:

- blank ID after `>`
- IDs with leading or trailing whitespace
- IDs containing control characters
- IDs containing path-like separators that commonly confuse scripts

This should be warning by default. A stricter submission gate can fail it later.

### Long Headers

Finding ID:

```text
long_headers
```

Severity:

```text
minor
```

Default threshold:

```text
header length > 200 characters
```

Rationale:

Long headers often embed coverage, coordinates, tool metadata, or free text that
can break brittle downstream scripts. This should be advisory, not blocking.

### Reserved Header Characters

Finding ID:

```text
reserved_header_chars
```

Severity:

```text
minor
```

Initial reserved characters:

```text
| ; " ' ` < > \t
```

This should be advisory. Some ecosystems use pipes legitimately, so the report
must explain that the concern is tool compatibility, not invalid FASTA.

### Terminal Ns

Finding ID:

```text
terminal_ns
```

Severity:

```text
major
```

Scope:

- records that start with one or more `N`
- records that end with one or more `N`

Rationale:

Terminal Ns can signal untrimmed ambiguous sequence or submission-readiness
problems. NCBI genome submission guidance explicitly advises no Ns at the ends
of submitted sequences.

This should be advisory in normal assembly profile and blocking only under a
future submission-focused gate.

### Gap Pattern Warnings

Finding ID:

```text
gap_pattern_warnings
```

Severity:

```text
minor
```

Scope:

- many identical N-run lengths, especially exactly 100 Ns
- mixed short ambiguity runs and long scaffold-gap runs
- records with multiple long gap runs

Rationale:

This does not prove a problem. It helps route users toward gap handling,
AGP/table2asn decisions, or submission validators when needed.

### Assembly Size Out Of Expected Range

Finding ID:

```text
expected_size_outlier
```

Severity:

```text
major
```

v0.4 default:

Do not call external APIs. Support only user-provided expected size:

```bash
fastaguard sample.fa --expected-size 5mb --expected-size-tolerance 0.25
```

or:

```bash
fastaguard compare *.fa --expected-size-column expected_size.tsv
```

This should compare ungapped assembly length against the expected range. It is
advisory by default and should route users to official NCBI expected genome size
checks or deeper contamination/completeness analysis.

### Cohort Metric Outliers

Finding IDs:

```text
cohort_total_length_outliers
cohort_gc_outliers
cohort_n_percent_outliers
cohort_sequence_count_outliers
cohort_n50_outliers
```

Severity:

```text
minor or major depending on signal strength
```

Scope:

Compare mode only. These findings should rank unusual samples relative to the
batch. They should not automatically fail the pipeline unless the user includes
them with `--fail-on`.

## Gate Behavior

The v0.3 `--gate pipeline` preset should remain conservative.

Add to the default blocking set:

```text
duplicate_first_token_ids
```

Keep blocking:

```text
duplicate_ids
invalid_chars
invalid_fasta_structure
high_n_rate
```

Do not block by default:

```text
unsafe_ids
long_headers
reserved_header_chars
terminal_ns
gap_pattern_warnings
expected_size_outlier
cohort_*_outliers
gc_outliers
length_outliers
composite_anomalies
```

Rationale:

Default blocking should mean the FASTA is likely unsafe for routine downstream
tool execution. Advisory findings should support inspection and routing without
creating false drama.

## CLI Design

### Existing Single-File Command

No breaking changes.

Add optional flags:

```text
--expected-size <SIZE>
--expected-size-tolerance <FRACTION>
```

Readiness should be included by default in JSON and HTML outputs once the schema
version moves to `0.4.0`. Do not add a `--readiness` flag in v0.4; a flag that
only confirms default behavior would make the CLI noisier without adding value.

### New Compare Command

```text
fastaguard compare <FASTA>... [OPTIONS]
```

Options should mirror the single-file command where practical:

```text
--profile assembly
--gate pipeline
--fail-on <IDS>
--out <HTML>
--json <JSON>
--tsv <TSV>
--multiqc <JSON>
--min-contig-length <INT>
--high-n <FRACTION>
--high-global-n <FRACTION>
--max-gap-run <INT>
--gc-outlier-zscore <FLOAT>
--expected-size <SIZE>
--expected-size-tolerance <FRACTION>
```

Potential later flags, not required for v0.4:

```text
--sample-sheet <TSV>
--sample-id-regex <REGEX>
--write-sample-reports
--threads <N>
```

## Data Model Changes

Add models for:

- readiness overall status
- readiness categories
- readiness category findings
- compare report
- compare sample row
- cohort findings

The existing single-file `FastaguardReport` should gain:

```text
readiness
```

The schema should be versioned to:

```text
0.4.0
```

Backward compatibility:

- Existing v0.3 users should still receive the same core fields.
- New fields may be added, but existing field names and meanings should not be
  changed.
- Existing output filenames should remain unchanged for single-file mode.

## Implementation Notes

### Metrics

Extend sequence summaries with:

- first token ID
- header length
- unsafe ID flags
- reserved header character flags
- terminal N counts
- gap run length histogram or compact gap pattern summary
- ungapped length

Do not store full sequence strings.

### Performance

v0.4 should preserve the "seconds-level preflight" promise.

Implementation should:

- stream input records
- avoid loading whole FASTA files into memory
- keep bounded evidence lists
- cap plotted points as v0.2/v0.3 already do
- run compare samples sequentially first unless parallelism is simple and
  deterministic

Parallel compare can come later if needed. Deterministic output is more
important than squeezing early speed.

### Evidence Ordering

Evidence records should be deterministic:

- blocking findings first
- stronger signal before weaker signal
- longer records before shorter records when biology relevance is similar
- ID as final tie-breaker

### Size Parsing

`--expected-size` should accept:

```text
5000000
5kb
5mb
5gb
5k
5m
5g
```

Use decimal units for biological readability:

```text
1kb = 1,000 bases
1mb = 1,000,000 bases
1gb = 1,000,000,000 bases
```

Report the parsed value in provenance.

## Report Design

### Single HTML

Add a "Readiness" section near the top after verdict/gate:

```text
Readiness
File: PASS
Structure: PASS
Alphabet: PASS
Index: FAIL
Assembly: WARN
Submission: WARN
Machine: PASS
```

Each category should include short action language:

```text
Index readiness failed because duplicate first-token IDs can make faidx,
BLAST databases, mapping references, or annotation joins ambiguous.
```

### Compare HTML

The compare report should show:

- top-line counts: PASS/WARN/FAIL
- sortable-looking static table, ordered by input order
- readiness matrix with samples as rows and categories as columns
- cohort metric plots
- cohort findings
- next-tool suggestions

No external JavaScript.

## Documentation

Add or update:

```text
docs/preflight-readiness.md
docs/compare-mode.md
docs/value-benchmark.md
README.md
docs/tool-landscape.md
docs/benchmarking.md
docs/releases/v0.4.0.md
```

`docs/preflight-readiness.md` should explain:

- what FastaGuard checks before QC tools
- what FastaGuard cannot conclude
- readiness categories
- why index readiness matters
- why submission readiness is advisory
- how to route to QUAST, BUSCO, BlobToolKit, CheckM, samtools, BLAST, or
  official validators

`docs/value-benchmark.md` should include measured local benchmarks:

```text
10 Mbp synthetic FASTA: about 0.51 seconds, about 17 MB RSS
100 Mbp synthetic FASTA: about 0.98 seconds, about 50 MB RSS
```

It should also include careful value scenarios:

```text
FastaGuard costs seconds; it can save minutes, CPU-hours, or days when it blocks
a bad FASTA before heavier QC starts.
```

## Tests

### Rust Unit And Integration Tests

Add tests for:

- duplicate first-token IDs
- unsafe IDs
- long headers
- reserved header characters
- terminal Ns
- gap pattern warnings
- expected-size parsing
- expected-size outlier findings
- readiness matrix status aggregation
- `--gate pipeline` includes `duplicate_first_token_ids`
- compare command rejects fewer than two inputs
- compare JSON shape and schema
- compare TSV columns
- compare MultiQC custom content
- compare exit codes for pass, warn, fail, and tool error
- deterministic compare output order

### Golden Reports

Regenerate golden JSON for:

```text
valid_assembly
problem_assembly
invalid_empty_record
```

Add compare golden fixtures:

```text
compare_mixed_status.json
compare_all_pass.json
```

### Python Tests

Update adoption asset tests to check:

- compare docs exist
- readiness docs explain boundaries
- value benchmark docs contain measured local numbers
- nf-core/Snakemake examples mention compare mode as optional/starter
- MultiQC custom content accepts compare output

### Verification Gates

Run:

```bash
python3 -m unittest discover tests/python -v
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
git ls-files | xargs perl -ne 'print "$ARGV:$.:$_" if /[ \t]$/'
```

Optional:

```bash
cargo build --release --locked
python3 scripts/benchmark_large_fasta.py \
  --records 10000 \
  --length 1000 \
  --binary target/release/fastaguard \
  --out-dir target/benchmarks/v0.4-10mbp
```

## Release Criteria

v0.4 is ready when:

- compare mode works for many FASTA files
- single-file JSON includes readiness without breaking existing core fields
- compare JSON and TSV are stable enough for workflow authors
- MultiQC can consume compare output
- readiness findings are documented in the finding catalog
- local benchmark/value docs are committed
- all tests and lint gates pass
- release notes clearly say FastaGuard remains preflight QC, not downstream
  biological interpretation

## Open Questions

1. Should compare mode write full per-sample reports by default, or only compact
   cohort outputs?

   Recommendation: compact cohort outputs by default. Add
   `--write-sample-reports` later if users ask for it.

2. Should submission readiness be a separate gate in v0.4?

   Recommendation: no. Add submission advisories now, then consider
   `--gate submission` after users validate the thresholds.

3. Should compare mode use parallel processing immediately?

   Recommendation: no. Start sequential and deterministic. Optimize after the
   contract is trusted.

4. Should expected genome size call the NCBI API?

   Recommendation: no default internet access. Support user-provided expected
   size in v0.4; document official NCBI checks as follow-up.

## Recommended Implementation Order

1. Add tests and models for readiness categories.
2. Add new single-file findings and evidence.
3. Add readiness output to single-file JSON/HTML/TSV/MultiQC.
4. Add compare command with compact per-sample summaries.
5. Add cohort findings and compare plots.
6. Add docs, examples, release notes, and value benchmark.
7. Regenerate schema and golden reports.
8. Run full verification gates.

This order keeps the contract stable before building the larger compare report.
