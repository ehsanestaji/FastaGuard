import email
import io
import json
import os
import subprocess
import sys
import tarfile
import tomllib
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
MCP_ROOT = ROOT / "integrations" / "mcp"
sys.path.insert(0, str(MCP_ROOT / "src"))

from fastaguard_mcp import (  # noqa: E402
    EXIT_CODE_MEANINGS,
    SERVER_NAME,
    SERVER_VERSION,
    TOOL_DEFINITIONS,
    FastaguardMcpServer,
    main as mcp_main,
    resolve_fastaguard_binary,
)


class McpServerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        supplied = os.environ.get("FASTAGUARD_BIN") or os.environ.get(
            "FASTAGUARD_BINARY"
        )
        if supplied:
            cls.binary = Path(supplied).resolve(strict=True)
        else:
            subprocess.run(
                ["cargo", "build", "--locked", "--quiet", "--bin", "fastaguard"],
                cwd=ROOT,
                check=True,
            )
            cls.binary = ROOT / "target" / "debug" / "fastaguard"

        cls.server = FastaguardMcpServer(binary=cls.binary, cwd=ROOT)
        cls.schema = json.loads(
            subprocess.check_output([str(cls.binary), "--schema"], text=True)
        )

    def test_pyproject_declares_zero_dependency_mcp_package(self):
        pyproject_path = MCP_ROOT / "pyproject.toml"
        self.assertTrue(pyproject_path.is_file())
        data = tomllib.loads(pyproject_path.read_text(encoding="utf-8"))

        project = data["project"]
        self.assertEqual(project["name"], "fastaguard-mcp")
        self.assertEqual(project["requires-python"], ">=3.10")
        self.assertEqual(project["dependencies"], [])
        self.assertEqual(
            project["scripts"]["fastaguard-mcp"],
            "fastaguard_mcp.server:main",
        )
        self.assertTrue((MCP_ROOT / "README.md").is_file())

    def test_llm_vision_doc_references_mcp_integration(self):
        vision_text = (ROOT / "docs" / "llm-tooling-vision.md").read_text(
            encoding="utf-8"
        )
        self.assertIn("integrations/mcp/", vision_text)
        self.assertIn("fastaguard-mcp", vision_text)
        self.assertIn("cannot_conclude", vision_text)

    def test_binary_resolution_order(self):
        explicit = resolve_fastaguard_binary("/custom/path/fastaguard")
        self.assertEqual(explicit, "/custom/path/fastaguard")

        old_bin = os.environ.get("FASTAGUARD_BIN")
        try:
            os.environ["FASTAGUARD_BIN"] = "/env/path/fastaguard"
            self.assertEqual(resolve_fastaguard_binary(), "/env/path/fastaguard")
        finally:
            if old_bin is None:
                os.environ.pop("FASTAGUARD_BIN", None)
            else:
                os.environ["FASTAGUARD_BIN"] = old_bin

    def test_initialize_ping_and_notifications(self):
        init_resp = self.server.handle_message(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test-client", "version": "1.0.0"},
                },
            }
        )
        self.assertIsNotNone(init_resp)
        self.assertEqual(init_resp["id"], 1)
        self.assertEqual(init_resp["result"]["protocolVersion"], "2024-11-05")
        self.assertEqual(
            init_resp["result"]["serverInfo"],
            {"name": SERVER_NAME, "version": SERVER_VERSION},
        )
        self.assertIn("tools", init_resp["result"]["capabilities"])

        notif_resp = self.server.handle_message(
            {"jsonrpc": "2.0", "method": "notifications/initialized"}
        )
        self.assertIsNone(notif_resp)

        ping_resp = self.server.handle_message(
            {"jsonrpc": "2.0", "id": 2, "method": "ping"}
        )
        self.assertEqual(ping_resp, {"jsonrpc": "2.0", "id": 2, "result": {}})

    def test_tools_list_exposes_all_six_contract_tools(self):
        resp = self.server.handle_message(
            {"jsonrpc": "2.0", "id": 3, "method": "tools/list"}
        )
        tools = resp["result"]["tools"]
        names = [tool["name"] for tool in tools]
        self.assertEqual(
            names,
            [
                "fastaguard_schema",
                "fastaguard_finding_catalog",
                "fastaguard_explain_finding",
                "fastaguard_inspect_fasta",
                "fastaguard_compare",
                "fastaguard_reference",
            ],
        )
        self.assertEqual(len(TOOL_DEFINITIONS), 6)
        for tool in tools:
            self.assertEqual(tool["inputSchema"]["type"], "object")
            self.assertIs(tool["inputSchema"]["additionalProperties"], False)

    def test_schema_and_finding_catalog_tools(self):
        schema_resp = self.server.handle_message(
            {
                "jsonrpc": "2.0",
                "id": 10,
                "method": "tools/call",
                "params": {"name": "fastaguard_schema", "arguments": {}},
            }
        )
        schema_res = schema_resp["result"]
        self.assertIs(schema_res["isError"], False)
        self.assertEqual(schema_res["exit_code"], 0)
        self.assertEqual(schema_res["structuredContent"]["title"], "FastaguardReport")
        self.assertEqual(
            json.loads(schema_res["content"][0]["text"])["title"], "FastaguardReport"
        )

        catalog_resp = self.server.handle_message(
            {
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/call",
                "params": {"name": "fastaguard_finding_catalog", "arguments": {}},
            }
        )
        catalog_res = catalog_resp["result"]
        self.assertIs(catalog_res["isError"], False)
        self.assertEqual(catalog_res["exit_code"], 0)
        finding_ids = {
            entry["id"] for entry in catalog_res["structuredContent"]["findings"]
        }
        self.assertIn("high_n_rate", finding_ids)
        self.assertIn("reference_length_mismatch", finding_ids)

    def test_explain_finding_known_unknown_and_missing(self):
        known = self.server.call_tool(
            "fastaguard_explain_finding", {"id": "high_n_rate"}
        )
        self.assertIs(known["isError"], False)
        self.assertEqual(known["exit_code"], 0)
        self.assertEqual(known["structuredContent"]["id"], "high_n_rate")
        self.assertEqual(known["structuredContent"]["category"], "composition")

        unknown = self.server.call_tool(
            "fastaguard_explain_finding", {"id": "nonexistent_finding_id"}
        )
        self.assertIs(unknown["isError"], True)
        self.assertEqual(unknown["exit_code"], 3)
        self.assertEqual(
            unknown["structuredContent"]["exit_meaning"],
            EXIT_CODE_MEANINGS[3],
        )
        self.assertIn("unknown finding id", unknown["structuredContent"]["error"])

        missing = self.server.call_tool("fastaguard_explain_finding", {})
        self.assertIs(missing["isError"], True)
        self.assertEqual(missing["exit_code"], 2)
        self.assertEqual(
            missing["structuredContent"]["exit_meaning"],
            EXIT_CODE_MEANINGS[2],
        )

    def test_inspect_fasta_pass_fail_ncbi_and_bundle_persistence(self):
        clean_fa = ROOT / "testdata" / "clean_corpus" / "clean_bacterial.fa"
        clean_before = clean_fa.read_bytes()

        pass_res = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {
                "input": str(clean_fa),
                "profile": "assembly",
                "gate": "pipeline",
            },
        )
        self.assertIs(pass_res["isError"], False)
        self.assertEqual(pass_res["exit_code"], 0)
        pass_report = pass_res["structuredContent"]
        jsonschema.validate(pass_report, self.schema)
        self.assertEqual(pass_report["verdict"]["status"], "PASS")
        self.assertIs(pass_report["gate"]["can_continue"], True)
        self.assertIn("readiness", pass_report)
        self.assertIn("machine_summary", pass_report)
        self.assertIn("scope", pass_report)
        self.assertEqual(clean_fa.read_bytes(), clean_before)

        problem_fa = ROOT / "testdata" / "problem_assembly.fa"
        problem_before = problem_fa.read_bytes()
        fail_res = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {
                "input": str(problem_fa),
                "gate": "pipeline",
                "fail_on": ["tiny_contigs"],
            },
        )
        self.assertIs(fail_res["isError"], False)
        self.assertEqual(fail_res["exit_code"], 0)
        fail_report = fail_res["structuredContent"]
        jsonschema.validate(fail_report, self.schema)
        self.assertEqual(fail_report["verdict"]["status"], "FAIL")
        self.assertIs(fail_report["gate"]["can_continue"], False)
        self.assertTrue(len(fail_report["findings"]) > 0)
        self.assertEqual(problem_fa.read_bytes(), problem_before)

        ncbi_fa = ROOT / "testdata" / "ncbi_genome" / "seqid_50.fa"
        ncbi_res = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {
                "input": str(ncbi_fa),
                "gate": "submission",
                "submission_target": "ncbi",
            },
        )
        self.assertIs(ncbi_res["isError"], False)
        self.assertEqual(ncbi_res["exit_code"], 0)
        ncbi_report = ncbi_res["structuredContent"]
        jsonschema.validate(ncbi_report, self.schema)
        self.assertEqual(
            ncbi_report["gate"]["submission_policy"]["id"], "ncbi_genome"
        )
        self.assertIs(ncbi_report["gate"]["can_continue"], False)
        self.assertIn(
            "ncbi_genome_seqid", ncbi_report["gate"]["blocking_findings"]
        )

        with TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "bundle"
            bundle_res = self.server.call_tool(
                "fastaguard_inspect_fasta",
                {
                    "input": str(clean_fa),
                    "outdir": str(outdir),
                    "prefix": "sample01",
                },
            )
            self.assertIs(bundle_res["isError"], False)
            self.assertEqual(bundle_res["exit_code"], 0)
            self.assertTrue((outdir / "sample01.fastaguard.json").is_file())
            self.assertTrue((outdir / "sample01.fastaguard.html").is_file())
            self.assertTrue((outdir / "sample01.fastaguard.tsv").is_file())
            self.assertTrue((outdir / "sample01.fastaguard_mqc.json").is_file())

            # Re-running without force must surface exit code 3 (no-clobber).
            clobber_res = self.server.call_tool(
                "fastaguard_inspect_fasta",
                {
                    "input": str(clean_fa),
                    "outdir": str(outdir),
                    "prefix": "sample01",
                },
            )
            self.assertIs(clobber_res["isError"], True)
            self.assertEqual(clobber_res["exit_code"], 3)

    def test_compare_tool_cohort_and_error_handling(self):
        fa1 = ROOT / "testdata" / "clean_corpus" / "clean_bacterial.fa"
        fa2 = ROOT / "testdata" / "problem_assembly.fa"
        before1 = fa1.read_bytes()
        before2 = fa2.read_bytes()

        res = self.server.call_tool(
            "fastaguard_compare",
            {
                "inputs": [str(fa1), str(fa2)],
                "gate": "pipeline",
            },
        )
        self.assertIs(res["isError"], False)
        self.assertEqual(res["exit_code"], 0)
        report = res["structuredContent"]
        jsonschema.validate(report, self.schema)
        self.assertEqual(report["report_type"], "compare")
        self.assertEqual(report["summary"]["sample_count"], 2)
        self.assertEqual(fa1.read_bytes(), before1)
        self.assertEqual(fa2.read_bytes(), before2)

        single_input_err = self.server.call_tool(
            "fastaguard_compare",
            {"inputs": [str(fa1)]},
        )
        self.assertIs(single_input_err["isError"], True)
        self.assertEqual(single_input_err["exit_code"], 3)

    def test_reference_tool_pass_mismatch_and_lock(self):
        fixtures = ROOT / "examples" / "reference" / "fixtures"
        ref_fa = fixtures / "reference.fa"
        ref_before = ref_fa.read_bytes()

        pass_res = self.server.call_tool(
            "fastaguard_reference",
            {
                "reference": str(ref_fa),
                "fai": str(fixtures / "reference.fa.fai"),
                "dict": str(fixtures / "reference.dict"),
                "require": ["fai", "dict"],
                "policy": "coordinate",
            },
        )
        self.assertIs(pass_res["isError"], False)
        self.assertEqual(pass_res["exit_code"], 0)
        pass_report = pass_res["structuredContent"]
        jsonschema.validate(pass_report, self.schema)
        self.assertEqual(pass_report["report_type"], "reference")
        self.assertEqual(pass_report["schema_version"], "1.0.0")
        self.assertEqual(pass_report["verdict"]["status"], "PASS")
        self.assertIs(pass_report["gate"]["can_continue"], True)
        self.assertEqual(ref_fa.read_bytes(), ref_before)

        repeated_res = self.server.call_tool(
            "fastaguard_reference",
            {
                "reference": str(ref_fa),
                "fai": str(fixtures / "aliased.fa.fai"),
                "dict": str(fixtures / "reference.dict"),
                "alias_map": str(fixtures / "aliases.tsv"),
                "alignment": [str(fixtures / "reads.sam")],
                "variants": [
                    str(fixtures / "variants" / "a" / "sites.vcf"),
                    str(fixtures / "variants" / "b" / "sites.vcf"),
                ],
                "annotation": [str(fixtures / "genes.gff3")],
                "require": ["fai", "dict", "alignment", "variants", "annotation"],
                "policy": "coordinate",
            },
        )
        self.assertIs(repeated_res["isError"], False)
        self.assertEqual(repeated_res["exit_code"], 0)
        repeated_report = repeated_res["structuredContent"]
        jsonschema.validate(repeated_report, self.schema)
        self.assertEqual(repeated_report["verdict"]["status"], "WARN")
        self.assertIs(repeated_report["gate"]["can_continue"], True)

        mismatch_res = self.server.call_tool(
            "fastaguard_reference",
            {
                "reference": str(ref_fa),
                "dict": str(fixtures / "copied-length-mismatch.dict"),
                "require": "dict",
                "policy": "coordinate",
            },
        )
        self.assertIs(mismatch_res["isError"], False)
        self.assertEqual(mismatch_res["exit_code"], 0)
        mismatch_report = mismatch_res["structuredContent"]
        jsonschema.validate(mismatch_report, self.schema)
        self.assertEqual(mismatch_report["verdict"]["status"], "FAIL")
        self.assertIs(mismatch_report["gate"]["can_continue"], False)
        finding_ids = [f["id"] for f in mismatch_report["findings"]]
        self.assertIn("reference_declaration_mismatch", finding_ids)

        with TemporaryDirectory() as tmp:
            outdir = Path(tmp) / "ref_bundle"
            lock_res = self.server.call_tool(
                "fastaguard_reference",
                {
                    "reference": str(ref_fa),
                    "dict": str(fixtures / "reference.dict"),
                    "require": ["dict"],
                    "outdir": str(outdir),
                    "prefix": "grch38",
                    "write_lock": True,
                },
            )
            self.assertIs(lock_res["isError"], False)
            self.assertEqual(lock_res["exit_code"], 0)
            lock_file = outdir / "grch38.fastaguard.lock.json"
            self.assertTrue(lock_file.is_file())
            lock_data = json.loads(lock_file.read_text(encoding="utf-8"))
            self.assertEqual(
                lock_data["semantic_digest"],
                lock_res["structuredContent"]["reference_manifest"]["semantic_digest"],
            )

    def test_exit_codes_2_and_3_and_input_protection(self):
        clean_fa = ROOT / "testdata" / "clean_corpus" / "clean_bacterial.fa"

        # Invalid enum triggers clap exit code 2.
        bad_gate = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {"input": str(clean_fa), "gate": "not_a_gate"},
        )
        self.assertIs(bad_gate["isError"], True)
        self.assertEqual(bad_gate["exit_code"], 2)

        # Prefix without outdir triggers clap exit code 2.
        prefix_without_outdir = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {"input": str(clean_fa), "prefix": "stem_only"},
        )
        self.assertIs(prefix_without_outdir["isError"], True)
        self.assertEqual(prefix_without_outdir["exit_code"], 2)

        # Unknown tool argument triggers exit code 2.
        unknown_arg = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {"input": str(clean_fa), "unexpected_flag": True},
        )
        self.assertIs(unknown_arg["isError"], True)
        self.assertEqual(unknown_arg["exit_code"], 2)

        # Missing input file triggers runtime exit code 3.
        missing_file = self.server.call_tool(
            "fastaguard_inspect_fasta",
            {"input": str(ROOT / "testdata" / "does_not_exist.fa")},
        )
        self.assertIs(missing_file["isError"], True)
        self.assertEqual(missing_file["exit_code"], 3)

        # Output collision with input path is refused with exit code 3 without touching input.
        with TemporaryDirectory() as tmp:
            tmp_dir = Path(tmp)
            tricky_input = tmp_dir / "sample.fastaguard.json"
            tricky_input.write_text(">seq1\nACGT\n", encoding="utf-8")
            original_bytes = tricky_input.read_bytes()
            collision_res = self.server.call_tool(
                "fastaguard_inspect_fasta",
                {
                    "input": str(tricky_input),
                    "outdir": str(tmp_dir),
                    "prefix": "sample",
                    "force": True,
                },
            )
            self.assertIs(collision_res["isError"], True)
            self.assertEqual(collision_res["exit_code"], 3)
            self.assertEqual(tricky_input.read_bytes(), original_bytes)

    def test_stdio_serve_newline_and_content_length_framing(self):
        # 1. Newline-delimited JSON-RPC stream
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "ping"},
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "fastaguard_explain_finding",
                    "arguments": {"id": "high_n_rate"},
                },
            },
        ]
        stdin_nl = io.BytesIO(
            b"".join(json.dumps(req).encode("utf-8") + b"\n" for req in requests)
        )
        stdout_nl = io.BytesIO()
        self.assertEqual(self.server.serve(stdin=stdin_nl, stdout=stdout_nl), 0)

        lines = [
            json.loads(line.decode("utf-8"))
            for line in stdout_nl.getvalue().splitlines()
            if line.strip()
        ]
        self.assertEqual(len(lines), 3)
        self.assertEqual(lines[0]["id"], 1)
        self.assertEqual(lines[1]["id"], 2)
        self.assertEqual(lines[2]["id"], 3)
        self.assertEqual(
            lines[2]["result"]["structuredContent"]["id"], "high_n_rate"
        )

        # 2. Content-Length framed JSON-RPC stream
        framed_req = json.dumps(
            {"jsonrpc": "2.0", "id": 42, "method": "tools/list"}
        ).encode("utf-8")
        framed_input = (
            f"Content-Length: {len(framed_req)}\r\n\r\n".encode("ascii")
            + framed_req
        )
        stdin_cl = io.BytesIO(framed_input)
        stdout_cl = io.BytesIO()
        self.assertEqual(self.server.serve(stdin=stdin_cl, stdout=stdout_cl), 0)

        raw_out = stdout_cl.getvalue()
        self.assertTrue(raw_out.startswith(b"Content-Length: "))
        header_part, body_part = raw_out.split(b"\r\n\r\n", 1)
        declared_len = int(header_part.split(b":", 1)[1].strip())
        self.assertEqual(declared_len, len(body_part))
        parsed_cl = json.loads(body_part.decode("utf-8"))
        self.assertEqual(parsed_cl["id"], 42)
        self.assertEqual(len(parsed_cl["result"]["tools"]), 6)

    def test_cli_entrypoint_and_subprocess_stdio(self):
        self.assertEqual(mcp_main(["--version"]), 0)
        self.assertEqual(mcp_main(["--help"]), 0)
        self.assertEqual(mcp_main(["--unknown-flag"]), 2)
        self.assertEqual(mcp_main(["--fastaguard-bin"]), 2)

        env = os.environ.copy()
        env["PYTHONPATH"] = str(MCP_ROOT / "src")
        env["FASTAGUARD_BIN"] = str(self.binary)
        req_line = (
            json.dumps({"jsonrpc": "2.0", "id": 99, "method": "ping"}) + "\n"
        )
        proc = subprocess.run(
            [sys.executable, "-m", "fastaguard_mcp"],
            input=req_line,
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=True,
        )
        resp = json.loads(proc.stdout.strip())
        self.assertEqual(resp, {"jsonrpc": "2.0", "id": 99, "result": {}})

    def test_jsonrpc_error_codes_and_missing_binary(self):
        invalid_req = self.server.handle_message({"jsonrpc": "1.0", "id": 1})
        self.assertEqual(invalid_req["error"]["code"], -32600)

        missing_method = self.server.handle_message({"jsonrpc": "2.0", "id": 2})
        self.assertEqual(missing_method["error"]["code"], -32600)

        bad_params = self.server.handle_message(
            {"jsonrpc": "2.0", "id": 3, "method": "ping", "params": "not-a-dict"}
        )
        self.assertEqual(bad_params["error"]["code"], -32602)

        unknown_method = self.server.handle_message(
            {"jsonrpc": "2.0", "id": 4, "method": "resources/list"}
        )
        self.assertEqual(unknown_method["error"]["code"], -32601)

        unknown_tool = self.server.handle_message(
            {
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {"name": "not_a_fastaguard_tool", "arguments": {}},
            }
        )
        self.assertEqual(unknown_tool["error"]["code"], -32602)

        stdin_bad = io.BytesIO(b"{invalid json\n")
        stdout_bad = io.BytesIO()
        self.assertEqual(self.server.serve(stdin=stdin_bad, stdout=stdout_bad), 0)
        parse_err = json.loads(stdout_bad.getvalue().decode("utf-8").strip())
        self.assertEqual(parse_err["error"]["code"], -32700)

        broken_server = FastaguardMcpServer(
            binary=ROOT / "target" / "does_not_exist_fastaguard", cwd=ROOT
        )
        missing_bin_res = broken_server.call_tool("fastaguard_schema", {})
        self.assertIs(missing_bin_res["isError"], True)
        self.assertEqual(missing_bin_res["exit_code"], 3)

    def test_wheel_and_sdist_build_metadata(self):
        with TemporaryDirectory(prefix="fastaguard-mcp-build-") as tmp:
            dist_dir = Path(tmp) / "dist"
            env = os.environ.copy()
            env.pop("PYTHONHOME", None)
            env.pop("PYTHONPATH", None)
            env["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
            result = subprocess.run(
                [
                    sys.executable,
                    "-m",
                    "build",
                    "--no-isolation",
                    "--outdir",
                    str(dist_dir),
                    str(MCP_ROOT),
                ],
                cwd=ROOT,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            if result.returncode != 0:
                # Fallback to standard build if hatchling is not in global site-packages
                result = subprocess.run(
                    [
                        sys.executable,
                        "-m",
                        "build",
                        "--outdir",
                        str(dist_dir),
                        str(MCP_ROOT),
                    ],
                    cwd=ROOT,
                    env=env,
                    capture_output=True,
                    text=True,
                    check=True,
                )

            wheel = next(dist_dir.glob("*.whl"))
            sdist = next(dist_dir.glob("*.tar.gz"))

            with zipfile.ZipFile(wheel) as archive:
                names = set(archive.namelist())
                metadata_name = next(
                    n for n in names if n.endswith(".dist-info/METADATA")
                )
                entry_points_name = next(
                    n for n in names if n.endswith(".dist-info/entry_points.txt")
                )
                metadata = email.message_from_bytes(archive.read(metadata_name))
                entry_points = archive.read(entry_points_name).decode("utf-8")

            self.assertEqual(metadata["Name"], "fastaguard-mcp")
            self.assertEqual(metadata["Version"], "0.1.0")
            self.assertEqual(metadata["Requires-Python"], ">=3.10")
            self.assertIsNone(metadata.get_all("Requires-Dist"))
            self.assertIn("[console_scripts]", entry_points)
            self.assertIn(
                "fastaguard-mcp = fastaguard_mcp.server:main", entry_points
            )
            self.assertIn("fastaguard_mcp/server.py", names)

            with tarfile.open(sdist, "r:gz") as archive:
                sdist_names = set(archive.getnames())
            self.assertIn("fastaguard_mcp-0.1.0/README.md", sdist_names)
            self.assertIn(
                "fastaguard_mcp-0.1.0/src/fastaguard_mcp/server.py", sdist_names
            )


if __name__ == "__main__":
    unittest.main()
