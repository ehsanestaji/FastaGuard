nextflow.enable.dsl = 2

params.fasta = "sample.fa"

process FASTAGUARD {
    input:
    path fasta

    output:
    path "fastaguard_report.html"
    path "fastaguard.json"
    path "fastaguard.tsv"
    path "fastaguard_multiqc.json"

    script:
    """
    fastaguard ${fasta} \
      --profile assembly \
      --out fastaguard_report.html \
      --json fastaguard.json \
      --tsv fastaguard.tsv \
      --multiqc fastaguard_multiqc.json
    """
}

workflow {
    FASTAGUARD(file(params.fasta))
}
