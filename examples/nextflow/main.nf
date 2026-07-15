nextflow.enable.dsl = 2

params.fasta = "sample.fa"

process FASTAGUARD {
    input:
    path fasta

    output:
    path "fastaguard_report.html"
    path "fastaguard.json"
    path "fastaguard.tsv"
    path "fastaguard_mqc.json"

    script:
    """
    # FastaGuard writes PASS/WARN/FAIL decisions to JSON/TSV/HTML reports.
    # Route downstream workflow steps by reading gate.status from JSON.
    fastaguard ${fasta} \
      --profile assembly \
      --gate pipeline \
      --out fastaguard_report.html \
      --json fastaguard.json \
      --tsv fastaguard.tsv \
      --multiqc fastaguard_mqc.json
    """
}

workflow {
    FASTAGUARD(file(params.fasta))
}

// Compare mode starter pattern for v0.4 cohort triage:
// fastaguard compare assemblies/*.fa --profile assembly --gate pipeline
// This is local starter guidance, not an upstream nf-core submission yet.
// FastaGuard returns success when reports are written; collect-then-gate wrappers
// can apply workflow stop/go logic from gate.status.
