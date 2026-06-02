# Compare Mode

```bash
fastaguard compare assemblies/*.fa \
  --profile assembly \
  --gate pipeline \
  --out cohort_report.html \
  --json cohort.json \
  --tsv cohort.tsv \
  --multiqc fastaguard_compare_mqc.json
```

Compare mode is a starter cohort triage layer. It ranks many FASTA files by
preflight status, readiness status, structural metrics, composition metrics, and
cohort outliers.

Use it when a workflow has many candidate assemblies and needs a first-pass
table before sending selected samples to QUAST, BUSCO, BlobToolKit, CheckM,
official validators, annotation, or other interpretive QC tools.

The compare report should stay database-free by default. It summarizes and
aggregates FastaGuard's own FASTA-level findings; it does not assign taxonomy,
prove completeness, or decide biological correctness.

Expected starter outputs:

- `cohort_report.html` for human review
- `cohort.json` as the source of truth
- `cohort.tsv` for spreadsheet and workflow filtering
- `fastaguard_compare_mqc.json` for MultiQC-compatible cohort summaries
