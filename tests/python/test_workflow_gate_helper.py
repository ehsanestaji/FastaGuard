import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "examples" / "workflows" / "check_fastaguard_gate.py"
MISSING = object()


class WorkflowGateHelperTest(unittest.TestCase):
    def write_report(self, directory, status, can_continue=MISSING):
        path = Path(directory) / f"{status.lower()}.json"
        gate = {
            "mode": "pipeline",
            "status": status,
            "blocking_findings": [] if can_continue is True else ["example_blocker"],
        }
        if can_continue is not MISSING:
            gate["can_continue"] = can_continue
        path.write_text(json.dumps({"verdict": {"status": status}, "gate": gate}))
        return path

    def run_helper(self, path):
        return subprocess.run(
            [sys.executable, str(HELPER), str(path)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_pass_with_continuation_exits_zero(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "PASS", True))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            result.stdout.strip(),
            "FastaGuard report: verdict=PASS gate.status=PASS "
            "gate.can_continue=true gate.mode=pipeline "
            "gate.blocking_findings=[]",
        )

    def test_warn_with_continuation_exits_zero(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "WARN", True))
        self.assertEqual(result.returncode, 0)
        self.assertIn("verdict=WARN", result.stdout)
        self.assertIn("gate.can_continue=true", result.stdout)

    def test_warn_without_continuation_exits_two(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "WARN", False))
        self.assertEqual(result.returncode, 2)
        self.assertIn("verdict=WARN", result.stdout)
        self.assertIn("gate.can_continue=false", result.stdout)

    def test_fail_without_continuation_exits_two(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "FAIL", False))
        self.assertEqual(result.returncode, 2)
        self.assertIn("verdict=FAIL", result.stdout)
        self.assertIn("gate.can_continue=false", result.stdout)

    def test_legacy_report_without_can_continue_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "WARN"))
        self.assertEqual(result.returncode, 3)
        self.assertIn("gate.can_continue", result.stderr)

    def test_non_boolean_can_continue_exits_three(self):
        for value in (None, 0, 1, "true", [], {}):
            with self.subTest(value=value), TemporaryDirectory() as temp_dir:
                result = self.run_helper(self.write_report(temp_dir, "WARN", value))
            self.assertEqual(result.returncode, 3)
            self.assertIn("gate.can_continue", result.stderr)

    def test_missing_gate_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "bad.json"
            path.write_text("{}")
            result = self.run_helper(path)
        self.assertEqual(result.returncode, 3)
        self.assertIn("gate.can_continue", result.stderr)


if __name__ == "__main__":
    unittest.main()
