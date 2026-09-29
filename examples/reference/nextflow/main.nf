nextflow.enable.dsl = 2

def shellQuote(value) {
    "'" + value.toString().replace("'", "'\"'\"'") + "'"
}

def parameter(key) {
    params.containsKey(key) ? params.get(key) : null
}

def defaulted(value, fallback) {
    value == null ? fallback : value
}

def asList(value) {
    value ? (value instanceof List ? value : [value]) : []
}

def optionalPath(value, key) {
    if (value == null) return []
    if (!(value instanceof String) || !value) error "${key} must be a path when supplied"
    file(value, checkIfExists: true)
}

def fileList(value, key) {
    def values = defaulted(value, [])
    if (!(values instanceof List) || !values.every { item -> item instanceof String && item }) {
        error "${key} must be a list of paths"
    }
    values.collect { item -> file(item, checkIfExists: true) }
}

def resolveBinary(value) {
    def location = value.contains('/') ? value : System.getenv('PATH')
        .split(java.io.File.pathSeparator)
        .collect { directory -> java.nio.file.Paths.get(directory, value) }
        .find { candidate -> java.nio.file.Files.isExecutable(candidate) && java.nio.file.Files.isRegularFile(candidate) }
    if (location == null) error "executable not found on PATH: ${value}"
    def path = file(location, checkIfExists: true)
    if (!java.nio.file.Files.isExecutable(java.nio.file.Paths.get(path.toString()))) {
        error "binary is not executable: ${value}"
    }
    path
}

def fileDigest(path) {
    def digest = java.security.MessageDigest.getInstance('SHA-256')
    java.nio.file.Files.newInputStream(java.nio.file.Paths.get(path.toString())).withCloseable { input ->
        def hashing = new java.security.DigestInputStream(input, digest)
        hashing.transferTo(java.io.OutputStream.nullOutputStream())
    }
    digest.digest().encodeHex().toString()
}

def inputSignature(files, settings) {
    def entries = files.collectEntries { kind, paths ->
        [(kind): asList(paths).collect { path ->
            [path: path.toString(), sha256: fileDigest(path)]
        }]
    }
    def payload = groovy.json.JsonOutput.toJson([files: entries, settings: settings])
    java.security.MessageDigest.getInstance('SHA-256')
        .digest(payload.getBytes('UTF-8')).encodeHex().toString()
}

process REFERENCE_REPORT {
    cache 'deep'

    input:
    path fasta, stageAs: 'canonical/*'
    path fai, stageAs: 'fai/*'
    path dictionary, stageAs: 'dict/*'
    path aliases, stageAs: 'aliases/*'
    path alignments, stageAs: 'alignments??/*'
    path variants, stageAs: 'variants??/*'
    path annotations, stageAs: 'annotations??/*'
    path binary, stageAs: 'fastaguard-bin'
    val policy
    val required
    val write_lock

    output:
    path 'bundle/reference.fastaguard.html', emit: html
    path 'bundle/reference.fastaguard.json', emit: json
    path 'bundle/reference.fastaguard.tsv', emit: tsv
    path 'bundle/reference.fastaguard_mqc.json', emit: multiqc
    path 'bundle/reference.fastaguard.lock.json', optional: true, emit: lock
    path 'bundle', emit: bundle

    script:
    def args = ['./fastaguard-bin', 'reference', fasta]
    if (fai) args.addAll(['--fai', fai])
    if (dictionary) args.addAll(['--dict', dictionary])
    if (aliases) args.addAll(['--alias-map', aliases])
    asList(alignments).each { path -> args.addAll(['--alignment', path]) }
    asList(variants).each { path -> args.addAll(['--variants', path]) }
    asList(annotations).each { path -> args.addAll(['--annotation', path]) }
    args.addAll(['--policy', policy, '--outdir', 'bundle', '--prefix', 'reference'])
    if (required) args.addAll(['--require', required.join(',')])
    if (write_lock) args.add('--write-lock')
    """
    ${args.collect { arg -> shellQuote(arg) }.join(' ')}
    """
}

process PERSIST_REPORTS {
    cache false

    input:
    path bundle
    val destination
    val signature

    output:
    path 'persist.receipt', emit: receipt

    script:
    def parent = java.nio.file.Paths.get(destination).parent
    """
    mkdir -p ${shellQuote(parent)}
    if [ -e ${shellQuote(destination)} ]; then
        [ -d ${shellQuote(destination)} ] || exit 3
        [ -f ${shellQuote(destination)}/.reference-inputs.sha256 ] || exit 3
        [ "\$(cat ${shellQuote(destination)}/.reference-inputs.sha256)" = ${shellQuote(signature)} ] || {
            printf 'Run ID already belongs to different inputs.\\n' >&2
            exit 3
        }
        for report in bundle/*; do
            name=\$(basename "\$report")
            cmp -s "\$report" ${shellQuote(destination)}/"\$name" || {
                printf 'Existing report differs: %s\\n' "\$name" >&2
                exit 3
            }
        done
        if [ -e ${shellQuote(destination)}/reference.fastaguard.lock.json ] && [ ! -e bundle/reference.fastaguard.lock.json ]; then
            printf 'Existing lockfile differs from this run.\\n' >&2
            exit 3
        fi
    else
        temporary=\$(mktemp -d ${shellQuote(destination + '.tmp.XXXXXXXX')})
        trap 'rm -rf "\$temporary"' EXIT
        cp -R bundle/. "\$temporary"/
        printf '%s\\n' ${shellQuote(signature)} > "\$temporary"/.reference-inputs.sha256
        mv "\$temporary" ${shellQuote(destination)}
        trap - EXIT
    fi
    printf '%s\\n' ${shellQuote(destination)} > persist.receipt
    """
}

