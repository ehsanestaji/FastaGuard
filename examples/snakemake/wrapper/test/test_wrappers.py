"""Add this pytest test to an upstream snakemake-wrappers checkout."""


def test_fastaguard(run):
    run(
        "bio/fastaguard",
        [
            "snakemake",
            "pass/fastaguard.json",
            "warn/fastaguard.json",
            "fail/fastaguard.json",
            "invalid/fastaguard.json",
        ],
    )
