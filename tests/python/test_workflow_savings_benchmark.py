import unittest
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from scripts.benchmark_workflow_savings import summarise_repeats


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "benchmark_workflow_savings.py"


class WorkflowSavingsBenchmarkTest(unittest.TestCase):
    def test_summarises_late_validation_work_avoided_by_preflight(self):
        gated = [
            {
                "preflight_wall_seconds": 0.4,
                "preflight_cpu_seconds": 0.2,
                "preflight_peak_rss_kib": 100,
                "downstream_tasks": [],
            },
            {
                "preflight_wall_seconds": 0.5,
                "preflight_cpu_seconds": 0.3,
                "preflight_peak_rss_kib": 110,
                "downstream_tasks": [],
            },
            {
                "preflight_wall_seconds": 0.6,
                "preflight_cpu_seconds": 0.4,
                "preflight_peak_rss_kib": 120,
                "downstream_tasks": [],
            },
        ]
        ungated = [
            {
                "preflight_wall_seconds": 0.0,
                "preflight_cpu_seconds": 0.0,
                "downstream_tasks": [
                    {
                        "name": "map_and_sort",
                        "wall_seconds": 120.0,
                        "cpu_seconds": 180.0,
                        "requested_cpus": 2,
                        "peak_rss_kib": 800,
                    }
                ],
            },
            {
                "preflight_wall_seconds": 0.0,
                "preflight_cpu_seconds": 0.0,
                "downstream_tasks": [
                    {
                        "name": "map_and_sort",
                        "wall_seconds": 150.0,
                        "cpu_seconds": 210.0,
                        "requested_cpus": 2,
                        "peak_rss_kib": 900,
                    }
                ],
            },
            {
                "preflight_wall_seconds": 0.0,
                "preflight_cpu_seconds": 0.0,
                "downstream_tasks": [
                    {
                        "name": "map_and_sort",
                        "wall_seconds": 180.0,
                        "cpu_seconds": 240.0,
                        "requested_cpus": 2,
                        "peak_rss_kib": 1000,
                    }
                ],
            },
        ]

        summary = summarise_repeats(gated, ungated)

        self.assertEqual(summary["repeat_count"], 3)
        self.assertEqual(summary["median_preflight_wall_seconds"], 0.5)
        self.assertEqual(summary["median_downstream_tasks_started_without_gate"], 1)
        self.assertEqual(summary["median_allocated_cpu_hours_avoided"], 0.0833)
        self.assertEqual(summary["median_actual_cpu_seconds_avoided"], 209.7)
        self.assertEqual(summary["median_wall_seconds_avoided"], 149.5)
        self.assertEqual(summary["median_preflight_peak_rss_kib"], 110)
        self.assertEqual(summary["median_downstream_peak_rss_kib_without_gate"], 900)
        self.assertIn("late validation", summary["interpretation"].lower())

    def test_cli_writes_portable_summary_from_matched_observations(self):
        gated = [{"preflight_wall_seconds": 1, "preflight_cpu_seconds": 0.5, "preflight_peak_rss_kib": 100, "downstream_tasks": []}]
        ungated = [{"preflight_wall_seconds": 0, "preflight_cpu_seconds": 0, "downstream_tasks": [{"name": "mapping", "wall_seconds": 30, "cpu_seconds": 20, "requested_cpus": 2, "peak_rss_kib": 400}]}]
        with tempfile.TemporaryDirectory() as directory:
            directory_path = Path(directory)
            gated_path = directory_path / "gated.json"
            ungated_path = directory_path / "ungated.json"
            output_path = directory_path / "summary.json"
            gated_path.write_text(json.dumps(gated), encoding="utf-8")
            ungated_path.write_text(json.dumps(ungated), encoding="utf-8")

            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "--gated", str(gated_path), "--ungated", str(ungated_path), "--out", str(output_path)],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads(output_path.read_text(encoding="utf-8"))
        self.assertEqual(summary["median_allocated_cpu_hours_avoided"], 0.0167)
        self.assertEqual(summary["median_downstream_tasks_started_without_gate"], 1)


if __name__ == "__main__":
    unittest.main()
