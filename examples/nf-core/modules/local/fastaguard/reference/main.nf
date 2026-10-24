process FASTAGUARD_REFERENCE {
    tag "$meta.id"
    label 'process_low'

    conda "${moduleDir}/environment.yml"
    container "${ workflow.containerEngine in ['singularity', 'apptainer'] && !task.ext.singularity_pull_docker_container ?
        'https://depot.galaxyproject.org/singularity/fastaguard:1.0.0--hfa8f182_0':
        'quay.io/biocontainers/fastaguard:1.0.0--hfa8f182_0' }"

    input:
    tuple val(meta), path(reference), path(fai), path(dict), path(alignments), path(variants), path(annotations), path(alias_map)

    output:
    tuple val(meta), path("*.fastaguard.html"), emit: html
    tuple val(meta), path("*.fastaguard.json"), emit: json
    tuple val(meta), path("*.fastaguard.tsv"), emit: tsv
    tuple val(meta), path("*.fastaguard_mqc.json"), emit: mqc
    tuple val(meta), path("*.fastaguard.lock.json"), optional: true, emit: lock
    tuple val("${task.process}"), val('fastaguard'), eval('fastaguard --version | cut -d " " -f 2'), emit: versions_fastaguard, topic: versions

    when:
    task.ext.when == null || task.ext.when

    script:
    def prefix = task.ext.prefix ?: "${meta.id}"
    def args = task.ext.args ?: ''
    def fai_arg = fai ? "--fai ${fai}" : ''
    def dict_arg = dict ? "--dict ${dict}" : ''
    def alignment_args = (alignments instanceof List ? alignments : (alignments ? [alignments] : [])).collect { "--alignment ${it}" }.join(' ')
    def variant_args = (variants instanceof List ? variants : (variants ? [variants] : [])).collect { "--variants ${it}" }.join(' ')
    def annotation_args = (annotations instanceof List ? annotations : (annotations ? [annotations] : [])).collect { "--annotation ${it}" }.join(' ')
    def alias_arg = alias_map ? "--alias-map ${alias_map}" : ''
    // Keep report collection explicit; downstream policy reads the final JSON.
    """
    fastaguard reference ${reference} \\
      ${fai_arg} \\
      ${dict_arg} \\
      ${alignment_args} \\
      ${variant_args} \\
      ${annotation_args} \\
      ${alias_arg} \\
      ${args} \\
      --out ${prefix}.fastaguard.html \\
      --json ${prefix}.fastaguard.json \\
      --tsv ${prefix}.fastaguard.tsv \\
      --multiqc ${prefix}.fastaguard_mqc.json \\
      --lock ${prefix}.fastaguard.lock.json
    """
}
