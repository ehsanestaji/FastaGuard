import filecmp
import hashlib
import json
import os
import shutil
from pathlib import Path
from tempfile import NamedTemporaryFile


reports = dict(snakemake.params.reports)
if snakemake.params.lock:
    reports["lock"] = snakemake.params.lock

for name, destination in reports.items():
    source = Path(getattr(snakemake.input, name))
    target = Path(destination)
    if target.exists() and not filecmp.cmp(source, target, shallow=False):
        raise RuntimeError(f"existing report differs: {target}")

for name, destination in reports.items():
    source = Path(getattr(snakemake.input, name))
    target = Path(destination)
    if target.exists():
        continue
    with NamedTemporaryFile(dir=target.parent, prefix=".reference-copy-", delete=False) as temp:
        temporary = Path(temp.name)
        with source.open("rb") as handle:
            shutil.copyfileobj(handle, temp)
    try:
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)

digests = {}
for name, destination in reports.items():
    digest = hashlib.sha256()
    with Path(destination).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    digests[name] = digest.hexdigest()
with NamedTemporaryFile(
    mode="w", dir=Path(snakemake.params.report_digests).parent,
    prefix=".reference-digests-", delete=False,
) as temp:
    temporary = Path(temp.name)
    json.dump(digests, temp, sort_keys=True)
try:
    os.replace(temporary, snakemake.params.report_digests)
finally:
    temporary.unlink(missing_ok=True)

with NamedTemporaryFile(
    mode="w", dir=Path(snakemake.params.signature_path).parent,
    prefix=".reference-signature-", delete=False,
) as temp:
    temporary = Path(temp.name)
    temp.write(snakemake.params.signature + "\n")
try:
    os.replace(temporary, snakemake.params.signature_path)
finally:
    temporary.unlink(missing_ok=True)
Path(snakemake.output.complete).write_text(snakemake.params.signature + "\n")
Path(snakemake.log[0]).write_text("Reference reports published.\n")
