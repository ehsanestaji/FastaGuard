# FastaGuard v0.7 NCBI Genome Policy Evidence

This evidence check compares FastaGuard's local `ncbi_genome` policy with a
separately installed table2asn executable over small synthetic FASTA fixtures.
It is optional so ordinary development and pull-request checks remain offline.
The committed
[`ncbi-genome-policy.json`](ncbi-genome-policy.json) manifest selects only
policy-corpus entries classified as `table2asn_fasta_overlap`.

## Compared Boundaries

The differential covers these FASTA-only boundaries:

- record lengths immediately below, at, and above 200 bases;
- first-token SeqID length, ASCII, and allowed-character cases; and
- leading or trailing `N` ambiguity at record boundaries.

The evidence excludes submission metadata, annotation, taxonomy, biological
completeness, contamination detection, and every other repository workflow
check. The corpus's duplicate-identifier and invalid-sequence-symbol cases remain
FastaGuard structural extensions and are deliberately absent from the direct
comparison manifest.

## Running the Verifier

Without table2asn, the verifier exits successfully and writes an explicit
unavailable result:

```bash
python3 scripts/verify_ncbi_genome_policy.py \
  --fastaguard target/debug/fastaguard \
  --manifest docs/evidence/ncbi-genome-policy.json \
  --out target/ncbi-policy.json
```

That result has `table2asn_available: false`,
`comparison_performed: false`, and no case results or comparison summary. It
therefore makes no differential comparison claim.

For a controlled external check, supply a table2asn executable and require it:

```bash
python3 scripts/verify_ncbi_genome_policy.py \
  --fastaguard target/debug/fastaguard \
  --table2asn /path/to/table2asn \
  --require-table2asn \
  --manifest docs/evidence/ncbi-genome-policy.json \
  --out target/ncbi-policy.json
```

The manual CI dispatch requires an official NCBI download URL and its SHA-256
digest. The workflow verifies the compressed download before installation and
passes both values into the normalized result as `table2asn_source` provenance.
Normal push and pull-request jobs only exercise the offline manifest and
fake-tool tests.

## Result Interpretation

Each completed case records the FastaGuard report exit code, findings, and gate
continuation decision. A completed FastaGuard report must exit zero; a policy
blocker is represented by `fastaguard_can_continue: false`, not a process
failure. A nonzero FastaGuard exit is a verifier error even if a report file was
written.

The table2asn category is derived conservatively from both the process result
and its documented `.val` or `.stats` validation artifact:

- `accepted` requires exit code zero and a readable artifact with no validation
  error or rejection;
- `rejected` requires exit code zero and a readable artifact that records a
  validation error or rejection; and
- `tool_error` covers every nonzero exit, timeout, execution failure, missing
  artifact, or unparseable artifact.

Optional runs preserve `tool_error` evidence without failing the verifier.
`--require-table2asn` turns any such tool error into a verifier error. These are
local differential categories, not repository decisions. Every table2asn run
uses the same synthetic-organism and genomic-DNA source modifiers so that the
comparison isolates the listed FASTA boundaries; those fixed modifiers do not
test submission metadata. Commands use stable placeholders for executables and
temporary paths so the JSON artifact does not expose checkout, input, or
runner-specific absolute paths.

The comparison cannot prove NCBI acceptance and cannot guarantee submission
readiness. Continue with the complete official submission workflow and review
all validator output for the actual submission package.

Official references:

- [NCBI FASTA format guidance](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/)
- [NCBI table2asn guidance](https://www.ncbi.nlm.nih.gov/genbank/table2asn/)
