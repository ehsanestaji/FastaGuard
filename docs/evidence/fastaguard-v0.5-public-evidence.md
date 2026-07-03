# FastaGuard v0.5 Public Evidence

This page defines the v0.5 public evidence workflow. The goal is to show that
FastaGuard is cheap to run before expensive downstream QC and that its gate
outputs can be audited against exact input bytes.

The public assembly list lives in:

```text
docs/evidence/public_assemblies.json
```

Each manifest case declares:

- `accession`
- `evidence_role`
- `expected_scale`
- `downstream_route`
- `source_url`

These fields are copied into `evidence_summary.json` and
`evidence_summary.tsv` by `scripts/collect_evidence.py`.

## Local Smoke Run

Run the offline cases first. This does not download public data:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.5-local \
  --local-only
```

The local smoke run covers:

| Case | Evidence role | Why it matters |
| --- | --- | --- |
| `synthetic_valid` | offline valid FASTA smoke case | proves the evidence workflow can produce PASS reports |
| `problem_fixture` | local blocker case | proves duplicate IDs, invalid characters, high-N records, and tiny contigs are visible before downstream tools |
| `gzipped_valid` | offline gzipped FASTA input smoke case | proves compressed FASTA input is accepted by the evidence workflow |

## Public Run

Install the NCBI Datasets CLI, then run:

```bash
cargo build --release --locked
python3 scripts/collect_evidence.py \
  --binary target/release/fastaguard \
  --out-dir target/evidence/v0.5-public
```

The public run downloads the accessions in
`docs/evidence/public_assemblies.json` and writes one report directory per
case. Keep downloaded FASTA files and full HTML/JSON reports under `target/`.
Do not commit large public FASTA files.

## Summary Table

The compact TSV summary is the artifact to copy into release notes, README
updates, or adoption discussions:

| Column | Meaning |
| --- | --- |
| `id` | Evidence case ID |
| `label` | Reader-facing assembly label |
| `category` | Coarse case category |
| `source` | `local` or `public_ncbi` |
| `accession` | Public assembly accession when available |
| `source_url` | Public source page when available |
| `evidence_role` | Why this case belongs in the evidence set |
| `expected_scale` | Approximate size class |
| `downstream_route` | What FastaGuard should route toward after preflight |
| `elapsed_seconds` | FastaGuard runtime for the local machine |
| `verdict` | PASS, WARN, or FAIL |
| `gate_status` | Pipeline gate status |
| `gate_blocking_findings` | Findings that block downstream workflow steps |
| `input_sha256` | Exact input checksum for auditability |
| `sequence_count`, `total_length`, `n50`, `n90` | FASTA-level structural metrics |
| `finding_count`, `top_findings` | Compact finding summary |

## Interpretation

This evidence is FASTA-level preflight evidence. It is not biological completeness,
not contamination confirmation, not annotation validation, and not repository
acceptance. Passing FastaGuard means the FASTA passed FastaGuard's local
contract for the selected gate.

Use the summary to show routing:

- FAIL with blocking findings: fix the FASTA before QUAST, BUSCO, BlobToolKit,
  CheckM, annotation, or submission validators.
- PASS or WARN without blockers: continue to downstream tools appropriate for
  the biological question.
- WARN with advisory findings: continue if policy allows, but route the finding
  IDs to the relevant follow-up tool.
