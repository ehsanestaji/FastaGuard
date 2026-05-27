# Adoption Plan

## Recommendation

The next product phase should focus on installability and pipeline trust before
adding many new biological heuristics.

Priority:

```text
Bioconda published -> BioContainers available -> MultiQC plugin -> public benchmarks -> upstream workflow examples
```

## Phase 1: Package

Goal: make installation natural for bioinformatics users.

Status: Bioconda is live for FastaGuard v0.2.0 on Linux and macOS x86_64/ARM64
platforms. BioContainers publishes the pinned workflow image
`quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0`.

- Keep GitHub release binaries working.
- Keep Docker smoke tests passing.
- Keep `packaging/bioconda/` aligned with the upstream Bioconda recipe.
- Keep workflow examples pinned to the confirmed BioContainers image tag.

Done when:

```bash
mamba install -c conda-forge -c bioconda fastaguard
fastaguard --schema
```

works in a clean environment, and workflow engines can pull the pinned
BioContainers image.

## Phase 2: Aggregate

Goal: make FastaGuard visible in standard pipeline reports.

- Continue emitting `fastaguard_mqc.json` custom content.
- Develop `integrations/multiqc/` into a packaged MultiQC plugin.
- Test the plugin against multiple sample reports.
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

## Phase 4: Expand

Goal: add profiles once the assembly preflight contract is trusted.

- transcriptome profile
- protein profile
- reference-panel profile
- compare mode for many FASTA files
- richer anomaly evidence
- LLM/tool-agent affordances on top of stable JSON and finding catalogs

Avoid expanding profiles before packaging and benchmarks are credible.
