import subprocess
from pathlib import Path


command = [snakemake.params.binary, "reference", str(snakemake.input.fasta)]
for name, flag in (
    ("fai", "--fai"), ("dict", "--dict"), ("alias_map", "--alias-map")
):
    values = getattr(snakemake.input, name)
    if values:
        command.extend([flag, str(values[0])])
for name, flag in (
    ("alignments", "--alignment"),
    ("variants", "--variants"),
    ("annotations", "--annotation"),
):
    for path in getattr(snakemake.input, name):
        command.extend([flag, str(path)])
command.extend(["--policy", snakemake.params.policy])
if snakemake.params.required:
    command.extend(["--require", ",".join(snakemake.params.required)])
command.extend(["--outdir", str(Path(snakemake.output.json).parent), "--prefix", "reference"])
if snakemake.params.write_lock:
    command.append("--write-lock")

log_path = Path(snakemake.log[0])
log_path.parent.mkdir(parents=True, exist_ok=True)
with log_path.open("w") as log:
    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
