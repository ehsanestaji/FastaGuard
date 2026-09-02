#!/usr/bin/env python3
"""Summarise measured work avoided by an early reference-contract gate."""

from __future__ import annotations

import argparse
import json
from statistics import median
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Summarise measured savings from a FastaGuard reference gate."
    )
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--ungated", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    gated = read_observations(args.gated)
    ungated = read_observations(args.ungated)
    summary = summarise_repeats(gated, ungated)
    args.out.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))
    return 0


def summarise_repeats(gated: list[dict], ungated: list[dict]) -> dict:
    """Return median savings for matched gated and late-validation observations."""
    if not gated or len(gated) != len(ungated):
        raise ValueError("gated and ungated observations must have the same non-zero length")

    preflight_walls = []
    preflight_rss = []
    task_counts = []
    allocated_cpu_hours = []
    actual_cpu_seconds = []
    wall_seconds = []
    downstream_rss = []

    for gated_run, ungated_run in zip(gated, ungated, strict=True):
        preflight_wall = number(gated_run, "preflight_wall_seconds")
        preflight_cpu = number(gated_run, "preflight_cpu_seconds")
        preflight_rss.append(number(gated_run, "preflight_peak_rss_kib"))
        tasks = ungated_run.get("downstream_tasks")
        if not isinstance(tasks, list):
            raise ValueError("ungated downstream_tasks must be a list")

        task_counts.append(len(tasks))
        preflight_walls.append(preflight_wall)
        allocated_seconds = 0.0
        downstream_cpu = 0.0
        downstream_wall = 0.0
        peak_rss = 0.0
        for task in tasks:
            wall = number(task, "wall_seconds")
            cpu = number(task, "cpu_seconds")
            cpus = number(task, "requested_cpus")
            peak_rss = max(peak_rss, number(task, "peak_rss_kib"))
            allocated_seconds += wall * cpus
            downstream_cpu += cpu
            downstream_wall += wall
        allocated_cpu_hours.append(allocated_seconds / 3600)
        actual_cpu_seconds.append(max(0.0, downstream_cpu - preflight_cpu))
        wall_seconds.append(max(0.0, downstream_wall - preflight_wall))
        downstream_rss.append(peak_rss)

    return {
        "repeat_count": len(gated),
        "median_preflight_wall_seconds": rounded(median(preflight_walls)),
        "median_downstream_tasks_started_without_gate": rounded(median(task_counts)),
        "median_allocated_cpu_hours_avoided": rounded(median(allocated_cpu_hours)),
        "median_actual_cpu_seconds_avoided": rounded(median(actual_cpu_seconds)),
        "median_wall_seconds_avoided": rounded(median(wall_seconds)),
        "median_preflight_peak_rss_kib": rounded(median(preflight_rss)),
        "median_downstream_peak_rss_kib_without_gate": rounded(median(downstream_rss)),
        "interpretation": (
            "These are contextual savings from a controlled late validation baseline. "
            "They measure work avoided when the same reference mismatch is stopped "
            "at preflight, not a universal workflow performance guarantee."
        ),
    }


def number(value: dict, field: str) -> float:
    measured = value.get(field)
    if isinstance(measured, bool) or not isinstance(measured, (int, float)):
        raise ValueError(f"{field} must be numeric")
    if measured < 0:
        raise ValueError(f"{field} must not be negative")
    return float(measured)


def rounded(value: float) -> float | int:
    rounded_value = round(value, 4)
    return int(rounded_value) if rounded_value.is_integer() else rounded_value


def read_observations(path: Path) -> list[dict]:
    try:
        observations = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"could not read observations from {path}: {error}") from error
    if not isinstance(observations, list) or not all(isinstance(item, dict) for item in observations):
        raise ValueError("observations must be a JSON array of objects")
    return observations


if __name__ == "__main__":
    raise SystemExit(main())
