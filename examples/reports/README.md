# Example Reports

These tiny examples show the current v0.7 output contract without requiring large datasets.

## Assembly Pass

Generated from `testdata/valid_assembly.fa` with `--min-contig-length 1`.

- `assembly_pass/fastaguard.json`
- `assembly_pass/fastaguard.tsv`
- `assembly_pass/fastaguard_mqc.json`
- `assembly_pass/fastaguard_report.html`

Regenerate:

```bash
cargo build --locked
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-08-21T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='target/debug/fastaguard testdata/valid_assembly.fa --min-contig-length 1 --force --out examples/reports/assembly_pass/fastaguard_report.html --json examples/reports/assembly_pass/fastaguard.json --tsv examples/reports/assembly_pass/fastaguard.tsv --multiqc examples/reports/assembly_pass/fastaguard_mqc.json' \
target/debug/fastaguard testdata/valid_assembly.fa \
  --min-contig-length 1 \
  --force \
  --out examples/reports/assembly_pass/fastaguard_report.html \
  --json examples/reports/assembly_pass/fastaguard.json \
  --tsv examples/reports/assembly_pass/fastaguard.tsv \
  --multiqc examples/reports/assembly_pass/fastaguard_mqc.json
```

## Assembly Fail

Generated from `testdata/problem_assembly.fa`. This fixture intentionally contains duplicate IDs, invalid characters, high-N sequence content, tiny contigs, and a long gap run.

- `assembly_fail/fastaguard.json`
- `assembly_fail/fastaguard.tsv`
- `assembly_fail/fastaguard_mqc.json`
- `assembly_fail/fastaguard_report.html`

Regenerate:

```bash
cargo build --locked
FASTAGUARD_PROVENANCE_TIMESTAMP=2026-08-21T00:00:00Z \
FASTAGUARD_PROVENANCE_COMMAND='target/debug/fastaguard testdata/problem_assembly.fa --force --out examples/reports/assembly_fail/fastaguard_report.html --json examples/reports/assembly_fail/fastaguard.json --tsv examples/reports/assembly_fail/fastaguard.tsv --multiqc examples/reports/assembly_fail/fastaguard_mqc.json' \
target/debug/fastaguard testdata/problem_assembly.fa \
  --force \
  --out examples/reports/assembly_fail/fastaguard_report.html \
  --json examples/reports/assembly_fail/fastaguard.json \
  --tsv examples/reports/assembly_fail/fastaguard.tsv \
  --multiqc examples/reports/assembly_fail/fastaguard_mqc.json
```

The command exits with code `0` after writing the FAIL report. Inspect the JSON
`gate` object and readiness categories to separate blocking findings from
advisory findings.
