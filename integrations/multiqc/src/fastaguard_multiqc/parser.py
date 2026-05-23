"""Parser helpers for FastaGuard MultiQC integration."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


REQUIRED_SUMMARY_FIELDS = (
    "verdict",
    "sequence_count",
    "total_length",
    "n50",
    "n90",
    "gc_percent",
    "n_percent",
    "finding_count",
)

OPTIONAL_SUMMARY_FIELDS = (
    "duplicate_id_count",
    "invalid_sequence_count",
    "high_n_sequence_count",
    "tiny_contig_count",
    "max_gap_run",
    "gc_outlier_count",
    "length_outlier_count",
    "composite_anomaly_count",
)

SUMMARY_FIELDS = REQUIRED_SUMMARY_FIELDS + OPTIONAL_SUMMARY_FIELDS

FASTAGUARD_SEARCH_PATTERN = {
    "fastaguard": [
        {"fn": "fastaguard_mqc.json"},
        {"fn": "*.fastaguard_mqc.json"},
    ]
}


def register_search_patterns() -> None:
    """Register FastaGuard file search patterns with MultiQC."""
    from multiqc import config

    config.sp = {
        "fastaguard": FASTAGUARD_SEARCH_PATTERN["fastaguard"],
        **{key: value for key, value in config.sp.items() if key != "fastaguard"},
    }


def load_custom_content_summary(path: str | Path) -> dict[str, dict[str, Any]]:
    """Load one FastaGuard MultiQC custom-content JSON file."""
    report_path = Path(path)
    payload = json.loads(report_path.read_text())

    if payload.get("id") != "fastaguard":
        raise ValueError(f"{report_path} is not a FastaGuard MultiQC custom-content file")
    if payload.get("plot_type") != "table":
        raise ValueError(f"{report_path} is not a FastaGuard table custom-content file")

    data = payload.get("data")
    if not isinstance(data, dict) or not data:
        raise ValueError(f"{report_path} has no FastaGuard sample data")

    parsed: dict[str, dict[str, Any]] = {}
    for sample_name, row in data.items():
        if not isinstance(row, dict):
            raise ValueError(f"{report_path} sample {sample_name!r} is not a table row")
        missing_fields = [
            field for field in REQUIRED_SUMMARY_FIELDS if row.get(field) is None
        ]
        if missing_fields:
            missing = ", ".join(missing_fields)
            raise ValueError(
                f"{report_path} sample {sample_name!r} missing required "
                f"FastaGuard summary fields: {missing}"
            )

        parsed_row = {field: row[field] for field in REQUIRED_SUMMARY_FIELDS}
        parsed_row.update(
            {
                field: row[field]
                for field in OPTIONAL_SUMMARY_FIELDS
                if row.get(field) is not None
            }
        )
        parsed[str(sample_name)] = parsed_row

    return parsed


def find_custom_content_files(root: str | Path) -> list[Path]:
    """Find likely FastaGuard MultiQC custom-content files below root."""
    search_root = Path(root)
    candidates = {
        path
        for pattern in ("*fastaguard_mqc.json", "*fastaguard*.mqc.json")
        for path in search_root.rglob(pattern)
        if path.is_file()
    }
    return sorted(candidates)
