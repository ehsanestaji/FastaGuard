#!/usr/bin/env python3
import json
import sys
from pathlib import Path


EXIT_BY_STATUS = {
    "PASS": 0,
    "WARN": 1,
    "FAIL": 2,
}


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

    status = report.get("gate", {}).get("status")
    if status not in EXIT_BY_STATUS:
        print("missing or unsupported gate.status", file=sys.stderr)
        return 3

    print(f"FastaGuard gate status: {status}")
    return EXIT_BY_STATUS[status]


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
