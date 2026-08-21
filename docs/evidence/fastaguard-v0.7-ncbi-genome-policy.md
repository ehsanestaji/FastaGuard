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

The controlled macOS observation for this evidence record used table2asn
`1.29.324` from the official NCBI `mac.table2asn.gz` URL. Its compressed SHA-256
was observed as
`348b9d5f1b05065f4e5e61b6c2350e5ff044de2d6dc14765be6ac02c00e59341` and
was verified against a second download before execution. The NCBI directory did
not publish an independent digest alongside the binary, so this locally
established value pins the repeated download but is not an independent
authenticity attestation.

## Result Interpretation

Each completed case records the FastaGuard report exit code, findings, and gate
continuation decision. A completed FastaGuard report must exit zero; a policy
blocker is represented by `fastaguard_can_continue: false`, not a process
failure. A nonzero FastaGuard exit is a verifier error even if a report file was
written.

The table2asn category is derived conservatively from both the process result
and its documented `.val` or `.stats` validation artifact. The controlled run
uses a local synthetic submission template so missing citation metadata does
not turn every FASTA boundary case into an unrelated rejection:

- `accepted` requires exit code zero and a readable artifact with no validation
  error or rejection;
- `rejected` requires a readable artifact that records a fatal, critical,
  error, or rejection result; table2asn exit code `1` is accepted only with
  that rejecting artifact because current binaries use it for some invalid
  FASTA inputs; and
- `tool_error` covers every other nonzero exit, timeout, execution failure,
  missing artifact, or unparseable artifact.

`matches_expected` compares the observed table2asn category with the pinned
manifest expectation. `fastaguard_policy_agreement` separately compares that
category with FastaGuard's gate decision. The summary reports both dimensions;
a current table2asn expectation can match while its acceptance behavior remains
stricter or looser than FastaGuard's documented policy.

For table2asn `1.29.324`, all 14 observed categories match the committed
expectations. Ten also agree with FastaGuard's gate. Four are explicit policy
disagreements: table2asn accepts `contig_199`, `seqid_50`,
`seqid_disallowed_ascii`, and `seqid_invalid_chars`, while FastaGuard blocks
them under its documented NCBI FASTA preflight policy. These observations do
not weaken the FastaGuard findings or gate and do not imply that table2asn
enforces every FastaGuard policy rule.

Optional runs preserve `tool_error` evidence without failing the verifier.
`--require-table2asn` turns any such tool error into a verifier error. These are
local differential categories, not repository decisions. Every table2asn run
uses the same local submission template plus synthetic-organism and genomic-DNA
source modifiers so that the comparison isolates the listed FASTA boundaries;
those fixed values do not test submission metadata. Commands use stable
placeholders for executables and temporary paths so the JSON artifact does not
expose checkout, input, or runner-specific absolute paths.

The comparison cannot prove NCBI acceptance and cannot guarantee submission
readiness. Continue with the complete official submission workflow and review
all validator output for the actual submission package.

Official references:

- [NCBI FASTA format guidance](https://www.ncbi.nlm.nih.gov/genbank/fastaformat/)
- [NCBI table2asn guidance](https://www.ncbi.nlm.nih.gov/genbank/table2asn/)
