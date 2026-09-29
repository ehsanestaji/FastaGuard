import hashlib
import subprocess
import sys
from pathlib import Path


Path(snakemake.output.accepted).unlink(missing_ok=True)
result = subprocess.run(
    [sys.executable, str(snakemake.input.helper), str(snakemake.params.report)],
    text=True,
    capture_output=True,
)
message = result.stdout + result.stderr
Path(snakemake.log[0]).write_text(message)
print(message, end="")
if result.returncode:
    raise RuntimeError("reference gate failed; see gate log")
Path(snakemake.output.accepted).write_text(
    hashlib.sha256(Path(snakemake.params.report).read_bytes()).hexdigest() + "\n"
)
