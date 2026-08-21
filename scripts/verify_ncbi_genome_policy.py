#!/usr/bin/env python3
"""Run an optional, normalized table2asn comparison over the NCBI FASTA corpus."""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.parse import urlsplit


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
MANIFEST_KEYS = {"cases", "policy_scope", "schema_version", "source_manifest"}
SCOPE_KEYS = {"excluded", "id", "included"}
CASE_KEYS = {"expected_table2asn_result", "fixture", "id"}
SOURCE_CASE_KEYS = {
    "description",
    "expect_can_continue",
    "expect_findings",
    "fixture",
    "id",
    "source_scope",
}
RESULT_CLASSES = {"accepted", "rejected"}
REQUIRED_EXCLUSIONS = {"annotation", "contamination", "submission_metadata"}
FASTAGUARD_TIMEOUT_SECONDS = 30.0
VALIDATION_COUNT_PATTERN = re.compile(
    r"(?i)\b(?:errors?|reject(?:ions?)?)\s*[:=]\s*(\d+)\b"
)
VALIDATION_ERROR_PATTERN = re.compile(
    r"(?i)(?:^|[\s\[])ERROR(?:\]|\s*:)|(?:^|[\s\[])REJECT(?:ED|ION)?(?:\]|\s*:)"
)
VALIDATION_MESSAGE_PATTERN = re.compile(
    r"(?i)(?:^|[\s\[])(?:INFO|WARNING|ERROR|REJECT(?:ED|ION)?)(?:\]|\s*:)"
)


class VerificationError(Exception):
    """Raised when the verifier input or one of its required tools is invalid."""


def parse_args():
    parser = argparse.ArgumentParser(
        description="Compare FastaGuard's NCBI genome FASTA policy with table2asn."
    )
    parser.add_argument("--fastaguard", required=True, type=Path)
    parser.add_argument("--table2asn", type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--require-table2asn", action="store_true")
    parser.add_argument("--table2asn-source-url")
    parser.add_argument("--table2asn-source-sha256")
    parser.add_argument(
        "--timeout",
        type=float,
        default=30.0,
        help="Per-process timeout in seconds (default: 30).",
    )
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be greater than zero")
    if bool(args.table2asn_source_url) != bool(args.table2asn_source_sha256):
        parser.error(
            "--table2asn-source-url and --table2asn-source-sha256 must be supplied together"
        )
    if args.table2asn_source_url:
        source = urlsplit(args.table2asn_source_url)
        if (
            source.scheme != "https"
            or source.hostname != "ftp.ncbi.nlm.nih.gov"
            or source.username is not None
            or source.password is not None
            or source.query
            or source.fragment
        ):
            parser.error(
                "--table2asn-source-url must be an unadorned HTTPS URL on ftp.ncbi.nlm.nih.gov"
            )
        if not re.fullmatch(r"[0-9a-fA-F]{64}", args.table2asn_source_sha256):
            parser.error("--table2asn-source-sha256 must be 64 hexadecimal characters")
        args.table2asn_source_sha256 = args.table2asn_source_sha256.lower()
    return args


def read_json(path, label):
    try:
        return json.loads(path.read_text())
    except OSError as error:
        raise VerificationError(f"cannot read {label}: {error.strerror}") from error
    except json.JSONDecodeError as error:
        raise VerificationError(f"invalid JSON in {label}: {error.msg}") from error


def require_exact_keys(value, required, label):
    if not isinstance(value, dict):
        raise VerificationError(f"{label} must be an object")
    actual = set(value)
    if actual != required:
        missing = sorted(required - actual)
        unexpected = sorted(actual - required)
        details = []
        if missing:
            details.append(f"missing {', '.join(missing)}")
        if unexpected:
            details.append(f"unexpected {', '.join(unexpected)}")
        raise VerificationError(f"{label} has invalid keys: {'; '.join(details)}")


def validate_string_list(value, label):
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(item, str) or not item for item in value)
    ):
        raise VerificationError(f"{label} must be a non-empty string array")
    if value != sorted(value):
        raise VerificationError(f"{label} must be sorted")
    if len(value) != len(set(value)):
        raise VerificationError(f"{label} contains duplicate values")


def resolve_source_manifest(manifest):
    source_value = manifest["source_manifest"]
    if not isinstance(source_value, str) or not source_value:
        raise VerificationError("source_manifest must be a non-empty repository-relative path")
    source_path = Path(source_value)
    if source_path.is_absolute() or ".." in source_path.parts:
        raise VerificationError("source_manifest must be a repository-relative path")
    return REPOSITORY_ROOT / source_path


