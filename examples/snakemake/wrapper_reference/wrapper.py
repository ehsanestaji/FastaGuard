from snakemake.shell import shell


def _as_list(value):
    if not value:
        return []
    if isinstance(value, (str, bytes)) or hasattr(value, "__fspath__"):
        return [str(value)]
    return [str(item) for item in value if item]


extra = snakemake.params.get("extra", "")
log = snakemake.log_fmt_shell(stdout=True, stderr=True)

optional_args = []
fai = snakemake.input.get("fai", "")
if fai:
    optional_args.append(f"--fai {fai}")

dict_path = snakemake.input.get("dict", "")
if dict_path:
    optional_args.append(f"--dict {dict_path}")

for alignment in _as_list(
    snakemake.input.get("alignment") or snakemake.input.get("alignments")
):
    optional_args.append(f"--alignment {alignment}")

for variants in _as_list(snakemake.input.get("variants")):
    optional_args.append(f"--variants {variants}")

for annotation in _as_list(
    snakemake.input.get("annotation") or snakemake.input.get("annotations")
):
    optional_args.append(f"--annotation {annotation}")

alias_map = snakemake.input.get("alias_map", "")
if alias_map:
    optional_args.append(f"--alias-map {alias_map}")

lock = snakemake.output.get("lock", "")
if lock:
    if "--write-lock" not in extra.split():
        optional_args.append("--write-lock")
    optional_args.append(f"--lock {lock}")

optional_str = " ".join(optional_args)

# The wrapper writes reports; optional downstream policy reads the final JSON.
shell(
    "fastaguard reference {snakemake.input.reference} "
    "{optional_str} "
    "--out {snakemake.output.html} "
    "--json {snakemake.output.json} "
    "--tsv {snakemake.output.tsv} "
    "--multiqc {snakemake.output.multiqc} "
    "{extra} "
    "{log}"
)
