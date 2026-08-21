#!/usr/bin/env python3
"""Apply downstream workflow policy after FastaGuard has written its reports."""

import json
import sys
from pathlib import Path


MISSING = object()


def format_context(value):
    if value is MISSING:
        return "<missing>"
    if isinstance(value, str):
        return value
    return json.dumps(value, separators=(",", ":"), sort_keys=True)


def main(argv):
    if len(argv) != 2:
        print("usage: check_fastaguard_gate.py fastaguard.json", file=sys.stderr)
        return 3

    report_path = Path(argv[1])
    try:
        report = json.loads(report_path.read_text())
    except OSError as exc:
        print(f"could not read {report_path}: {exc}", file=sys.stderr)
        return 3
    except json.JSONDecodeError as exc:
        print(f"could not parse {report_path}: {exc}", file=sys.stderr)
        return 3

    if not isinstance(report, dict):
        print(
            "missing or malformed gate.can_continue (expected boolean)", file=sys.stderr
        )
        return 3

    gate = report.get("gate")
    can_continue = (
        gate.get("can_continue", MISSING) if isinstance(gate, dict) else MISSING
    )
    if not isinstance(can_continue, bool):
        print(
            "missing or malformed gate.can_continue (expected boolean)", file=sys.stderr
        )
        return 3

    verdict = report.get("verdict")
    verdict_status = (
        verdict.get("status", MISSING) if isinstance(verdict, dict) else MISSING
    )
    print(
        "FastaGuard report: "
        f"verdict={format_context(verdict_status)} "
        f"gate.status={format_context(gate.get('status', MISSING))} "
        f"gate.can_continue={str(can_continue).lower()} "
        f"gate.mode={format_context(gate.get('mode', MISSING))} "
        "gate.blocking_findings="
        f"{format_context(gate.get('blocking_findings', MISSING))}"
    )
    return 0 if can_continue else 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
