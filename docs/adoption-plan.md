# Adoption Plan

## Recommendation

The next product phase should focus on installability and pipeline trust before
adding many new biological heuristics.

Priority:

```text
Bioconda published -> BioContainers available -> MultiQC plugin -> public benchmarks -> upstream workflow readiness
```

## Phase 1: Package

Goal: make installation natural for bioinformatics users.

Status: Bioconda is live for FastaGuard v0.5.0 on Linux and macOS x86_64/ARM64
platforms. BioContainers publishes the pinned workflow image
`quay.io/biocontainers/fastaguard:0.5.0--hfa8f182_0`.

- Keep GitHub release binaries working.
- Keep Docker smoke tests passing.
- Keep `packaging/bioconda/` aligned with the upstream Bioconda recipe.
- Keep workflow examples pinned to the confirmed BioContainers image tag.

Done when:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.5.0
fastaguard --schema
```

works in a clean environment, and workflow engines can pull the pinned
BioContainers image. This is now true for v0.5.0; keep repeating the same
check for future releases.

## Phase 2: Aggregate

Goal: make FastaGuard visible in standard pipeline reports.

- Continue emitting `fastaguard_mqc.json` custom content.
- Develop `integrations/multiqc/` into a packaged MultiQC plugin.
- Test the plugin against multiple sample reports.
- Keep v0.5 gate, readiness, and submission fields visible in the native
  MultiQC summary table.
- Decide whether to submit upstream to MultiQC once public adoption begins.

Done when:

```bash
multiqc .
```

shows FastaGuard verdicts and key metrics across many samples.

## Phase 3: Prove

Goal: show why FastaGuard is worth adding before expensive tools.

- Benchmark public FASTA files.
- Capture examples of duplicate IDs, invalid symbols, high-N scaffolds, and suspicious composition.
- Document which findings should block downstream tools and which should only recommend deeper QC.
- Create a concise comparison against `seqkit stats`, QUAST, BUSCO, BlobToolKit, FastQC, and MultiQC.

Done when the README can show real examples rather than only promises.

## Phase 4: Workflow Readiness

Goal: make local workflow starters credible enough to become upstream
submissions.

Status: local nf-core-style and Snakemake wrapper-style starters are present,
pinned to the v0.5.0 Bioconda package and BioContainers image, documented as
starters rather than official upstream submissions, and validated in dedicated
upstream-style checkouts on 2026-07-03.

Next work:

- prepare external PR branches for nf-core/modules and snakemake-wrappers
- repeat nf-core lint/test and Snakemake formatting/lint/pytest immediately
  before opening upstream PRs
- preserve the collect-then-gate pattern so JSON, TSV, HTML, and MultiQC
  evidence survives blocking FASTA results
- keep `examples/workflows/check_fastaguard_gate.py` aligned with the JSON gate
  contract for evidence-preserving workflow examples
- keep the workflow examples focused on stable FastaGuard contracts instead of
  broad biological interpretation

Detailed checklist: `docs/workflow-readiness.md`.

## Phase 5: Upstream workflow readiness

Goal: submit the starter assets upstream once the package, container, examples,
and tests are aligned with current community expectations.

Done when:

```text
nf-core module PR ready + Snakemake wrapper PR ready + local evidence-preserving examples verified
```

This phase should not claim official upstream status until those PRs are
accepted.

## Phase 6: Expand

Goal: add profiles once the assembly preflight contract is trusted.

- transcriptome profile
- protein profile
- reference-panel profile
- compare mode for many FASTA files
- richer anomaly evidence
- LLM/tool-agent affordances on top of stable JSON and finding catalogs

Avoid expanding profiles before packaging and benchmarks are credible.
