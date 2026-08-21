# nf-core Module Reference

This directory mirrors the interface of the FastaGuard module merged into
nf-core/modules in [PR #12239](https://github.com/nf-core/modules/pull/12239).
It remains a local compatibility reference for future FastaGuard releases.

See `../../docs/workflow-readiness.md` for the current update checklist.

Expected input channel:

```nextflow
tuple val(meta), path(fasta)
```

The module emits four QC reports plus version metadata:

- `html`
- `json`
- `tsv`
- `mqc`
- `versions_fastaguard` on the versions topic

The module assumes `fastaguard` is available on `PATH` when run without a
container. The published v0.6 install and pinned workflow image are:

```bash
mamba install -c conda-forge -c bioconda fastaguard=0.6.0
docker pull quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0
```

The reusable module does not select a profile or gate. Callers pass optional
CLI arguments through `task.ext.args` and may customize output names through
`task.ext.prefix ?: "${meta.id}"`. For example:

```nextflow
process {
    withName: FASTAGUARD {
        ext.args = '--profile assembly --gate pipeline'
    }
}
```

FastaGuard v0.6 returns `0` after successfully writing PASS, WARN, or FAIL
reports. A downstream collect-then-gate step should parse the JSON or TSV and
route on `verdict.status`, `gate.status`, and `gate.blocking_findings`. Process
status is reserved for command, input, runtime, and output-write errors.

For submission-readiness preflight before official validators, callers can use:

```nextflow
process {
    withName: FASTAGUARD {
        ext.args = '--profile assembly --gate submission --submission-target ncbi'
    }
}
```

This remains FASTA-level readiness only and does not replace repository
validators or downstream interpretive QC.

For cohort triage, `fastaguard compare` remains a separate starter pattern:

```bash
fastaguard compare assemblies/*.fa --profile assembly --gate pipeline
```

Example include:

```nextflow
include { FASTAGUARD } from './modules/local/fastaguard'
```

For a future upstream update, use this safe local order:

1. Run the repository Python contract tests.
2. Validate a fresh nf-core/modules checkout with the current nf-core/tools,
   Nextflow, and nf-test versions.
3. Run `nf-core modules lint fastaguard`.
4. Run `nf-core modules test fastaguard`.
5. Update the existing upstream module only after the FastaGuard release and
   package are published.
