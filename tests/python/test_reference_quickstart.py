import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "examples/reference/quickstart.sh"
HELPER = ROOT / "examples/workflows/check_fastaguard_gate.py"
FIXTURES = ROOT / "examples/reference/fixtures"


class ReferenceQuickstartTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        supplied = os.environ.get("FASTAGUARD_BINARY")
        if supplied:
            cls.binary = Path(supplied).resolve(strict=True)
        else:
            subprocess.run(
                ["cargo", "build", "--locked", "--quiet", "--bin", "fastaguard"],
                cwd=ROOT,
                check=True,
            )
            cls.binary = ROOT / "target/debug/fastaguard"
        cls.schema = json.loads(
            subprocess.check_output([str(cls.binary), "--schema"], text=True)
        )

    def run_example(self, output, binary=None):
        return subprocess.run(
            ["bash", str(RUNNER), str(binary or self.binary), str(output)],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )

    def test_example_preserves_pass_and_blocked_reports(self):
        with TemporaryDirectory(prefix="reference quickstart ") as tmp:
            root = Path(tmp)
            executable = root / "tool with spaces"
            executable.symlink_to(self.binary)
            output = root / "reports with spaces"
            result = self.run_example(output, executable)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            for case, status, allowed, gate_code in (
                ("pass", "PASS", True, 0),
                ("mismatch", "FAIL", False, 2),
            ):
                with self.subTest(case=case):
                    directory = output / case
                    paths = [
                        directory / name
                        for name in (
                            "reference.fastaguard.html",
                            "reference.fastaguard.json",
                            "reference.fastaguard.tsv",
                            "reference.fastaguard_mqc.json",
                        )
                    ]
                    before = {path: path.read_bytes() for path in paths}
                    self.assertTrue(all(before.values()))
                    report = json.loads(paths[1].read_text())
                    jsonschema.validate(report, self.schema)
                    self.assertEqual(report["report_type"], "reference")
                    self.assertEqual(report["schema_version"], "1.0.0")
                    self.assertEqual(report["verdict"]["status"], status)
                    self.assertIs(report["gate"]["can_continue"], allowed)
                    gate = subprocess.run(
                        [sys.executable, str(HELPER), str(paths[1])],
                        text=True,
                        capture_output=True,
                    )
                    self.assertEqual(gate.returncode, gate_code, gate.stderr)
                    self.assertEqual(
                        before, {path: path.read_bytes() for path in paths}
                    )
            lock = json.loads(
                (output / "pass/reference.fastaguard.lock.json").read_text()
            )
            report = json.loads(
                (output / "pass/reference.fastaguard.json").read_text()
            )
            self.assertEqual(
                lock["semantic_digest"],
                report["reference_manifest"]["semantic_digest"],
            )
            self.assertFalse(
                (output / "mismatch/reference.fastaguard.lock.json").exists()
            )

    def test_rerun_does_not_overwrite_existing_reports(self):
        with TemporaryDirectory() as tmp:
            output = Path(tmp) / "reports"
            first = self.run_example(output)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            before = {
                path.relative_to(output): path.read_bytes()
                for path in output.rglob("*")
                if path.is_file()
            }
            second = self.run_example(output)
            self.assertEqual(second.returncode, 3, second.stdout + second.stderr)
            after = {
                path.relative_to(output): path.read_bytes()
                for path in output.rglob("*")
                if path.is_file()
            }
            self.assertEqual(before, after)

    def test_runtime_failure_is_not_treated_as_expected_gate_block(self):
        with TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = root / "failing tool"
            binary.write_text("#!/bin/sh\nexit 3\n")
            binary.chmod(0o755)
            output = root / "reports"
            result = self.run_example(output, binary)
            self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
            self.assertFalse(any(output.rglob("*.fastaguard.json")))

    def test_explicit_lock_path_uses_the_boolean_write_flag(self):
        with TemporaryDirectory(prefix="reference lock ") as tmp:
            root = Path(tmp)
            report_path = root / "report.json"
            lock_path = root / "baseline.json"
            result = subprocess.run(
                [
                    str(self.binary), "reference", str(FIXTURES / "reference.fa"),
                    "--dict", str(FIXTURES / "reference.dict"), "--require", "dict",
                    "--format", "json", "--json", str(report_path),
                    "--write-lock", "--lock", str(lock_path),
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(report_path.read_text())
            lock = json.loads(lock_path.read_text())
            self.assertEqual(
                lock["semantic_digest"],
                report["reference_manifest"]["semantic_digest"],
            )


if __name__ == "__main__":
    unittest.main()
