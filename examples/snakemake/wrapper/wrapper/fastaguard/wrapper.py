from snakemake.shell import shell

profile = snakemake.params.get("profile", "assembly")
extra = snakemake.params.get("extra", "")

shell(
    "fastaguard {snakemake.input.fasta} "
    "--profile {profile} "
    "--out {snakemake.output.html} "
    "--json {snakemake.output.json} "
    "--tsv {snakemake.output.tsv} "
    "--multiqc {snakemake.output.multiqc} "
    "{extra}"
)
