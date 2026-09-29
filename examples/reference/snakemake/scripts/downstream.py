import hashlib
import json
from pathlib import Path


report_bytes = Path(snakemake.params.report).read_bytes()
digest = hashlib.sha256(report_bytes).hexdigest()
if Path(snakemake.input.accepted).read_text().strip() != digest:
    raise RuntimeError("acceptance marker does not match the current report")
report = json.loads(report_bytes)
if report.get("gate", {}).get("can_continue") is not True:
    raise RuntimeError("current reference report does not permit downstream work")
Path(snakemake.output.marker).write_text(f"{snakemake.params.run_id}\n")
Path(snakemake.log[0]).write_text("Reference accepted for downstream work.\n")
