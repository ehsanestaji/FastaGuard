process FASTAGUARD {
    tag "$meta.id"
    label 'process_low'
    container 'quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0'

    input:
    tuple val(meta), path(fasta)

    output:
    tuple val(meta), path("*.fastaguard.html"), emit: html
    tuple val(meta), path("*.fastaguard.json"), emit: json
    tuple val(meta), path("*.fastaguard.tsv"), emit: tsv
    tuple val(meta), path("*.fastaguard_mqc.json"), emit: mqc
    path "versions.yml", emit: versions

    script:
    def prefix = task.ext.prefix ?: meta.id
    """
    fastaguard ${fasta} \
      --profile assembly \
      --out ${prefix}.fastaguard.html \
      --json ${prefix}.fastaguard.json \
      --tsv ${prefix}.fastaguard.tsv \
      --multiqc ${prefix}.fastaguard_mqc.json

    cat <<-END_VERSIONS > versions.yml
    "${task.process}":
        fastaguard: \$(fastaguard --version | awk '{print \$2}')
    END_VERSIONS
    """
}
