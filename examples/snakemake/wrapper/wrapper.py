from snakemake.shell import shell

profile = snakemake.params.get("profile", "assembly")
gate = snakemake.params.get("gate", "pipeline")
extra = snakemake.params.get("extra", "")
log = snakemake.log_fmt_shell(stdout=True, stderr=True)

shell(
    "fastaguard {snakemake.input.fasta} "
    "--profile {profile} "
    "--gate {gate} "
    "--out {snakemake.output.html} "
    "--json {snakemake.output.json} "
    "--tsv {snakemake.output.tsv} "
    "--multiqc {snakemake.output.multiqc} "
    "{extra} "
    "{log}"
)
