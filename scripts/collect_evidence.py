#!/usr/bin/env python3
"""Collect reproducible FastaGuard evidence runs."""

from __future__ import annotations

import argparse
import csv
import gzip
import json
import platform
import shutil
import subprocess
import sys
import time
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
LOCAL_SUCCESS_CODES = {0, 1, 2}
SUMMARY_COLUMNS = [
    "id",
    "label",
    "category",
    "source",
    "accession",
    "input_bytes",
    "elapsed_seconds",
    "exit_code",
    "verdict",
    "gate_mode",
    "gate_status",
    "gate_blocking_findings",
    "input_sha256",
    "sequence_count",
    "total_length",
    "n50",
    "n90",
    "finding_count",
    "top_findings",
]


def main() -> int:
    args = parse_args()
    binary = args.binary.resolve()
    out_dir = args.out_dir.resolve()
    manifest_path = args.manifest.resolve()

    if not binary.exists():
        raise SystemExit(f"FastaGuard binary not found: {binary}")

    if not args.local_only and shutil.which("datasets") is None:
        raise SystemExit(
            "NCBI Datasets CLI not found. Install `datasets` or rerun with --local-only."
        )

    out_dir.mkdir(parents=True, exist_ok=True)
    cases = local_cases(out_dir)
    if not args.local_only:
        cases.extend(public_cases(manifest_path, out_dir))

    summary = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "fastaguard_version": fastaguard_version(binary),
        "git_commit": git_commit(),
        "platform": platform.platform(),
        "python": platform.python_version(),
        "command": " ".join(sys.argv),
        "local_only": args.local_only,
        "cases": [],
    }

    for case in cases:
        if case["source"] == "public_ncbi":
            prepare_public_input(case)
        result = run_case(binary, case)
        summary["cases"].append(result)

    write_summary(out_dir, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Collect FastaGuard evidence runs for local fixtures and public assemblies."
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path("target/release/fastaguard"),
        help="Path to the FastaGuard binary to run.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("target/evidence/v0.3"),
        help="Directory for evidence outputs and summaries.",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("docs/evidence/public_assemblies.json"),
        help="Public assembly manifest.",
    )
    parser.add_argument(
        "--local-only",
        action="store_true",
        help="Skip public NCBI downloads and run only local evidence cases.",
    )
    return parser.parse_args()


def local_cases(out_dir: Path) -> list[dict[str, Any]]:
    synthetic_dir = out_dir / "synthetic_valid"
    synthetic_path = synthetic_dir / "synthetic.fa"
    synthetic_dir.mkdir(parents=True, exist_ok=True)
    write_synthetic_fasta(synthetic_path)

    gzip_dir = out_dir / "gzipped_valid"
    gzip_path = gzip_dir / "valid_assembly.fa.gz"
    gzip_dir.mkdir(parents=True, exist_ok=True)
    gzip_fasta(ROOT / "testdata" / "valid_assembly.fa", gzip_path)

    return [
        {
            "id": "synthetic_valid",
            "label": "Deterministic synthetic FASTA",
            "category": "synthetic",
            "source": "local",
            "accession": None,
            "input_path": synthetic_path,
            "case_dir": synthetic_dir,
        },
        {
            "id": "problem_fixture",
            "label": "Problem assembly fixture",
            "category": "fixture",
            "source": "local",
            "accession": None,
            "input_path": ROOT / "testdata" / "problem_assembly.fa",
            "case_dir": out_dir / "problem_fixture",
        },
        {
            "id": "gzipped_valid",
            "label": "Gzipped valid assembly fixture",
            "category": "fixture",
            "source": "local",
            "accession": None,
            "input_path": gzip_path,
            "case_dir": gzip_dir,
        },
    ]


def public_cases(manifest_path: Path, out_dir: Path) -> list[dict[str, Any]]:
    manifest = json.loads(manifest_path.read_text())
    cases = []
    for assembly in manifest["assemblies"]:
        case_dir = out_dir / assembly["id"]
        cases.append(
            {
                **assembly,
                "source": "public_ncbi",
                "input_path": case_dir / "genomic.fna",
                "case_dir": case_dir,
            }
        )
    return cases


def write_synthetic_fasta(path: Path) -> None:
    records = [
        ("synthetic_0001", "ACGT" * 40),
        ("synthetic_0002", "TGCA" * 40),
        ("synthetic_0003", "GATTACA" * 20),
        ("synthetic_0004", "CCGGTTAA" * 18),
    ]
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        for record_id, sequence in records:
            handle.write(f">{record_id}\n")
            for offset in range(0, len(sequence), 80):
                handle.write(sequence[offset : offset + 80])
                handle.write("\n")


def gzip_fasta(source: Path, destination: Path) -> None:
    with source.open("rb") as src, gzip.open(destination, "wb") as dst:
        shutil.copyfileobj(src, dst)


def prepare_public_input(case: dict[str, Any]) -> None:
    case_dir = case["case_dir"]
    case_dir.mkdir(parents=True, exist_ok=True)
    zip_path = case_dir / "ncbi_dataset.zip"
    command = [
        "datasets",
        "download",
        "genome",
        "accession",
        case["accession"],
        "--include",
        "genome",
        "--filename",
        str(zip_path),
    ]
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise SystemExit(
            "NCBI Datasets download failed for "
            f"{case['accession']} with exit code {completed.returncode}\n"
            f"STDOUT:\n{completed.stdout}\nSTDERR:\n{completed.stderr}"
        )

    fasta_member = find_genomic_fasta(zip_path)
    with zipfile.ZipFile(zip_path) as archive:
        with archive.open(fasta_member) as src, case["input_path"].open("wb") as dst:
            shutil.copyfileobj(src, dst)


