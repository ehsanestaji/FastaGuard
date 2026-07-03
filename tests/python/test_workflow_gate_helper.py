import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "examples" / "workflows" / "check_fastaguard_gate.py"


class WorkflowGateHelperTest(unittest.TestCase):
    def write_report(self, directory, status):
        path = Path(directory) / f"{status.lower()}.json"
        path.write_text(json.dumps({"gate": {"status": status}}))
        return path

    def run_helper(self, path):
        return subprocess.run(
            [sys.executable, str(HELPER), str(path)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_pass_exits_zero(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "PASS"))
        self.assertEqual(result.returncode, 0)
        self.assertIn("PASS", result.stdout)

    def test_warn_exits_one(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "WARN"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("WARN", result.stdout)

    def test_fail_exits_two(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "FAIL"))
        self.assertEqual(result.returncode, 2)
        self.assertIn("FAIL", result.stdout)

    def test_malformed_report_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "bad.json"
            path.write_text("{}")
            result = self.run_helper(path)
        self.assertEqual(result.returncode, 3)
        self.assertIn("gate.status", result.stderr)


if __name__ == "__main__":
    unittest.main()
