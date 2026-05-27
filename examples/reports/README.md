# Example Reports

These tiny examples show the complete v0.3 output contract without requiring large datasets.

## Assembly Pass

Generated from `testdata/valid_assembly.fa` with `--min-contig-length 1`.

- `assembly_pass/fastaguard.json`
- `assembly_pass/fastaguard.tsv`
- `assembly_pass/fastaguard_mqc.json`
- `assembly_pass/fastaguard_report.html`

Regenerate:

```bash
cargo run -- testdata/valid_assembly.fa \
  --min-contig-length 1 \
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
cargo run -- testdata/problem_assembly.fa \
  --out examples/reports/assembly_fail/fastaguard_report.html \
  --json examples/reports/assembly_fail/fastaguard.json \
  --tsv examples/reports/assembly_fail/fastaguard.tsv \
  --multiqc examples/reports/assembly_fail/fastaguard_mqc.json
```

The command exits with code `2` because this example contains critical FASTA-level blockers. In v0.3, inspect the JSON `gate` object to separate blocking findings from advisory findings.