def validate_manifest(path):
    manifest = read_json(path, "comparison manifest")
    require_exact_keys(manifest, MANIFEST_KEYS, "comparison manifest")
    if manifest["schema_version"] != "1.0.0":
        raise VerificationError("comparison manifest schema_version must be 1.0.0")

    scope = manifest["policy_scope"]
    require_exact_keys(scope, SCOPE_KEYS, "policy_scope")
    if scope["id"] != "ncbi_genome_fasta_overlap":
        raise VerificationError("policy_scope.id must be ncbi_genome_fasta_overlap")
    validate_string_list(scope["included"], "policy_scope.included")
    validate_string_list(scope["excluded"], "policy_scope.excluded")
    if not REQUIRED_EXCLUSIONS <= set(scope["excluded"]):
        raise VerificationError(
            "policy_scope.excluded must include annotation, contamination, and submission_metadata"
        )

    cases = manifest["cases"]
    if not isinstance(cases, list) or not cases:
        raise VerificationError("cases must be a non-empty array")
    for index, case in enumerate(cases):
        require_exact_keys(case, CASE_KEYS, f"cases[{index}]")
        if any(not isinstance(case[key], str) or not case[key] for key in CASE_KEYS):
            raise VerificationError(f"cases[{index}] values must be non-empty strings")
        if case["expected_table2asn_result"] not in RESULT_CLASSES:
            raise VerificationError(
                f"cases[{index}].expected_table2asn_result must be accepted or rejected"
            )
        fixture = Path(case["fixture"])
        if fixture.is_absolute() or len(fixture.parts) != 1:
            raise VerificationError(f"cases[{index}].fixture must be a file name")

    ids = [case["id"] for case in cases]
    fixtures = [case["fixture"] for case in cases]
    if len(ids) != len(set(ids)) or len(fixtures) != len(set(fixtures)):
        raise VerificationError("comparison manifest contains duplicate cases or fixtures")
    if ids != sorted(ids):
        raise VerificationError("comparison manifest cases are unsorted by id")

    source_path = resolve_source_manifest(manifest)
    source = read_json(source_path, "source policy manifest")
    if not isinstance(source, list):
        raise VerificationError("source policy manifest must be an array")
    source_by_id = {}
    for index, case in enumerate(source):
        require_exact_keys(case, SOURCE_CASE_KEYS, f"source cases[{index}]")
        source_by_id[case["id"]] = case

    overlap = {
        case_id: case
        for case_id, case in source_by_id.items()
        if case["source_scope"] == "table2asn_fasta_overlap"
    }
    if set(ids) != set(overlap):
        missing = sorted(set(overlap) - set(ids))
        extra = sorted(set(ids) - set(overlap))
        raise VerificationError(
            "comparison cases must exactly match source table2asn_fasta_overlap cases"
            f" (missing={missing}, extra={extra})"
        )

    for case in cases:
        source_case = overlap[case["id"]]
        if case["fixture"] != source_case["fixture"]:
            raise VerificationError(f"{case['id']} fixture differs from source manifest")
        expected = "accepted" if source_case["expect_can_continue"] else "rejected"
        if case["expected_table2asn_result"] != expected:
            raise VerificationError(
                f"{case['id']} expected_table2asn_result differs from source classification"
            )
        fixture_path = source_path.parent / source_case["fixture"]
        if not fixture_path.is_file():
            raise VerificationError(f"{case['id']} fixture does not exist")

    return manifest, source_path, overlap


def resolve_executable(explicit_path):
    if explicit_path is None:
        discovered = shutil.which("table2asn")
        return Path(discovered).resolve() if discovered else None
    candidate = explicit_path.expanduser()
    if candidate.parent == Path("."):
        discovered = shutil.which(str(candidate))
        if discovered:
            candidate = Path(discovered)
    if candidate.is_file() and os.access(candidate, os.X_OK):
        return candidate.resolve()
    return None


def normalized_command(command, replacements):
    normalized = []
    for token in command:
        text = str(token)
        for path, replacement in replacements:
            path_text = str(path)
            if text == path_text:
                text = replacement
            elif text.startswith(path_text + os.sep):
                suffix = text[len(path_text) + 1 :].replace(os.sep, "/")
                text = f"{replacement}/{suffix}"
        normalized.append(text)
    return normalized


def normalized_text(text, replacements):
    normalized = text.strip()
    for path, replacement in replacements:
        normalized = normalized.replace(str(path), replacement)
    return " ".join(normalized.split())


