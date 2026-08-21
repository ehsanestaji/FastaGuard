import copy
import json
import stat
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "verify_ncbi_genome_policy.py"
MANIFEST = ROOT / "docs" / "evidence" / "ncbi-genome-policy.json"
SOURCE_MANIFEST = ROOT / "testdata" / "ncbi_genome" / "policy_cases.json"


class NcbiGenomePolicyVerifierTest(unittest.TestCase):
    def load_manifest(self):
        return json.loads(MANIFEST.read_text())

    def run_verifier(
        self,
        output,
        *,
        manifest=MANIFEST,
        fastaguard=None,
        table2asn=None,
        required=False,
        timeout=None,
    ):
        command = [
            sys.executable,
            str(SCRIPT),
            "--fastaguard",
            str(fastaguard or ROOT / "target" / "debug" / "fastaguard"),
            "--manifest",
            str(manifest),
            "--out",
            str(output),
        ]
        if table2asn is not None:
            command.extend(["--table2asn", str(table2asn)])
        if required:
            command.append("--require-table2asn")
        if timeout is not None:
            command.extend(["--timeout", str(timeout)])
        return subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def write_manifest(self, directory, manifest):
        path = Path(directory) / "manifest.json"
        path.write_text(json.dumps(manifest))
        return path

    def make_executable(self, path, source):
        path.write_text(source)
        path.chmod(path.stat().st_mode | stat.S_IXUSR)

    def make_fake_tools(self, directory, *, slow_table2asn=False):
        tool_dir = Path(directory) / "tools with spaces;literal"
        tool_dir.mkdir()
        fastaguard = tool_dir / "fastaguard fake"
        table2asn = tool_dir / "table2asn fake"
        findings_by_fixture = {
            case["fixture"]: case["expect_findings"]
            for case in json.loads(SOURCE_MANIFEST.read_text())
        }
        continuation_by_fixture = {
            case["fixture"]: case["expect_can_continue"]
            for case in json.loads(SOURCE_MANIFEST.read_text())
        }
        self.make_executable(
            fastaguard,
            "#!/usr/bin/env python3\n"
            "import json, pathlib, sys\n"
            f"findings = {findings_by_fixture!r}\n"
            f"continuation = {continuation_by_fixture!r}\n"
            "fixture = pathlib.Path(sys.argv[1]).name\n"
            "out = pathlib.Path(sys.argv[sys.argv.index('--json') + 1])\n"
            "out.write_text(json.dumps({\n"
            "    'findings': [{'id': item} for item in findings[fixture]],\n"
            "    'gate': {'can_continue': continuation[fixture]},\n"
            "}))\n",
        )
        table_source = (
            "#!/usr/bin/env python3\n"
            "import pathlib, sys, time\n"
            "if '-help' in sys.argv:\n"
            "    print('table2asn fake 99.1')\n"
            "    raise SystemExit(0)\n"
        )
        if slow_table2asn:
            table_source += "time.sleep(0.3)\nraise SystemExit(0)\n"
        else:
            rejected = sorted(
                case["fixture"]
                for case in self.load_manifest()["cases"]
                if case["expected_table2asn_result"] == "rejected"
            )
            table_source += (
                f"rejected = {rejected!r}\n"
                "fixture = pathlib.Path(sys.argv[sys.argv.index('-i') + 1]).name\n"
                "print('controlled temp path:', pathlib.Path.cwd(), file=sys.stderr)\n"
                "raise SystemExit(1 if fixture in rejected else 0)\n"
            )
        self.make_executable(table2asn, table_source)
        return fastaguard, table2asn

    def assert_output_is_normalized(self, payload, *forbidden_paths):
        serialized = json.dumps(payload, sort_keys=True)
        self.assertNotIn(str(ROOT), serialized)
        for path in forbidden_paths:
            self.assertNotIn(str(Path(path).resolve()), serialized)

    def test_manifest_rejects_duplicate_and_unsorted_cases(self):
        original = self.load_manifest()
        with TemporaryDirectory() as temp_dir:
            invalid_manifests = {
                "duplicate": {
                    **original,
                    "cases": original["cases"] + [copy.deepcopy(original["cases"][0])],
                },
                "unsorted": {
                    **original,
                    "cases": [original["cases"][1], original["cases"][0]]
                    + original["cases"][2:],
                },
            }
            for name, manifest in invalid_manifests.items():
                with self.subTest(name=name):
                    manifest_path = self.write_manifest(temp_dir, manifest)
                    result = self.run_verifier(
                        Path(temp_dir) / f"{name}.json",
                        manifest=manifest_path,
                        table2asn=Path(temp_dir) / "missing-table2asn",
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(name, result.stderr.lower())

    def test_manifest_rejects_case_without_required_key(self):
        manifest = self.load_manifest()
        del manifest["cases"][0]["expected_table2asn_result"]
        with TemporaryDirectory() as temp_dir:
            manifest_path = self.write_manifest(temp_dir, manifest)
            result = self.run_verifier(
                Path(temp_dir) / "result.json",
                manifest=manifest_path,
                table2asn=Path(temp_dir) / "missing-table2asn",
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expected_table2asn_result", result.stderr)

    def test_manifest_rejects_missing_top_level_key(self):
        manifest = self.load_manifest()
        del manifest["source_manifest"]
        with TemporaryDirectory() as temp_dir:
            manifest_path = self.write_manifest(temp_dir, manifest)
            result = self.run_verifier(
                Path(temp_dir) / "result.json",
                manifest=manifest_path,
                table2asn=Path(temp_dir) / "missing-table2asn",
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("source_manifest", result.stderr)

    def test_missing_optional_table2asn_writes_unavailable_result_without_comparison(self):
        with TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "result.json"
            missing_tool = Path(temp_dir) / "missing-table2asn"
            result = self.run_verifier(output, table2asn=missing_tool)

            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(output.read_text())
            self.assertIs(payload["table2asn_available"], False)
            self.assertIs(payload["comparison_performed"], False)
            self.assertIsNone(payload["table2asn_version"])
            self.assertEqual(payload["cases"], [])
            self.assertNotIn("comparison_summary", payload)
            self.assert_output_is_normalized(
                payload,
                temp_dir,
                ROOT / "testdata" / "ncbi_genome" / "contig_199.fa",
            )

    def test_missing_required_table2asn_is_a_verifier_error(self):
        with TemporaryDirectory() as temp_dir:
            output = Path(temp_dir) / "result.json"
            result = self.run_verifier(
                output,
                table2asn=Path(temp_dir) / "missing-table2asn",
                required=True,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("required table2asn", result.stderr.lower())
            payload = json.loads(output.read_text())
            self.assertIs(payload["table2asn_available"], False)
            self.assertIs(payload["comparison_performed"], False)
            self.assertEqual(payload["cases"], [])
            self.assert_output_is_normalized(payload, temp_dir)

    def test_available_tools_produce_categorical_normalized_case_results(self):
        with TemporaryDirectory() as temp_dir:
            fastaguard, table2asn = self.make_fake_tools(temp_dir)
            output = Path(temp_dir) / "result.json"
            result = self.run_verifier(
                output,
                fastaguard=fastaguard,
                table2asn=table2asn,
                required=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            payload = json.loads(output.read_text())
            self.assertIs(payload["table2asn_available"], True)
            self.assertIs(payload["comparison_performed"], True)
            self.assertEqual(payload["table2asn_version"], "table2asn fake 99.1")
            self.assertEqual(len(payload["cases"]), len(self.load_manifest()["cases"]))
            self.assertEqual(payload["comparison_summary"]["mismatched_cases"], 0)
            for case in payload["cases"]:
                with self.subTest(case=case["id"]):
                    self.assertIn(
                        case["table2asn_result"],
                        {"accepted", "rejected", "tool_error"},
                    )
                    self.assertEqual(
                        case["table2asn_result"],
                        case["expected_table2asn_result"],
                    )
                    self.assertEqual(
                        case["fastaguard_findings"],
                        sorted(case["fastaguard_findings"]),
                    )
                    self.assertEqual(case["commands"]["fastaguard"][0], "$FASTAGUARD")
                    self.assertEqual(case["commands"]["table2asn"][0], "$TABLE2ASN")
            self.assert_output_is_normalized(payload, temp_dir, fastaguard, table2asn)

    def test_table2asn_timeout_is_categorical_and_required_mode_fails(self):
        with TemporaryDirectory() as temp_dir:
            fastaguard, table2asn = self.make_fake_tools(
                temp_dir, slow_table2asn=True
            )
            optional_output = Path(temp_dir) / "optional.json"
            optional = self.run_verifier(
                optional_output,
                fastaguard=fastaguard,
                table2asn=table2asn,
                timeout=0.15,
            )
            required_output = Path(temp_dir) / "required.json"
            required = self.run_verifier(
                required_output,
                fastaguard=fastaguard,
                table2asn=table2asn,
                timeout=0.15,
                required=True,
            )

            self.assertEqual(optional.returncode, 0, optional.stderr)
            self.assertNotEqual(required.returncode, 0)
            optional_payload = json.loads(optional_output.read_text())
            required_payload = json.loads(required_output.read_text())
            self.assertTrue(
                all(
                    case["table2asn_result"] == "tool_error"
                    for case in optional_payload["cases"]
                )
            )
            self.assertTrue(
                all(
                    case["table2asn_result"] == "tool_error"
                    for case in required_payload["cases"]
                )
            )
            self.assert_output_is_normalized(optional_payload, temp_dir)
            self.assert_output_is_normalized(required_payload, temp_dir)


if __name__ == "__main__":
    unittest.main()
