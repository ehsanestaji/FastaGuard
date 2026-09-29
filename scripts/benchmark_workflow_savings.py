#!/usr/bin/env python3
"""Summarise measured work avoided by an early reference-contract gate."""

from __future__ import annotations

import argparse
import json
import math
import re
from pathlib import Path
from statistics import median


CONTEXT_FIELDS = ("engine", "case_id", "input_sha256", "binary_sha256", "runner_id")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Summarise measured savings from a FastaGuard reference gate."
    )
    parser.add_argument("--gated", type=Path, required=True)
    parser.add_argument("--ungated", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    try:
        summary = summarise_repeats(
            read_observations(args.gated), read_observations(args.ungated),
        )
        document = json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + "\n"
        with args.out.open("x", encoding="utf-8") as output:
            output.write(document)
    except (ValueError, OSError, RecursionError) as error:
        parser.error(str(error))

    print(document, end="")
    return 0


def summarise_repeats(gated: list[dict], ungated: list[dict]) -> dict:
    """Summarise one condition, pairing repeats by identity rather than position."""
    gated_by_id = index_observations(gated, "gated")
    ungated_by_id = index_observations(ungated, "ungated")
    if gated_by_id.keys() != ungated_by_id.keys():
        raise ValueError("gated and ungated run_id sets must match")
    run_ids = sorted(gated_by_id)
    context = observation_context(gated_by_id[run_ids[0]])
    for run in (*gated, *ungated):
        if observation_context(run) != context:
            raise ValueError("all observations must share the same comparison context")

    has_workflow_time = ["workflow_wall_seconds" in run for run in (*gated, *ungated)]
    if any(has_workflow_time) and not all(has_workflow_time):
        raise ValueError("workflow_wall_seconds must be supplied for every pair or none")

    preflight_walls, preflight_rss, task_counts = [], [], []
    allocated_cpu_hours, actual_cpu_seconds, task_wall_seconds = [], [], []
    downstream_rss, workflow_wall_seconds, timed_out_run_ids = [], [], []

    for run_id in run_ids:
        gated_run, ungated_run = gated_by_id[run_id], ungated_by_id[run_id]
        if gated_run.get("status") != "completed":
            raise ValueError("gated observations must have completed status")
        if gated_run.get("gate_can_continue") is not False:
            raise ValueError("gated observations must record gate_can_continue=false")
        if gated_run.get("downstream_tasks") != []:
            raise ValueError("gated observations must have an empty downstream_tasks list")
        if ungated_run.get("status") not in ("completed", "timeout"):
            raise ValueError("ungated status must be completed or timeout")
        if ungated_run["status"] == "timeout":
            timed_out_run_ids.append(run_id)

        preflight_wall = number(gated_run, "preflight_wall_seconds")
        preflight_cpu = number(gated_run, "preflight_cpu_seconds")
        preflight_rss.append(number(gated_run, "preflight_peak_rss_kib"))
        tasks = ungated_run.get("downstream_tasks")
        if not isinstance(tasks, list) or not tasks:
            raise ValueError("ungated downstream_tasks must be a non-empty list")

        task_counts.append(len(tasks))
        preflight_walls.append(preflight_wall)
        allocated_seconds = downstream_cpu = downstream_wall = peak_rss = 0.0
        for task in tasks:
            if not isinstance(task, dict):
                raise ValueError("each downstream task must be an object")
            text_field(task, "name")
            wall = number(task, "wall_seconds")
            cpu = number(task, "cpu_seconds")
            cpus = number(task, "requested_cpus")
            if not isinstance(task["requested_cpus"], int) or cpus < 1:
                raise ValueError("requested_cpus must be a positive integer")
            peak_rss = max(peak_rss, number(task, "peak_rss_kib"))
            allocated_seconds = finite(allocated_seconds + wall * cpus)
            downstream_cpu = finite(downstream_cpu + cpu)
            downstream_wall = finite(downstream_wall + wall)
        allocated_cpu_hours.append(allocated_seconds / 3600)
        actual_cpu_seconds.append(finite(downstream_cpu - preflight_cpu))
        task_wall_seconds.append(finite(downstream_wall - preflight_wall))
        downstream_rss.append(peak_rss)
        if all(has_workflow_time):
            workflow_wall_seconds.append(finite(
                number(ungated_run, "workflow_wall_seconds")
                - number(gated_run, "workflow_wall_seconds")
            ))

    censored_count = len(timed_out_run_ids)
    interpretation = (
        "These are contextual savings from a controlled late validation baseline, "
        "not a universal workflow performance guarantee. Task-summed wall time "
        "is not whole-workflow elapsed time. Allocated CPU-hours describe gross "
        "downstream allocation; CPU and task-time differences subtract preflight "
        "cost and retain negative values."
    )
    if censored_count:
        interpretation += (
            " Timed-out baselines are right-censored: their recorded task costs "
            "and the combined median savings are lower bounds, not completed-run "
            "estimates. Missing measurements must not be replaced with zero."
        )

    return {
        "summary_schema_version": 2,
        "context": context,
        "run_ids": run_ids,
        "repeat_count": len(run_ids),
        "completed_pair_count": len(run_ids) - censored_count,
        "censored_pair_count": censored_count,
        "timed_out_run_ids": timed_out_run_ids,
        "estimate_kind": "lower_bound" if censored_count else "observed",
        "median_preflight_wall_seconds": finite(median(preflight_walls)),
        "median_downstream_tasks_started_without_gate": finite(median(task_counts)),
        "median_allocated_cpu_hours_avoided": finite(median(allocated_cpu_hours)),
        "median_actual_cpu_seconds_avoided": finite(median(actual_cpu_seconds)),
        "median_task_summed_wall_seconds_avoided": finite(median(task_wall_seconds)),
        "median_workflow_wall_seconds_avoided": (
            finite(median(workflow_wall_seconds)) if workflow_wall_seconds else None
        ),
        "median_preflight_peak_rss_kib": finite(median(preflight_rss)),
        "median_downstream_peak_rss_kib_without_gate": finite(median(downstream_rss)),
        "interpretation": interpretation,
    }


def index_observations(observations: list[dict], label: str) -> dict[str, dict]:
    if not isinstance(observations, list) or not observations:
        raise ValueError(f"{label} observations must be a non-empty array")
    indexed = {}
    for run in observations:
        if not isinstance(run, dict):
            raise ValueError(f"each {label} observation must be an object")
        run_id = text_field(run, "run_id")
        if run_id in indexed:
            raise ValueError(f"duplicate run_id in {label} observations: {run_id}")
        indexed[run_id] = run
    return indexed


def observation_context(run: dict) -> dict:
    context = {field: text_field(run, field) for field in CONTEXT_FIELDS}
    for field in ("input_sha256", "binary_sha256"):
        if re.fullmatch(r"[a-f0-9]{64}", context[field]) is None:
            raise ValueError(f"{field} must be a lowercase SHA-256 digest")
    return context


def text_field(value: dict, field: str) -> str:
    text = value.get(field)
    if not isinstance(text, str) or not text.strip():
        raise ValueError(f"{field} must be a non-empty string")
    return text


def finite(value: float) -> float:
    try:
        result = float(value)
    except (OverflowError, ValueError) as error:
        raise ValueError("measurements and derived values must be finite") from error
    if not math.isfinite(result):
        raise ValueError("measurements and derived values must be finite")
    return result


def number(value: dict, field: str) -> float:
    measured = value.get(field)
    if isinstance(measured, bool) or not isinstance(measured, (int, float)):
        raise ValueError(f"{field} must be numeric")
    result = finite(measured)
    if result < 0:
        raise ValueError(f"{field} must not be negative")
    return result


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def reject_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON constant: {value}")


def read_observations(path: Path) -> list[dict]:
    try:
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
            parse_float=finite,
        )
    except (OSError, ValueError) as error:
        raise ValueError(f"could not read observations from {path}: {error}") from error


if __name__ == "__main__":
    raise SystemExit(main())