def write_result(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def source_provenance(args):
    if args.table2asn_source_url is None:
        return None
    return {
        "url": args.table2asn_source_url,
        "sha256": args.table2asn_source_sha256,
    }


def unavailable_result(manifest, args):
    return {
        "schema_version": "1.0.0",
        "policy_scope": manifest["policy_scope"],
        "table2asn_available": False,
        "table2asn_version": None,
        "table2asn_source": source_provenance(args),
        "comparison_performed": False,
        "cases": [],
    }


def run_version(table2asn, timeout, work_dir, replacements):
    command = [str(table2asn), "-help"]
    try:
        result = subprocess.run(
            command,
            cwd=work_dir,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unknown", normalized_command(command, replacements)
    version_text = result.stdout or result.stderr
    first_line = next((line for line in version_text.splitlines() if line.strip()), "unknown")
    return normalized_text(first_line, replacements), normalized_command(command, replacements)


def run_fastaguard(fastaguard, input_path, output_dir, timeout, replacements):
    report_path = output_dir / "fastaguard.json"
    command = [
        str(fastaguard),
        str(input_path),
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
        "--json",
        str(report_path),
        "--out",
        str(output_dir / "fastaguard.html"),
        "--tsv",
        str(output_dir / "fastaguard.tsv"),
        "--multiqc",
        str(output_dir / "fastaguard_mqc.json"),
    ]
    try:
        result = subprocess.run(
            command,
            cwd=output_dir,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise VerificationError("FastaGuard invocation timed out") from error
    except OSError as error:
        raise VerificationError(f"FastaGuard invocation failed: {error.strerror}") from error
    if result.returncode != 0:
        raise VerificationError(
            f"FastaGuard invocation returned exit code {result.returncode}"
        )
    report = read_json(report_path, "FastaGuard report")
    try:
        findings = sorted(finding["id"] for finding in report["findings"])
        can_continue = report["gate"]["can_continue"]
    except (KeyError, TypeError) as error:
        raise VerificationError("FastaGuard report does not match the required contract") from error
    if not isinstance(can_continue, bool) or any(
        not isinstance(finding, str) for finding in findings
    ):
        raise VerificationError("FastaGuard report contains invalid policy fields")
    return findings, can_continue, result.returncode, normalized_command(
        command, replacements
    )


def read_validation_artifact(path):
    try:
        return path.read_text()
    except (OSError, UnicodeDecodeError):
        return None


def classify_validation_artifact(input_path):
    validation_path = input_path.with_suffix(".val")
    stats_path = input_path.with_suffix(".stats")

    if validation_path.is_file():
        text = read_validation_artifact(validation_path)
        if text is None:
            return "tool_error", "unparseable_artifact", None
        counts = [int(value) for value in VALIDATION_COUNT_PATTERN.findall(text)]
        if text.strip() and not counts and VALIDATION_MESSAGE_PATTERN.search(text) is None:
            return "tool_error", "unparseable_artifact", None
        has_error = VALIDATION_ERROR_PATTERN.search(text) is not None
        result_class = "rejected" if has_error or any(counts) else "accepted"
        return result_class, None, {
            "type": "val",
            "path": validation_path.name,
        }

    if stats_path.is_file():
        text = read_validation_artifact(stats_path)
        if text is None:
            return "tool_error", "unparseable_artifact", None
        counts = [int(value) for value in VALIDATION_COUNT_PATTERN.findall(text)]
        if not counts:
            return "tool_error", "unparseable_artifact", None
        result_class = "rejected" if any(counts) else "accepted"
        return result_class, None, {
            "type": "stats",
            "path": stats_path.name,
        }

    return "tool_error", "missing_artifact", None


def run_table2asn(table2asn, input_path, output_dir, timeout, replacements):
    command = [
        str(table2asn),
        "-i",
        str(input_path),
        "-M",
        "n",
        "-j",
        "[organism=synthetic construct][moltype=genomic DNA]",
        "-V",
        "vb",
        "-Z",
        str(output_dir / "table2asn.discrepancy.txt"),
    ]
    try:
        result = subprocess.run(
            command,
            cwd=output_dir,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return (
            "tool_error",
            None,
            "timeout",
            normalized_command(command, replacements),
            None,
        )
    except OSError:
        return (
            "tool_error",
            None,
            "execution_error",
            normalized_command(command, replacements),
            None,
        )
    normalized = normalized_command(command, replacements)
    if result.returncode != 0:
        return "tool_error", result.returncode, "nonzero_exit", normalized, None
    result_class, error_kind, artifact = classify_validation_artifact(input_path)
    return result_class, result.returncode, error_kind, normalized, artifact


def run_comparison(args, manifest, source_path, source_by_id, table2asn):
    fastaguard = args.fastaguard.expanduser()
    if not fastaguard.is_file() or not os.access(fastaguard, os.X_OK):
        raise VerificationError("FastaGuard executable is missing or not executable")
    fastaguard = fastaguard.resolve()

    with TemporaryDirectory(prefix="fastaguard-ncbi-policy-") as temporary:
        work_root = Path(temporary).resolve()
        replacements = [
            (fastaguard, "$FASTAGUARD"),
            (table2asn, "$TABLE2ASN"),
            (fastaguard.parent, "$FASTAGUARD_DIR"),
            (table2asn.parent, "$TABLE2ASN_DIR"),
            (work_root, "$WORK"),
            (REPOSITORY_ROOT, "$REPOSITORY"),
        ]
        version, version_command = run_version(
            table2asn, args.timeout, work_root, replacements
        )
        results = []
        for case in manifest["cases"]:
            case_dir = work_root / case["id"]
            case_dir.mkdir()
            fixture_path = source_path.parent / case["fixture"]
            controlled_input = case_dir / case["fixture"]
            shutil.copyfile(fixture_path, controlled_input)

            findings, can_continue, fastaguard_exit_code, fastaguard_command = run_fastaguard(
                fastaguard,
                controlled_input,
                case_dir,
                FASTAGUARD_TIMEOUT_SECONDS,
                replacements,
            )
            source_case = source_by_id[case["id"]]
            if findings != source_case["expect_findings"]:
                raise VerificationError(
                    f"{case['id']} FastaGuard findings differ from source manifest"
                )
            if can_continue != source_case["expect_can_continue"]:
                raise VerificationError(
                    f"{case['id']} FastaGuard continuation differs from source manifest"
                )

            (
                result_class,
                exit_code,
                error_kind,
                table2asn_command,
                validation_artifact,
            ) = run_table2asn(
                table2asn,
                controlled_input,
                case_dir,
                args.timeout,
                replacements,
            )
            expected = case["expected_table2asn_result"]
            matches_expected = (
                result_class == expected if result_class != "tool_error" else None
            )
            results.append(
                {
                    "id": case["id"],
                    "fixture": case["fixture"],
                    "fastaguard_findings": findings,
                    "fastaguard_can_continue": can_continue,
                    "fastaguard_exit_code": fastaguard_exit_code,
                    "expected_table2asn_result": expected,
                    "table2asn_result": result_class,
                    "table2asn_exit_code": exit_code,
                    "tool_error_kind": error_kind,
                    "validation_artifact": validation_artifact,
                    "matches_expected": matches_expected,
                    "commands": {
                        "fastaguard": fastaguard_command,
                        "table2asn": table2asn_command,
                    },
                }
            )

    matched = sum(case["matches_expected"] is True for case in results)
    mismatched = sum(case["matches_expected"] is False for case in results)
    tool_errors = sum(case["table2asn_result"] == "tool_error" for case in results)
    result_counts = {
        result_class: sum(
            case["table2asn_result"] == result_class for case in results
        )
        for result_class in ("accepted", "rejected", "tool_error")
    }
    payload = {
        "schema_version": "1.0.0",
        "policy_scope": manifest["policy_scope"],
        "table2asn_available": True,
        "table2asn_version": version,
        "table2asn_source": source_provenance(args),
        "comparison_performed": True,
        "version_command": version_command,
        "comparison_summary": {
            "case_count": len(results),
            "matched_cases": matched,
            "mismatched_cases": mismatched,
            "tool_error_cases": tool_errors,
            "result_counts": result_counts,
        },
        "cases": results,
    }
    return payload, mismatched, tool_errors


def main():
    args = parse_args()
    try:
        manifest, source_path, source_by_id = validate_manifest(args.manifest)
        table2asn = resolve_executable(args.table2asn)
        if table2asn is None:
            write_result(args.out, unavailable_result(manifest, args))
            if args.require_table2asn:
                raise VerificationError("required table2asn executable is unavailable")
            return 0

        payload, mismatched, tool_errors = run_comparison(
            args, manifest, source_path, source_by_id, table2asn
        )
        write_result(args.out, payload)
        if args.require_table2asn and tool_errors:
            raise VerificationError(
                f"table2asn invocation failed for {tool_errors} case(s)"
            )
        if mismatched:
            raise VerificationError(
                f"table2asn result differed from the manifest for {mismatched} case(s)"
            )
        return 0
    except VerificationError as error:
        print(f"verifier error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
