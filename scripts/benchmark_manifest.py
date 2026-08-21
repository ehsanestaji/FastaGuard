#!/usr/bin/env python3
"""Run checksum-pinned FastaGuard benchmarks from a validated manifest."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.benchmark_large_fasta import ensure_unique_capacity, write_fasta


SHA256 = re.compile(r"[0-9a-f]{64}\Z")
CASE_KEYS = {
    "id",
    "accession",
    "assembly_version",
    "source_url",
    "sha256",
    "category",
    "expected_scale",
}
REQUIRED_CATEGORIES = {
    "bacterial",
    "fungal",
    "human-scale",
    "high-record-count-synthetic",
}
PUBLIC_CATEGORIES = REQUIRED_CATEGORIES - {"high-record-count-synthetic"}
NO_BASELINE_RUNTIME_CONTEXT = (
    "Elapsed seconds are contextual measurements from one pinned runner. "
    "No captured prior baseline was supplied; these measurements are not "
    "universal performance guarantees."
)
BASELINE_RUNTIME_CONTEXT = (
    "Elapsed ratios are contextual comparisons with a captured prior baseline "
    "from the same pinned runner context, not universal performance guarantees."
)
NO_BASELINE_CONTEXT = (
    "No captured prior baseline was supplied; no comparison or time/memory "
    "pass/fail threshold was applied."
)
BASELINE_CONTEXT = (
    "Each elapsed ratio compares the same case identity, input checksum, and "
    "expected scale on a matching pinned runner; no time/memory pass/fail "
    "threshold was applied."
)
SHARED_SUMMARY_COLUMNS = [
    "schema_version",
    "generated_at",
    "fastaguard_version",
    "runner_worktree_commit",
    "runner_worktree_dirty",
    "binary_sha256",
    "platform_system",
    "platform_release",
    "platform_machine",
    "python_version",
    "baseline_supplied",
    "runtime_context",
    "baseline_context",
]
CASE_SUMMARY_COLUMNS = [
    "id",
    "accession",
    "assembly_version",
    "source_url",
    "category",
    "expected_scale",
    "input_bytes",
    "input_sha256",
    "elapsed_seconds",
    "exit_code",
    "verdict",
    "sequence_count",
    "total_length",
    "n50",
    "n90",
    "scale_comparison",
    "prior_elapsed_seconds",
    "elapsed_ratio_to_prior",
]
SUMMARY_COLUMNS = SHARED_SUMMARY_COLUMNS + CASE_SUMMARY_COLUMNS


class BenchmarkManifestError(ValueError):
    """A benchmark manifest or publishable summary violates its contract."""


def main() -> int:
    args = parse_args()
    try:
        manifest = validate_manifest(read_json(args.manifest, "benchmark manifest"))
        if args.download and args.local_synthetic_only:
            raise BenchmarkManifestError(
                "--download cannot be combined with --local-synthetic-only"
            )

        binary = args.binary.resolve()
        if not binary.is_file():
            raise BenchmarkManifestError(f"FastaGuard binary not found: {binary}")

        out_dir = args.out_dir.resolve()
        out_dir.mkdir(parents=True, exist_ok=True)
        baseline = load_baseline(args.baseline)
        summary = run_manifest(manifest, binary, out_dir, args, baseline)
        validate_publishable_summary(summary)
        write_json(out_dir / "benchmark_summary.json", summary)
        write_tsv(out_dir / "benchmark_summary.tsv", summary)
    except BenchmarkManifestError as error:
        raise SystemExit(f"benchmark manifest error: {error}") from error

    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run checksum-pinned public or synthetic FastaGuard benchmarks."
    )
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    parser.add_argument(
        "--download",
        action="store_true",
        help="Explicitly allow downloads for public manifest cases.",
    )
    parser.add_argument(
        "--local-synthetic-only",
        action="store_true",
        help="Run only the deterministic high-record-count synthetic case.",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        help="Captured prior benchmark_summary.json from the same runner context.",
    )
    return parser.parse_args()


def read_json(path: Path, label: str) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BenchmarkManifestError(f"failed to read {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise BenchmarkManifestError(f"{label} must be a JSON object")
    return value


def validate_manifest(manifest: dict) -> dict:
    if set(manifest) != {"schema_version", "cases"}:
        raise BenchmarkManifestError(
            "manifest must contain exactly schema_version and cases"
        )
    if manifest["schema_version"] != 1:
        raise BenchmarkManifestError("schema_version must be 1")
    cases = manifest["cases"]
    if not isinstance(cases, list) or not cases:
        raise BenchmarkManifestError("cases must be a non-empty array")

    ids = []
    categories = set()
    for index, case in enumerate(cases):
        label = f"cases[{index}]"
        if not isinstance(case, dict):
            raise BenchmarkManifestError(f"{label} must be an object")
        missing = CASE_KEYS - set(case)
        extra = set(case) - CASE_KEYS
        if missing:
            raise BenchmarkManifestError(
                f"{label} missing required fields: {', '.join(sorted(missing))}"
            )
        if extra:
            raise BenchmarkManifestError(
                f"{label} has unknown fields: {', '.join(sorted(extra))}"
            )

        case_id = case["id"]
        if not isinstance(case_id, str) or not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", case_id):
            raise BenchmarkManifestError(f"{label} id must be a stable lowercase identifier")
        ids.append(case_id)

        category = case["category"]
        if category not in REQUIRED_CATEGORIES:
            raise BenchmarkManifestError(f"{case_id} has unsupported category {category!r}")
        categories.add(category)

        digest = case["sha256"]
        if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
            raise BenchmarkManifestError(f"{case_id} sha256 must be 64 lowercase hex digits")

        validate_expected_scale(case_id, case["expected_scale"], category)
        if category in PUBLIC_CATEGORIES:
            validate_public_case(case_id, case)
        elif any(case[field] is not None for field in ("accession", "assembly_version", "source_url")):
            raise BenchmarkManifestError(
                f"{case_id} synthetic accession, assembly_version, and source_url must be null"
            )

    if len(ids) != len(set(ids)):
        raise BenchmarkManifestError("manifest contains duplicate id values")
    if ids != sorted(ids):
        raise BenchmarkManifestError("manifest cases must be sorted by id")
    missing_categories = REQUIRED_CATEGORIES - categories
    if missing_categories:
        raise BenchmarkManifestError(
            "manifest missing categories: " + ", ".join(sorted(missing_categories))
        )
    return manifest


def validate_expected_scale(case_id: str, scale: object, category: str) -> None:
    if not isinstance(scale, dict):
        raise BenchmarkManifestError(f"{case_id} expected_scale must be an object")
    required = {"bases", "records"}
    if category == "high-record-count-synthetic":
        required.add("record_length")
    if set(scale) != required:
        raise BenchmarkManifestError(
            f"{case_id} expected_scale must contain exactly {', '.join(sorted(required))}"
        )
    for field in required:
        if not isinstance(scale[field], int) or scale[field] <= 0:
            raise BenchmarkManifestError(f"{case_id} expected_scale.{field} must be positive")
    if category == "high-record-count-synthetic" and (
        scale["bases"] != scale["records"] * scale["record_length"]
    ):
        raise BenchmarkManifestError(
            f"{case_id} expected_scale.bases must equal records times record_length"
        )


def validate_public_case(case_id: str, case: dict) -> None:
    for field in ("accession", "assembly_version", "source_url"):
        if not isinstance(case[field], str) or not case[field].strip():
            raise BenchmarkManifestError(f"{case_id} {field} must be non-empty")
    parsed = urlparse(case["source_url"])
    if parsed.scheme != "https" or not parsed.netloc:
        raise BenchmarkManifestError(f"{case_id} source_url must use HTTPS")


def validate_publishable_summary(summary: dict) -> dict:
    if not isinstance(summary, dict):
        raise BenchmarkManifestError("publishable summary must be an object")
    if "source_commit" in summary or "source_tree_dirty" in summary:
        raise BenchmarkManifestError(
            "publishable summary must use runner_worktree_commit and "
            "runner_worktree_dirty; the runner cannot attest the binary's source commit"
        )
    runner_commit = summary.get("runner_worktree_commit")
    if runner_commit != "unavailable" and (
        not isinstance(runner_commit, str)
        or re.fullmatch(r"[0-9a-f]{40}", runner_commit) is None
    ):
        raise BenchmarkManifestError(
            "runner_worktree_commit must be a 40-character Git commit or unavailable"
        )
    if not isinstance(summary.get("runner_worktree_dirty"), bool):
        raise BenchmarkManifestError("runner_worktree_dirty must be boolean")
    binary_sha256 = summary.get("binary_sha256")
    if not isinstance(binary_sha256, str) or SHA256.fullmatch(binary_sha256) is None:
        raise BenchmarkManifestError("binary_sha256 must be 64 lowercase hex digits")

    baseline_supplied = summary.get("baseline_supplied")
    if not isinstance(baseline_supplied, bool):
        raise BenchmarkManifestError("baseline_supplied must be boolean")
    expected_runtime = (
        BASELINE_RUNTIME_CONTEXT if baseline_supplied else NO_BASELINE_RUNTIME_CONTEXT
    )
    runtime_context = summary.get("runtime_context")
    if runtime_context != expected_runtime:
        mode = "baseline" if baseline_supplied else "no-baseline"
        raise BenchmarkManifestError(
            f"{mode} runtime_context must be mode-specific and reject universal performance claims"
        )
    expected_baseline_context = (
        BASELINE_CONTEXT if baseline_supplied else NO_BASELINE_CONTEXT
    )
    if summary.get("baseline_context") != expected_baseline_context:
        mode = "baseline" if baseline_supplied else "no-baseline"
        raise BenchmarkManifestError(f"{mode} baseline_context does not match execution mode")

    forbidden_keys = {
        "input_path",
        "output_path",
        "artifacts",
        "command",
        "sequence",
        "sequences",
        "sequence_data",
        "fasta_content",
        "performance_status",
        "runtime_threshold",
        "memory_threshold",
    }
    pending = [summary]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            overlap = forbidden_keys.intersection(value)
            if overlap:
                raise BenchmarkManifestError(
                    "publishable summary contains forbidden fields: "
                    + ", ".join(sorted(overlap))
                )
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
        elif isinstance(value, str) and (
            value.startswith("/") or re.match(r"^[A-Za-z]:[\\/]", value)
        ):
            raise BenchmarkManifestError(
                f"publishable summary contains an absolute path: {value}"
            )
    return summary


def load_baseline(path: Path | None) -> dict | None:
    if path is None:
        return None
    baseline = read_json(path, "prior baseline")
    return validate_publishable_summary(baseline)


def validate_baseline_case(case: dict, prior: dict) -> dict:
    current_identity = {
        "id": case["id"],
        "accession": case["accession"],
        "assembly_version": case["assembly_version"],
        "source_url": case["source_url"],
        "category": case["category"],
        "expected_scale": case["expected_scale"],
        "input_sha256": case["sha256"],
    }
    for field, current_value in current_identity.items():
        if prior.get(field) != current_value:
            raise BenchmarkManifestError(
                f"prior baseline {case['id']} {field} differs from the current case"
            )
    return prior


def run_manifest(
    manifest: dict,
    binary: Path,
    out_dir: Path,
    args: argparse.Namespace,
    baseline: dict | None,
) -> dict:
    runner = {
        "platform_system": platform.system(),
        "platform_release": platform.release(),
        "platform_machine": platform.machine(),
        "python_version": platform.python_version(),
    }
    if baseline is not None:
        for field, current in runner.items():
            if baseline.get(field) != current:
                raise BenchmarkManifestError(
                    f"prior baseline {field} differs from the current pinned runner"
                )

    selected = [
        case
        for case in manifest["cases"]
        if not args.local_synthetic_only
        or case["category"] == "high-record-count-synthetic"
    ]
    if not selected:
        raise BenchmarkManifestError("no benchmark cases selected")

    prior_by_id = {
        case["id"]: case for case in baseline.get("cases", [])
    } if baseline else {}
    if baseline is not None:
        missing_prior_cases = [
            case["id"] for case in selected if case["id"] not in prior_by_id
        ]
        if missing_prior_cases:
            raise BenchmarkManifestError(
                "prior baseline is missing selected cases: "
                + ", ".join(missing_prior_cases)
                + "; capture a baseline with the same selected cases"
            )

    if not args.local_synthetic_only and not args.download:
        public = [case["id"] for case in selected if case["category"] in PUBLIC_CATEGORIES]
        if public:
            raise BenchmarkManifestError(
                "public inputs are not downloaded by default; rerun with --download: "
                + ", ".join(public)
            )

    cases = []
    for case in selected:
        prior = prior_by_id.get(case["id"])
        if prior is not None:
            validate_baseline_case(case, prior)
        case_dir = out_dir / "runs" / case["id"]
        case_dir.mkdir(parents=True, exist_ok=True)
        input_path = prepare_input(case, out_dir, case_dir, args.download)
        actual_sha256 = sha256_path(input_path)
        if actual_sha256 != case["sha256"]:
            raise BenchmarkManifestError(
                f"{case['id']} SHA-256 mismatch before analysis: "
                f"expected {case['sha256']}, got {actual_sha256}"
            )
        cases.append(
            run_case(binary, input_path, case_dir, case, actual_sha256, prior)
        )

    runner_worktree_commit = git_value(["rev-parse", "HEAD"])
    summary = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "fastaguard_version": fastaguard_version(binary),
        "runner_worktree_commit": runner_worktree_commit,
        "runner_worktree_dirty": git_dirty(),
        "binary_sha256": sha256_path(binary),
        **runner,
        "baseline_supplied": baseline is not None,
        "runtime_context": (
            BASELINE_RUNTIME_CONTEXT if baseline else NO_BASELINE_RUNTIME_CONTEXT
        ),
        "baseline_context": BASELINE_CONTEXT if baseline else NO_BASELINE_CONTEXT,
        "cases": cases,
    }
    return summary


def prepare_input(case: dict, out_dir: Path, case_dir: Path, download: bool) -> Path:
    if case["category"] == "high-record-count-synthetic":
        scale = case["expected_scale"]
        ensure_unique_capacity(scale["records"], scale["record_length"])
        path = case_dir / "synthetic.fa"
        write_fasta(path, scale["records"], scale["record_length"])
        return path

    if not download:
        raise BenchmarkManifestError(f"{case['id']} requires explicit --download")
    input_dir = out_dir / "inputs"
    input_dir.mkdir(parents=True, exist_ok=True)
    destination = input_dir / f"{case['id']}.fna.gz"
    download_verified(case["source_url"], destination, case["sha256"])
    return destination


def download_verified(url: str, destination: Path, expected_sha256: str) -> None:
    temporary = destination.with_name(destination.name + ".download")
    digest = hashlib.sha256()
    try:
        request = urllib.request.Request(url, headers={"User-Agent": "FastaGuard-benchmark/0.7"})
        with urllib.request.urlopen(request) as response, temporary.open("wb") as handle:
            while chunk := response.read(1024 * 1024):
                digest.update(chunk)
                handle.write(chunk)
        actual = digest.hexdigest()
        if actual != expected_sha256:
            raise BenchmarkManifestError(
                f"download SHA-256 mismatch: expected {expected_sha256}, got {actual}"
            )
        os.replace(temporary, destination)
    finally:
        temporary.unlink(missing_ok=True)


def run_case(
    binary: Path,
    input_path: Path,
    case_dir: Path,
    case: dict,
    input_sha256: str,
    prior: dict | None,
) -> dict:
    command = [
        str(binary),
        str(input_path),
        "--profile",
        "assembly",
        "--min-contig-length",
        "1",
        "--outdir",
        str(case_dir),
        "--prefix",
        "report",
        "--force",
    ]
    started = time.perf_counter()
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        raise BenchmarkManifestError(
            f"{case['id']} analysis failed with exit code {completed.returncode}: "
            f"{completed.stderr.strip()}"
        )

    report_path = case_dir / "report.fastaguard.json"
    report = read_json(report_path, f"{case['id']} report")
    prior_elapsed = prior.get("elapsed_seconds") if prior else None
    elapsed_ratio = (
        round(elapsed / prior_elapsed, 4)
        if isinstance(prior_elapsed, (int, float)) and prior_elapsed > 0
        else None
    )
    return {
        "id": case["id"],
        "accession": case["accession"],
        "assembly_version": case["assembly_version"],
        "source_url": case["source_url"],
        "category": case["category"],
        "expected_scale": case["expected_scale"],
        "input_bytes": input_path.stat().st_size,
        "input_sha256": input_sha256,
        "elapsed_seconds": round(elapsed, 4),
        "exit_code": completed.returncode,
        "verdict": report["verdict"]["status"],
        "sequence_count": report["summary"]["sequence_count"],
        "total_length": report["summary"]["total_length"],
        "n50": report["summary"]["n50"],
        "n90": report["summary"]["n90"],
        "scale_comparison": compare_observed_scale(case, report),
        "prior_elapsed_seconds": prior_elapsed,
        "elapsed_ratio_to_prior": elapsed_ratio,
    }


def compare_observed_scale(case: dict, report: dict) -> dict:
    expected = case["expected_scale"]
    observed_bases = report["summary"]["total_length"]
    observed_records = report["summary"]["sequence_count"]
    return {
        "expected_bases": expected["bases"],
        "observed_bases": observed_bases,
        "bases_delta": observed_bases - expected["bases"],
        "expected_records": expected["records"],
        "observed_records": observed_records,
        "records_delta": observed_records - expected["records"],
        "exact_match": observed_bases == expected["bases"]
        and observed_records == expected["records"],
    }


def fastaguard_version(binary: Path) -> str:
    completed = subprocess.run(
        [str(binary), "--version"], capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        raise BenchmarkManifestError("failed to read FastaGuard version")
    return completed.stdout.strip()


def git_value(arguments: list[str]) -> str:
    completed = subprocess.run(
        ["git", *arguments], cwd=ROOT, capture_output=True, text=True, check=False
    )
    return completed.stdout.strip() if completed.returncode == 0 else "unavailable"


def git_dirty() -> bool:
    completed = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return completed.returncode != 0 or bool(completed.stdout.strip())


def sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_tsv(path: Path, summary: dict) -> None:
    shared = {field: summary[field] for field in SHARED_SUMMARY_COLUMNS}
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=SUMMARY_COLUMNS, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for case in summary["cases"]:
            row = {**shared, **case}
            row["expected_scale"] = json.dumps(row["expected_scale"], sort_keys=True, separators=(",", ":"))
            row["scale_comparison"] = json.dumps(
                row["scale_comparison"], sort_keys=True, separators=(",", ":")
            )
            writer.writerow(row)


if __name__ == "__main__":
    raise SystemExit(main())
