# Clean FASTA qualification corpus

This directory contains small, synthetic, known-good FASTA inputs for the NCBI
submission-readiness gate. Every fixture is enumerated in `clean_cases.json` and
must produce an empty `gate.blocking_findings` list with `--gate submission
--submission-target ncbi`.

This corpus is deliberately separate from `testdata/ncbi_genome`, which contains
adverse policy cases. The fixtures here establish that ordinary safe identifiers,
valid sequence characters, and records at or above the NCBI policy's 200-base
minimum are not blocked. They are contract fixtures, not biological references
or evidence of repository acceptance.
