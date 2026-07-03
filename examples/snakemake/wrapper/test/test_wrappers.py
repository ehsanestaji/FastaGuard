"""Run this snippet with pytest in an upstream wrapper checkout."""

import subprocess
from pathlib import Path


def test_fastaguard_wrapper():
    snakefile = Path(__file__).with_name("Snakefile")
    subprocess.run(
        ["snakemake", "-s", str(snakefile), "--cores", "1", "--use-conda"],
        check=True,
    )
