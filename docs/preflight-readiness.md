# Preflight Readiness

FastaGuard runs before interpretive QC tools. It checks whether a FASTA file is
safe enough for downstream tools to consume, index, summarize, and route.

Preflight readiness is narrower than biological quality. It answers practical
questions such as:

- can the file be parsed as FASTA?
- are records present and non-empty?
- are identifiers usable by workflow joins, indexes, and annotation steps?
- are sequence symbols valid for the selected profile?
- are assembly-level warning signs visible before heavier QC starts?
- can a workflow engine trust the JSON, TSV, HTML, and MultiQC artifacts?

## Readiness Categories

- File readiness
- Structure readiness
- Alphabet readiness
- Index readiness
- Assembly readiness
- Submission readiness
- Machine readiness

File, structure, alphabet, and index readiness are the core consumption checks:
they help decide whether downstream tools are likely to parse and reference the
FASTA consistently. Assembly and submission readiness surface early advisories
that may need review before expensive QC, database-backed analysis, or official
submission. Machine readiness keeps the report contract explicit for workflow
engines, dashboards, and tool agents.

## Boundary

FastaGuard does not prove biological completeness, assembly correctness, or
contamination. It routes users to QUAST, BUSCO, BlobToolKit, CheckM, samtools,
BLAST, official submission validators, or annotation tools when those questions
matter.