process REFERENCE_GATE {
    cache false

    input:
    path receipt
    path report
    path helper

    output:
    path 'reference.accepted.sha256', emit: accepted

    script:
    """
    python3 ${shellQuote(helper)} ${shellQuote(report)}
    python3 -c 'import hashlib, pathlib, sys; print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())' ${shellQuote(report)} > reference.accepted.sha256
    """
}

process DOWNSTREAM {
    cache false

    input:
    path accepted
    path report
    val destination
    val run_id

    output:
    path 'downstream.started', emit: marker

    script:
    """
    python3 -c 'import hashlib, json, pathlib, sys; report = pathlib.Path(sys.argv[1]).read_bytes(); marker = pathlib.Path(sys.argv[2]).read_text().strip(); assert marker == hashlib.sha256(report).hexdigest(); assert json.loads(report)["gate"]["can_continue"] is True' ${shellQuote(report)} ${shellQuote(accepted)}
    printf '%s\\n' ${shellQuote(run_id)} > downstream.started
    cp downstream.started ${shellQuote(destination)}/downstream.started
    """
}

workflow {
    def fasta = parameter('fasta')
    def output_dir = parameter('output_dir')
    if (!(fasta instanceof String) || !fasta) error 'fasta must be a non-empty path'
    if (!(output_dir instanceof String) || !output_dir) error 'output_dir must be a non-empty path'

    def run_id = defaulted(parameter('run_id'), 'reference')
    if (!(run_id instanceof String) || !(run_id ==~ /[A-Za-z0-9][A-Za-z0-9_-]*/)) {
        error 'run_id must contain only letters, digits, underscores or hyphens'
    }
    def mode = defaulted(parameter('mode'), 'report')
    def policy = defaulted(parameter('policy'), 'coordinate')
    def required = defaulted(parameter('require'), [])
    def write_lock = defaulted(parameter('write_lock'), false)
    def binary_value = defaulted(parameter('binary'), 'fastaguard')
    if (!(mode in ['report', 'enforce'])) error 'mode must be report or enforce'
    if (!(policy in ['strict', 'coordinate', 'advisory'])) error 'policy must be strict, coordinate or advisory'
    if (!(required instanceof List) || !required.every { kind -> kind in ['fai', 'dict', 'alignment', 'variants', 'annotation'] }) {
        error 'require must list supported companion kinds'
    }
    if (!(write_lock instanceof Boolean)) error 'write_lock must be boolean'
    if (!(binary_value instanceof String) || !binary_value) error 'binary must be a non-empty executable path or command'

    def binary_file = resolveBinary(binary_value)
    def fasta_file = file(fasta, checkIfExists: true)
    def fai_file = optionalPath(parameter('fai'), 'fai')
    def dict_file = optionalPath(parameter('dict'), 'dict')
    def alias_file = optionalPath(parameter('alias_map'), 'alias_map')
    def alignment_files = fileList(parameter('alignments'), 'alignments')
    def variant_files = fileList(parameter('variants'), 'variants')
    def annotation_files = fileList(parameter('annotations'), 'annotations')
    def signature = inputSignature(
        [fasta: fasta_file, fai: fai_file, dict: dict_file, aliases: alias_file,
         alignments: alignment_files, variants: variant_files,
         annotations: annotation_files, binary: binary_file],
        [mode: mode, policy: policy, require: required, write_lock: write_lock],
    )
    def destination = java.nio.file.Paths.get(output_dir).toAbsolutePath().normalize().resolve(run_id).toString()
    def helper = file("${projectDir}/../../workflows/check_fastaguard_gate.py", checkIfExists: true)

    REFERENCE_REPORT(
        channel.value(fasta_file),
        channel.value(fai_file),
        channel.value(dict_file),
        channel.value(alias_file),
        channel.value(alignment_files),
        channel.value(variant_files),
        channel.value(annotation_files),
        channel.value(binary_file),
        channel.value(policy),
        channel.value(required),
        channel.value(write_lock),
    )
    PERSIST_REPORTS(REFERENCE_REPORT.out.bundle, channel.value(destination), channel.value(signature))
    if (mode == 'enforce') {
        REFERENCE_GATE(
            PERSIST_REPORTS.out.receipt,
            REFERENCE_REPORT.out.json,
            channel.value(helper),
        )
        DOWNSTREAM(
            REFERENCE_GATE.out.accepted,
            REFERENCE_REPORT.out.json,
            channel.value(destination),
            channel.value(run_id),
        )
    }
}
