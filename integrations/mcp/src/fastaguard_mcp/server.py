"""Stdio Model Context Protocol (MCP) server backed by the FastaGuard CLI."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any, BinaryIO

SERVER_NAME = "fastaguard-mcp"
SERVER_VERSION = "0.1.0"
DEFAULT_PROTOCOL_VERSION = "2024-11-05"

EXIT_CODE_MEANINGS: dict[int, str] = {
    0: "completed",
    2: "argument_parsing_error",
    3: "configuration_io_or_runtime_error",
}

SCOPE_BOUNDARY_NOTE = (
    "FastaGuard MCP reports only deterministic CLI contract outputs. "
    "It does not fabricate findings outside the FastaGuard JSON report, "
    "does not query external biological databases, and never modifies input FASTA files."
)

TOOL_DEFINITIONS: list[dict[str, Any]] = [
    {
        "name": "fastaguard_schema",
        "description": (
            "Return the formal FastaGuard JSON Schema from `fastaguard --schema`. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "properties": {},
            "additionalProperties": False,
        },
        "annotations": {
            "title": "FastaGuard JSON Schema",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
    {
        "name": "fastaguard_finding_catalog",
        "description": (
            "Return the machine-readable FastaGuard finding catalog from "
            "`fastaguard --finding-catalog`. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "properties": {},
            "additionalProperties": False,
        },
        "annotations": {
            "title": "FastaGuard Finding Catalog",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
    {
        "name": "fastaguard_explain_finding",
        "description": (
            "Return the catalog definition for one FastaGuard finding ID via "
            "`fastaguard --explain-finding <id>`. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Stable FastaGuard finding ID (e.g. 'high_n_rate' or 'reference_length_mismatch').",
                }
            },
            "additionalProperties": False,
        },
        "annotations": {
            "title": "Explain FastaGuard Finding",
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
    {
        "name": "fastaguard_inspect_fasta",
        "description": (
            "Run single-FASTA preflight QC (`fastaguard <input>`) and return the "
            "structured JSON report including verdict, gate.can_continue, readiness, "
            "findings, machine_summary, and scope. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "required": ["input"],
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Path to the input FASTA file (.fa, .fasta, or .gz).",
                },
                "profile": {
                    "type": "string",
                    "enum": ["assembly", "protein"],
                    "description": "QC profile ('assembly' or 'protein').",
                },
                "gate": {
                    "type": "string",
                    "enum": ["none", "pipeline", "submission"],
                    "description": "Gate preset for report blocking policy.",
                },
                "submission_target": {
                    "type": "string",
                    "enum": ["generic", "ncbi"],
                    "description": "Submission-readiness target to record.",
                },
                "fail_on": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "Finding IDs (list or comma-separated string) that mark the report as FAIL when triggered.",
                },
                "outdir": {
                    "type": "string",
                    "description": "Optional directory in which to persist the four-report bundle.",
                },
                "prefix": {
                    "type": "string",
                    "description": "Optional filename stem when writing a report bundle with outdir.",
                },
                "force": {
                    "type": "boolean",
                    "description": "Allow existing report files in outdir to be replaced.",
                },
                "max_n_rate": {
                    "type": "number",
                    "description": "Maximum allowed global N fraction before a high_n_rate finding.",
                },
                "min_contig_length": {
                    "type": "integer",
                    "description": "Minimum contig length used for tiny_contigs finding.",
                },
                "expected_size": {
                    "type": "string",
                    "description": "Expected assembly size using decimal bases, kb, mb, or gb units.",
                },
                "expected_size_tolerance": {
                    "type": "number",
                    "description": "Fractional tolerance around expected_size.",
                },
            },
            "additionalProperties": False,
        },
        "annotations": {
            "title": "Inspect Single FASTA",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
    {
        "name": "fastaguard_compare",
        "description": (
            "Run cohort comparison (`fastaguard compare <inputs...>`) across two or more "
            "FASTA files and return the structured cohort JSON report. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "required": ["inputs"],
            "properties": {
                "inputs": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 2,
                    "description": "Paths to two or more input FASTA files.",
                },
                "profile": {
                    "type": "string",
                    "enum": ["assembly", "protein"],
                    "description": "QC profile ('assembly' or 'protein').",
                },
                "gate": {
                    "type": "string",
                    "enum": ["none", "pipeline", "submission"],
                    "description": "Gate preset for report blocking policy.",
                },
                "submission_target": {
                    "type": "string",
                    "enum": ["generic", "ncbi"],
                    "description": "Submission-readiness target to record.",
                },
                "fail_on": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "Finding IDs (list or comma-separated string) that mark a sample report as FAIL when triggered.",
                },
                "outdir": {
                    "type": "string",
                    "description": "Optional directory in which to persist cohort report artifacts.",
                },
                "max_n_rate": {
                    "type": "number",
                    "description": "Maximum allowed global N fraction before a high_n_rate finding.",
                },
                "min_contig_length": {
                    "type": "integer",
                    "description": "Minimum contig length used for tiny_contigs finding.",
                },
                "expected_size": {
                    "type": "string",
                    "description": "Expected assembly size using decimal bases, kb, mb, or gb units.",
                },
                "expected_size_tolerance": {
                    "type": "number",
                    "description": "Fractional tolerance around expected_size.",
                },
            },
            "additionalProperties": False,
        },
        "annotations": {
            "title": "Compare FASTA Cohort",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
    {
        "name": "fastaguard_reference",
        "description": (
            "Verify that declared reference companions (.fai, .dict, alignment headers, "
            "variant headers, GFF3 annotations) match a canonical reference FASTA via "
            "`fastaguard reference <reference>` and return the structured reference JSON report. "
            + SCOPE_BOUNDARY_NOTE
        ),
        "inputSchema": {
            "type": "object",
            "required": ["reference"],
            "properties": {
                "reference": {
                    "type": "string",
                    "description": "Path to the canonical reference FASTA.",
                },
                "fai": {
                    "type": "string",
                    "description": "FASTA index (.fai) declaration path.",
                },
                "dict": {
                    "type": "string",
                    "description": "SAM sequence dictionary (.dict) declaration path.",
                },
                "alignment": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "Alignment header path(s) (.sam, .bam, .cram) to compare with the canonical reference.",
                },
                "variants": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "Variant header path(s) (.vcf, .vcf.gz, .bcf) to compare with the canonical reference.",
                },
                "annotation": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "GFF3 annotation path(s) to compare with the canonical reference.",
                },
                "alias_map": {
                    "type": "string",
                    "description": "Two-column TSV mapping from declared names to canonical reference names.",
                },
                "require": {
                    "oneOf": [
                        {"type": "array", "items": {"type": "string"}},
                        {"type": "string"},
                    ],
                    "description": "Required declaration kinds (fai, dict, alignment, variants, annotation).",
                },
                "policy": {
                    "type": "string",
                    "enum": ["strict", "coordinate", "advisory"],
                    "description": "Compatibility policy for declared reference artifacts.",
                },
                "outdir": {
                    "type": "string",
                    "description": "Optional directory for a deterministic reference report bundle.",
                },
                "prefix": {
                    "type": "string",
                    "description": "Optional filename stem for reports written with outdir.",
                },
                "write_lock": {
                    "type": "boolean",
                    "description": "Write a reference manifest lock file alongside the report bundle.",
                },
                "force": {
                    "type": "boolean",
                    "description": "Allow existing reference report files to be replaced.",
                },
            },
            "additionalProperties": False,
        },
        "annotations": {
            "title": "Verify Reference Contract",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    },
]

TOOLS_BY_NAME: dict[str, dict[str, Any]] = {
    tool["name"]: tool for tool in TOOL_DEFINITIONS
}


def resolve_fastaguard_binary(binary: str | Path | None = None) -> str:
    """Locate the fastaguard CLI executable."""
    if binary is not None and str(binary).strip():
        return str(binary)

    for env_key in ("FASTAGUARD_BIN", "FASTAGUARD_BINARY"):
        env_val = os.environ.get(env_key, "").strip()
        if env_val:
            return env_val

    which_path = shutil.which("fastaguard")
    if which_path:
        return which_path

    repo_root = Path(__file__).resolve().parents[4]
    candidates = [
        Path.cwd() / "target" / "release" / "fastaguard",
        Path.cwd() / "target" / "debug" / "fastaguard",
        repo_root / "target" / "release" / "fastaguard",
        repo_root / "target" / "debug" / "fastaguard",
    ]
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate)

    return "fastaguard"


def _normalize_csv_or_list(value: Any) -> str | None:
    if value is None:
        return None
    if isinstance(value, (list, tuple)):
        items = [str(item).strip() for item in value if str(item).strip()]
        return ",".join(items) if items else None
    text = str(value).strip()
    return text if text else None


def _normalize_multi_paths(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, (list, tuple)):
        return [str(item) for item in value]
    return [str(value)]


def _append_analysis_args(cmd: list[str], arguments: dict[str, Any]) -> None:
    if arguments.get("profile") is not None:
        cmd.extend(["--profile", str(arguments["profile"])])
    if arguments.get("gate") is not None:
        cmd.extend(["--gate", str(arguments["gate"])])
    if arguments.get("submission_target") is not None:
        cmd.extend(["--submission-target", str(arguments["submission_target"])])
    fail_on = _normalize_csv_or_list(arguments.get("fail_on"))
    if fail_on is not None:
        cmd.extend(["--fail-on", fail_on])
    if arguments.get("max_n_rate") is not None:
        cmd.extend(["--max-n-rate", str(arguments["max_n_rate"])])
    if arguments.get("min_contig_length") is not None:
        cmd.extend(["--min-contig-length", str(arguments["min_contig_length"])])
    if arguments.get("expected_size") is not None:
        cmd.extend(["--expected-size", str(arguments["expected_size"])])
    if arguments.get("expected_size_tolerance") is not None:
        cmd.extend(
            ["--expected-size-tolerance", str(arguments["expected_size_tolerance"])]
        )


def _check_no_input_overwrite(
    input_paths: list[Path], output_paths: list[Path]
) -> str | None:
    resolved_inputs = {path.resolve() for path in input_paths}
    for out_path in output_paths:
        if out_path.resolve() in resolved_inputs:
            return f"refusing to overwrite input file {out_path}"
    return None


class FastaguardMcpServer:
    """Stdio JSON-RPC 2.0 server exposing FastaGuard contract and QC tools."""

    def __init__(self, binary: str | Path | None = None, cwd: str | Path | None = None):
        self.binary = binary
        self.cwd = Path(cwd) if cwd is not None else None

    def _resolve_path(self, path: str | Path) -> Path:
        candidate = Path(path)
        if not candidate.is_absolute() and self.cwd is not None:
            return self.cwd / candidate
        return candidate

    def resolve_binary(self) -> str:
        return resolve_fastaguard_binary(self.binary)

    def _run_cli(self, args: list[str]) -> tuple[int, str, str, list[str]]:
        cmd = [self.resolve_binary(), *args]
        try:
            completed = subprocess.run(
                cmd,
                cwd=self.cwd,
                capture_output=True,
                text=True,
                check=False,
            )
            return completed.returncode, completed.stdout, completed.stderr, cmd
        except OSError as exc:
            return 3, "", f"fastaguard execution error: {exc}", cmd

    @staticmethod
    def _build_tool_result(
        exit_code: int,
        payload: dict[str, Any],
        command: list[str],
        is_error: bool,
    ) -> dict[str, Any]:
        meaning = EXIT_CODE_MEANINGS.get(exit_code, "unexpected_exit_code")
        return {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps(payload, indent=2),
                }
            ],
            "structuredContent": payload,
            "isError": is_error,
            "exit_code": exit_code,
            "_meta": {
                "exit_code": exit_code,
                "exit_meaning": meaning,
                "command": command,
            },
        }

    def _build_cli_error_result(
        self, exit_code: int, stderr: str, stdout: str, command: list[str]
    ) -> dict[str, Any]:
        meaning = EXIT_CODE_MEANINGS.get(exit_code, "unexpected_exit_code")
        message = stderr.strip() or stdout.strip() or f"CLI exited with code {exit_code}"
        error_payload = {
            "exit_code": exit_code,
            "exit_meaning": meaning,
            "error": message,
            "command": command,
        }
        return self._build_tool_result(
            exit_code=exit_code,
            payload=error_payload,
            command=command,
            is_error=True,
        )

    def _validate_known_arguments(
        self, tool_name: str, arguments: dict[str, Any]
    ) -> dict[str, Any] | None:
        schema = TOOLS_BY_NAME[tool_name]["inputSchema"]
        allowed = set(schema.get("properties", {}).keys())
        unknown = sorted(set(arguments.keys()) - allowed)
        if unknown:
            return self._build_cli_error_result(
                exit_code=2,
                stderr=f"unknown argument(s) for {tool_name}: {', '.join(unknown)}",
                stdout="",
                command=[self.resolve_binary()],
            )
        return None

    def call_tool(
        self, name: str, arguments: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        """Execute a FastaGuard MCP tool and return a CallToolResult dictionary."""
        if arguments is None:
            arguments = {}
        if not isinstance(arguments, dict):
            return self._build_cli_error_result(
                exit_code=2,
                stderr="tool arguments must be a JSON object",
                stdout="",
                command=[self.resolve_binary()],
            )

        unknown_err = self._validate_known_arguments(name, arguments)
        if unknown_err is not None:
            return unknown_err

        if name == "fastaguard_schema":
            return self._tool_schema()
        if name == "fastaguard_finding_catalog":
            return self._tool_finding_catalog()
        if name == "fastaguard_explain_finding":
            return self._tool_explain_finding(arguments)
        if name == "fastaguard_inspect_fasta":
            return self._tool_inspect_fasta(arguments)
        if name == "fastaguard_compare":
            return self._tool_compare(arguments)
        if name == "fastaguard_reference":
            return self._tool_reference(arguments)

        raise KeyError(f"Unknown tool: {name}")

    def _tool_schema(self) -> dict[str, Any]:
        code, stdout, stderr, cmd = self._run_cli(["--schema"])
        if code != 0:
            return self._build_cli_error_result(code, stderr, stdout, cmd)
        parsed = json.loads(stdout)
        return self._build_tool_result(0, parsed, cmd, is_error=False)

    def _tool_finding_catalog(self) -> dict[str, Any]:
        code, stdout, stderr, cmd = self._run_cli(["--finding-catalog"])
        if code != 0:
            return self._build_cli_error_result(code, stderr, stdout, cmd)
        parsed = json.loads(stdout)
        return self._build_tool_result(0, parsed, cmd, is_error=False)

    def _tool_explain_finding(self, arguments: dict[str, Any]) -> dict[str, Any]:
        finding_id = arguments.get("id")
        if finding_id is None or not isinstance(finding_id, str) or not finding_id.strip():
            code, stdout, stderr, cmd = self._run_cli(["--explain-finding"])
            return self._build_cli_error_result(
                code if code != 0 else 2, stderr, stdout, cmd
            )

        code, stdout, stderr, cmd = self._run_cli(["--explain-finding", finding_id])
        if code != 0:
            return self._build_cli_error_result(code, stderr, stdout, cmd)
        parsed = json.loads(stdout)
        return self._build_tool_result(0, parsed, cmd, is_error=False)

    def _tool_inspect_fasta(self, arguments: dict[str, Any]) -> dict[str, Any]:
        input_path = arguments.get("input")
        args: list[str] = []
        if input_path is not None and str(input_path).strip():
            args.append(str(input_path))

        _append_analysis_args(args, arguments)
        if arguments.get("force") is True:
            args.append("--force")

        outdir = arguments.get("outdir")
        prefix = arguments.get("prefix")

        if outdir is not None:
            stem = str(prefix) if prefix is not None else "fastaguard"
            out_dir_path = self._resolve_path(str(outdir))
            json_path = out_dir_path / f"{stem}.fastaguard.json"
            bundle_paths = [
                out_dir_path / f"{stem}.fastaguard.html",
                json_path,
                out_dir_path / f"{stem}.fastaguard.tsv",
                out_dir_path / f"{stem}.fastaguard_mqc.json",
            ]
            collision = _check_no_input_overwrite(
                [self._resolve_path(str(input_path))] if input_path else [],
                bundle_paths,
            )
            if collision:
                return self._build_cli_error_result(
                    3, collision, "", [self.resolve_binary(), *args]
                )

            args.extend(["--outdir", str(outdir)])
            if prefix is not None:
                args.extend(["--prefix", str(prefix)])

            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

        if prefix is not None:
            # Passing --prefix without --outdir lets clap return exit code 2.
            args.extend(["--prefix", str(prefix)])
            code, stdout, stderr, cmd = self._run_cli(args)
            return self._build_cli_error_result(
                code if code != 0 else 2, stderr, stdout, cmd
            )

        with TemporaryDirectory(prefix="fastaguard-mcp-inspect-") as tmp:
            tmp_path = Path(tmp)
            json_path = tmp_path / "fastaguard.fastaguard.json"
            args.extend(["--outdir", str(tmp_path)])

            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

    def _tool_compare(self, arguments: dict[str, Any]) -> dict[str, Any]:
        raw_inputs = arguments.get("inputs")
        if raw_inputs is None:
            inputs: list[str] = []
        elif isinstance(raw_inputs, list):
            inputs = [str(item) for item in raw_inputs]
        else:
            return self._build_cli_error_result(
                exit_code=2,
                stderr="'inputs' must be an array of FASTA file paths",
                stdout="",
                command=[self.resolve_binary(), "compare"],
            )

        args: list[str] = ["compare", *inputs]
        _append_analysis_args(args, arguments)

        outdir = arguments.get("outdir")
        if outdir is not None:
            target_dir = self._resolve_path(str(outdir))
            target_dir.mkdir(parents=True, exist_ok=True)
            html_path = target_dir / "cohort_report.html"
            json_path = target_dir / "cohort.json"
            tsv_path = target_dir / "cohort.tsv"
            mqc_path = target_dir / "fastaguard_compare_mqc.json"
            collision = _check_no_input_overwrite(
                [self._resolve_path(item) for item in inputs if item],
                [html_path, json_path, tsv_path, mqc_path],
            )
            if collision:
                return self._build_cli_error_result(
                    3, collision, "", [self.resolve_binary(), *args]
                )
            args.extend(
                [
                    "--out",
                    str(html_path),
                    "--json",
                    str(json_path),
                    "--tsv",
                    str(tsv_path),
                    "--multiqc",
                    str(mqc_path),
                ]
            )
            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

        with TemporaryDirectory(prefix="fastaguard-mcp-compare-") as tmp:
            target_dir = Path(tmp)
            json_path = target_dir / "cohort.json"
            args.extend(
                [
                    "--out",
                    str(target_dir / "cohort_report.html"),
                    "--json",
                    str(json_path),
                    "--tsv",
                    str(target_dir / "cohort.tsv"),
                    "--multiqc",
                    str(target_dir / "fastaguard_compare_mqc.json"),
                ]
            )
            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

    def _tool_reference(self, arguments: dict[str, Any]) -> dict[str, Any]:
        reference = arguments.get("reference")
        args: list[str] = ["reference"]
        input_files: list[str] = []
        if reference is not None and str(reference).strip():
            ref_str = str(reference)
            args.append(ref_str)
            input_files.append(ref_str)

        if arguments.get("fai") is not None:
            fai_str = str(arguments["fai"])
            args.extend(["--fai", fai_str])
            input_files.append(fai_str)

        if arguments.get("dict") is not None:
            dict_str = str(arguments["dict"])
            args.extend(["--dict", dict_str])
            input_files.append(dict_str)

        for align_path in _normalize_multi_paths(arguments.get("alignment")):
            args.extend(["--alignment", align_path])
            input_files.append(align_path)

        for var_path in _normalize_multi_paths(arguments.get("variants")):
            args.extend(["--variants", var_path])
            input_files.append(var_path)

        for ann_path in _normalize_multi_paths(arguments.get("annotation")):
            args.extend(["--annotation", ann_path])
            input_files.append(ann_path)

        if arguments.get("alias_map") is not None:
            alias_str = str(arguments["alias_map"])
            args.extend(["--alias-map", alias_str])
            input_files.append(alias_str)

        require = _normalize_csv_or_list(arguments.get("require"))
        if require is not None:
            args.extend(["--require", require])

        if arguments.get("policy") is not None:
            args.extend(["--policy", str(arguments["policy"])])

        if arguments.get("write_lock") is True:
            args.append("--write-lock")

        if arguments.get("force") is True:
            args.append("--force")

        outdir = arguments.get("outdir")
        prefix = arguments.get("prefix")

        if outdir is not None:
            stem = str(prefix) if prefix is not None else "reference"
            out_dir_path = self._resolve_path(str(outdir))
            json_path = out_dir_path / f"{stem}.fastaguard.json"
            bundle_paths = [
                out_dir_path / f"{stem}.fastaguard.html",
                json_path,
                out_dir_path / f"{stem}.fastaguard.tsv",
                out_dir_path / f"{stem}.fastaguard_mqc.json",
                out_dir_path / f"{stem}.fastaguard.lock.json",
            ]
            collision = _check_no_input_overwrite(
                [self._resolve_path(item) for item in input_files if item],
                bundle_paths,
            )
            if collision:
                return self._build_cli_error_result(
                    3, collision, "", [self.resolve_binary(), *args]
                )

            args.extend(["--outdir", str(outdir)])
            if prefix is not None:
                args.extend(["--prefix", str(prefix)])

            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

        if prefix is not None:
            # Passing --prefix without --outdir lets clap return exit code 2.
            args.extend(["--prefix", str(prefix)])
            code, stdout, stderr, cmd = self._run_cli(args)
            return self._build_cli_error_result(
                code if code != 0 else 2, stderr, stdout, cmd
            )

        with TemporaryDirectory(prefix="fastaguard-mcp-reference-") as tmp:
            tmp_path = Path(tmp)
            json_path = tmp_path / "reference.fastaguard.json"
            args.extend(["--outdir", str(tmp_path)])

            code, stdout, stderr, cmd = self._run_cli(args)
            if code != 0:
                return self._build_cli_error_result(code, stderr, stdout, cmd)
            report = json.loads(json_path.read_text(encoding="utf-8"))
            return self._build_tool_result(0, report, cmd, is_error=False)

    def handle_message(self, message: Any) -> dict[str, Any] | None:
        """Process one decoded JSON-RPC 2.0 request or notification."""
        if not isinstance(message, dict) or message.get("jsonrpc") != "2.0":
            return {
                "jsonrpc": "2.0",
                "id": message.get("id") if isinstance(message, dict) else None,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request",
                },
            }

        method = message.get("method")
        has_id = "id" in message
        req_id = message.get("id")
        params = message.get("params") or {}

        if not isinstance(method, str) or not method:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32600,
                    "message": "Invalid Request: missing method",
                },
            }

        # Notifications do not receive a JSON-RPC response.
        if not has_id or method.startswith("notifications/"):
            return None

        if not isinstance(params, dict):
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32602,
                    "message": "Invalid params: expected object",
                },
            }

        if method == "initialize":
            requested_version = params.get("protocolVersion")
            protocol_version = (
                requested_version
                if isinstance(requested_version, str) and requested_version.strip()
                else DEFAULT_PROTOCOL_VERSION
            )
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": protocol_version,
                    "capabilities": {
                        "tools": {
                            "listChanged": False,
                        }
                    },
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": SERVER_VERSION,
                    },
                },
            }

        if method == "ping":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {},
            }

        if method == "tools/list":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": TOOL_DEFINITIONS,
                },
            }

        if method == "tools/call":
            tool_name = params.get("name")
            if not isinstance(tool_name, str) or tool_name not in TOOLS_BY_NAME:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32602,
                        "message": f"Unknown tool: {tool_name}",
                    },
                }
            arguments = params.get("arguments")
            if arguments is None:
                arguments = {}
            result = self.call_tool(tool_name, arguments)
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": result,
            }

        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": f"Method not found: {method}",
            },
        }

    @staticmethod
    def _read_next_message(stream: BinaryIO) -> tuple[bytes | None, bool]:
        """Read one JSON-RPC payload supporting Content-Length framing and JSON lines."""
        while True:
            line = stream.readline()
            if not line:
                return None, False

            stripped = line.strip()
            if not stripped:
                continue

            if stripped.lower().startswith(b"content-length:"):
                try:
                    length = int(stripped.split(b":", 1)[1].strip())
                except ValueError as exc:
                    raise ValueError(f"Invalid Content-Length header: {stripped!r}") from exc

                # Consume remaining headers until the blank separator line.
                while True:
                    header_line = stream.readline()
                    if not header_line or header_line in (b"\r\n", b"\n"):
                        break

                body = stream.read(length)
                return body, True

            return stripped, False

    @staticmethod
    def _write_message(
        stream: BinaryIO, response: dict[str, Any], framed: bool
    ) -> None:
        payload = json.dumps(response, separators=(",", ":")).encode("utf-8")
        if framed:
            header = f"Content-Length: {len(payload)}\r\n\r\n".encode("ascii")
            stream.write(header + payload)
        else:
            stream.write(payload + b"\n")
        stream.flush()

    def serve(
        self,
        stdin: BinaryIO | None = None,
        stdout: BinaryIO | None = None,
    ) -> int:
        """Run the stdio JSON-RPC loop until EOF."""
        in_stream = stdin if stdin is not None else sys.stdin.buffer
        out_stream = stdout if stdout is not None else sys.stdout.buffer

        while True:
            try:
                raw_bytes, framed = self._read_next_message(in_stream)
            except ValueError as exc:
                err_resp = {
                    "jsonrpc": "2.0",
                    "id": None,
                    "error": {
                        "code": -32700,
                        "message": f"Parse error: {exc}",
                    },
                }
                self._write_message(out_stream, err_resp, framed=True)
                continue

            if raw_bytes is None:
                break

            try:
                decoded = json.loads(raw_bytes.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                err_resp = {
                    "jsonrpc": "2.0",
                    "id": None,
                    "error": {
                        "code": -32700,
                        "message": f"Parse error: {exc}",
                    },
                }
                self._write_message(out_stream, err_resp, framed=framed)
                continue

            response = self.handle_message(decoded)
            if response is not None:
                self._write_message(out_stream, response, framed=framed)

        return 0


def main(argv: list[str] | None = None) -> int:
    """CLI entry point for fastaguard-mcp."""
    args = list(sys.argv[1:] if argv is None else argv)
    binary: str | None = None

    idx = 0
    while idx < len(args):
        arg = args[idx]
        if arg in ("-h", "--help"):
            print(
                "usage: fastaguard-mcp [--fastaguard-bin PATH] [--version]\n\n"
                "Run the FastaGuard Model Context Protocol (MCP) server over stdio.",
                file=sys.stdout,
            )
            return 0
        if arg in ("-V", "--version"):
            print(f"{SERVER_NAME} {SERVER_VERSION}", file=sys.stdout)
            return 0
        if arg == "--fastaguard-bin":
            if idx + 1 >= len(args):
                print(
                    "fastaguard-mcp error: --fastaguard-bin requires a path",
                    file=sys.stderr,
                )
                return 2
            binary = args[idx + 1]
            idx += 2
            continue
        if arg.startswith("--fastaguard-bin="):
            binary = arg.split("=", 1)[1]
            idx += 1
            continue
        print(f"fastaguard-mcp error: unknown argument '{arg}'", file=sys.stderr)
        return 2

    server = FastaguardMcpServer(binary=binary)
    return server.serve()


if __name__ == "__main__":
    raise SystemExit(main())
