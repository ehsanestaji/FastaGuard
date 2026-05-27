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
    # Gate failures intentionally exit 2 after writing reports.
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
