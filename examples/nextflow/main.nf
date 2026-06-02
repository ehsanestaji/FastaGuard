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
    # Fail-fast starter example: FastaGuard WARN exits 1 and FAIL exits 2.
    # Depending on engine behavior, evidence may remain only in the work directory.
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
// This is local fail-fast starter guidance, not an upstream nf-core submission yet.
// FastaGuard WARN exits 1 and FAIL exits 2; collect-then-gate wrappers can preserve
// evidence in publish directories before applying workflow stop/go logic.