def find_genomic_fasta(zip_path: Path) -> str:
    with zipfile.ZipFile(zip_path) as archive:
        candidates = [
            name
            for name in archive.namelist()
            if name.endswith((".fna", ".fa", ".fasta"))
            and not name.endswith("/")
            and "/data/" in name
        ]
    if not candidates:
        raise SystemExit(f"No genomic FASTA found in {zip_path}")
    return sorted(candidates)[0]


def run_case(binary: Path, case: dict[str, Any]) -> dict[str, Any]:
    case_dir = case["case_dir"]
    case_dir.mkdir(parents=True, exist_ok=True)
    json_path = case_dir / "fastaguard.json"
    html_path = case_dir / "fastaguard_report.html"
    tsv_path = case_dir / "fastaguard.tsv"
    multiqc_path = case_dir / "fastaguard_mqc.json"
    command = [
        str(binary),
        str(case["input_path"]),
        "--profile",
        "assembly",
        "--gate",
        "pipeline",
        "--min-contig-length",
        "1",
        "--out",
        str(html_path),
        "--json",
        str(json_path),
        "--tsv",
        str(tsv_path),
        "--multiqc",
        str(multiqc_path),
    ]

    started = time.perf_counter()
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    elapsed = time.perf_counter() - started

    if completed.returncode not in LOCAL_SUCCESS_CODES:
        raise SystemExit(
            "FastaGuard evidence run failed for "
            f"{case['id']} with exit code {completed.returncode}\n"
            f"Command: {' '.join(command)}\n"
            f"STDOUT:\n{completed.stdout}\nSTDERR:\n{completed.stderr}"
        )

    report = json.loads(json_path.read_text())
    summary = report["summary"]
    findings = report.get("findings", [])
    top_findings = [finding.get("id", "unknown") for finding in findings[:5]]
    gate = required_mapping(report, "gate", case["id"])
    provenance = required_mapping(report, "provenance", case["id"])
    gate_mode = required_value(gate, "mode", "gate.mode", case["id"])
    gate_status = required_value(gate, "status", "gate.status", case["id"])
    blocking_findings = required_list(
        gate, "blocking_findings", "gate.blocking_findings", case["id"]
    )
    input_sha256 = required_value(
        provenance, "input_sha256", "provenance.input_sha256", case["id"]
    )
    if not is_sha256(input_sha256):
        raise SystemExit(
            f"FastaGuard evidence report for {case['id']} has invalid "
            "provenance.input_sha256"
        )
    if gate_mode != "pipeline":
        raise SystemExit(
            f"FastaGuard evidence report for {case['id']} expected "
            f"gate.mode pipeline, got {gate_mode!r}"
        )
    if gate_status not in {"PASS", "WARN", "FAIL"}:
        raise SystemExit(
            f"FastaGuard evidence report for {case['id']} has invalid gate.status"
        )

    return {
        "id": case["id"],
        "label": case["label"],
        "category": case["category"],
        "source": case["source"],
        "accession": case.get("accession"),
        "input_path": str(case["input_path"]),
        "input_bytes": case["input_path"].stat().st_size,
        "elapsed_seconds": round(elapsed, 4),
        "exit_code": completed.returncode,
        "verdict": report["verdict"]["status"],
        "gate_mode": gate_mode,
        "gate_status": gate_status,
        "gate_blocking_findings": ",".join(blocking_findings),
        "input_sha256": input_sha256,
        "sequence_count": summary["sequence_count"],
        "total_length": summary["total_length"],
        "n50": summary["n50"],
        "n90": summary["n90"],
        "finding_count": len(findings),
        "top_findings": top_findings,
        "command": " ".join(command),
        "artifacts": {
            "json": str(json_path),
            "html": str(html_path),
            "tsv": str(tsv_path),
            "multiqc": str(multiqc_path),
        },
    }


def required_mapping(report: dict[str, Any], key: str, case_id: str) -> dict[str, Any]:
    value = report.get(key)
    if not isinstance(value, dict):
        raise SystemExit(f"FastaGuard evidence report for {case_id} missing {key}")
    return value


def required_value(
    mapping: dict[str, Any], key: str, label: str, case_id: str
) -> str:
    value = mapping.get(key)
    if not isinstance(value, str) or not value:
        raise SystemExit(f"FastaGuard evidence report for {case_id} missing {label}")
    return value


def required_list(
    mapping: dict[str, Any], key: str, label: str, case_id: str
) -> list[str]:
    value = mapping.get(key)
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise SystemExit(f"FastaGuard evidence report for {case_id} missing {label}")
    return value


def is_sha256(value: str) -> bool:
    return len(value) == 64 and all(character in "0123456789abcdef" for character in value)


def write_summary(out_dir: Path, summary: dict[str, Any]) -> None:
    json_path = out_dir / "evidence_summary.json"
    tsv_path = out_dir / "evidence_summary.tsv"
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")

    with tsv_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=SUMMARY_COLUMNS, delimiter="\t")
        writer.writeheader()
        for case in summary["cases"]:
            row = {key: case.get(key) for key in SUMMARY_COLUMNS}
            row["top_findings"] = ",".join(case["top_findings"])
            writer.writerow(row)


def fastaguard_version(binary: Path) -> str:
    completed = subprocess.run(
        [str(binary), "--version"], capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        return "unknown"
    return completed.stdout.strip() or "unknown"


def git_commit() -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return "unknown"
    return completed.stdout.strip() or "unknown"


if __name__ == "__main__":
    raise SystemExit(main())
