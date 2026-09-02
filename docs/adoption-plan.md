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

Status: GitHub, Bioconda, and BioContainers publish FastaGuard v0.7.0.
Bioconda serves `linux-64`, `linux-aarch64`, `osx-64`, and `osx-arm64`;
BioContainers publishes `quay.io/biocontainers/fastaguard:0.7.0--hfa8f182_0`.

- Keep GitHub release binaries working.
- Keep Docker smoke tests passing.
- Keep `packaging/bioconda/` aligned with the upstream Bioconda recipe.
- Keep workflow examples pinned to the confirmed BioContainers image tag.

Done when:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.7.0
fastaguard --schema
```

works in a clean environment, and workflow engines can pull the pinned
BioContainers image. This is now true for v0.7.0; keep repeating the same
check for future releases.

## Phase 2: Aggregate

Goal: make FastaGuard visible in standard pipeline reports.

- Continue emitting `fastaguard_mqc.json` custom content.
- Keep the unpublished local plugin starter in `integrations/multiqc/`
  compatible with current custom-content output.
- Keep gate, readiness, and submission fields visible in the local summary.
- Evaluate MultiQC publication only with adoption evidence.

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

Status: the v0.6 public evidence pack now commits portable JSON and TSV results
for three local contract cases and two exact NCBI reference assemblies. It
demonstrates a clean public PASS, a non-blocking public WARN, and a local
blocking FAIL while keeping complete reports and downloaded FASTA files out of
the repository.

Broader user evidence, cohort-scale compare runs, and direct comparisons with
downstream tools remain future adoption work.

## Phase 4: Workflow Readiness

Goal: keep the merged upstream integrations and local compatibility references
credible for workflow users.

Status: the nf-core module PR [#12239](https://github.com/nf-core/modules/pull/12239)
merged 2026-08-21. The Snakemake wrapper PR
[#5436](https://github.com/snakemake/snakemake-wrappers/pull/5436) merged
2026-07-27. Its v0.7 dependency update merged in
[#5826](https://github.com/snakemake/snakemake-wrappers/pull/5826) on
2026-08-24. Local starters remain useful compatibility references.

Next work:

- preserve the collect-then-gate pattern so JSON, TSV, HTML, and MultiQC
  evidence survives blocking FASTA results
- keep `examples/workflows/check_fastaguard_gate.py` aligned with the JSON gate
  contract for evidence-preserving workflow examples
- keep the workflow examples focused on stable FastaGuard contracts instead of
  broad biological interpretation

Detailed checklist: `docs/workflow-readiness.md`.

## Phase 5: Upstream workflow readiness

Goal: maintain the merged upstream integrations and their stable report
contracts.

Done when:

```text
future releases update existing integrations after GitHub and package publication
```

Keep local evidence-preserving examples verified as integrations evolve.

## Phase 6: Discovery and Pilot

Goal: make the current published tool discoverable and gather useful adoption
evidence without collecting user sequence data.

- Keep `docs/biotools-registration.json` as a reviewable bio.tools draft until
  a maintainer submits it. It remains pinned to the published v0.7.0
  distribution and uses EDAM `format_1929` (FASTA) with `operation_3180`
  (Sequence assembly validation).
- Offer optional report-only feedback pilots using `docs/pilots.md` before a
  team enables a new workflow gate.
- Never request FASTA files, raw sequences, input paths, or unreviewed reports.
- Quote a redacted case study only after explicit consent to the exact text.
- Use the current [bio.tools schema](https://github.com/bio-tools/biotoolsschema/blob/main/jsonschema/biotoolsj.json)
  and [EDAM ontology](https://github.com/edamontology/edamontology) when
  reviewing the registration draft.

The draft does not claim that a bio.tools record has been registered or that a
v1.0.0 package has been published.

## Phase 7: Expand

Goal: add profiles once the assembly preflight contract is trusted.

- transcriptome profile
- protein profile
- reference-panel profile
- compare mode for many FASTA files
- richer anomaly evidence
- LLM/tool-agent affordances on top of stable JSON and finding catalogs

Avoid expanding profiles before packaging and benchmarks are credible.
