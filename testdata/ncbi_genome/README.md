# NCBI genome policy corpus

This directory contains small, deterministic FASTA fixtures for FastaGuard's
`ncbi_genome` submission-policy checks. All sequence content is synthetic; no
raw public sequence data is included. Except for `seqid_unicode.fa`, every
fixture contains ASCII bytes only.

| Fixture | Purpose |
| --- | --- |
| `contig_199.fa` | Exercises the invalid 199-base record boundary. |
| `contig_200.fa` | Exercises the valid 200-base record boundary. |
| `contig_201.fa` | Exercises a valid record immediately above the boundary. |
| `duplicate_first_token.fa` | Exercises repeated first whitespace-delimited identifiers with different descriptions. |
| `duplicate_full_id.fa` | Exercises repeated identifiers with identical full headers. |
| `invalid_sequence_symbol.fa` | Exercises a non-IUPAC sequence symbol in an otherwise valid 200-base record. |
| `seqid_49.fa` | Exercises the valid 49-byte SeqID boundary. |
| `seqid_50.fa` | Exercises the invalid 50-byte SeqID boundary. |
| `seqid_51.fa` | Exercises a SeqID above the invalid boundary. |
| `seqid_allowed_ascii.fa` | Exercises every allowed ASCII punctuation character in the first-token SeqID. |
| `seqid_allowed_chars.fa` | Preserves the original allowed-character coverage fixture used by the CLI suite. |
| `seqid_disallowed_ascii.fa` | Exercises a disallowed ASCII percent sign in the first-token SeqID. |
| `seqid_invalid_chars.fa` | Preserves the original disallowed-character coverage fixture used by the CLI suite. |
| `seqid_unicode.fa` | Exercises NCBI SeqID rejection for a non-ASCII header token. |
| `terminal_n_prefix.fa` | Exercises a leading terminal `N`. |
| `terminal_n_suffix.fa` | Exercises a trailing terminal `N`. |
| `terminal_ns.fa` | Preserves the original combined leading-and-trailing terminal-`N` fixture used by the CLI suite. |

`policy_cases.json` is the machine-readable source of expected FastaGuard
findings and continuation decisions. Cases marked `table2asn_fasta_overlap`
cover policy boundaries suitable for optional differential comparison. Cases
marked `fastaguard_structural_extension` cover FastaGuard's additional FASTA
validity and identifier checks.

This corpus is a FASTA-level preflight test. It does not test or guarantee
repository acceptance, annotation validity, taxonomy, contamination status, or
submission metadata.
