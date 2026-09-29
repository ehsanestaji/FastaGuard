import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.benchmark_workflow_savings import summarise_repeats


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "benchmark_workflow_savings.py"


def observation_pair(run_id="repeat-1"):
    context = {
        "run_id": run_id,
        "engine": "nextflow",
        "case_id": "copied-fai-length-mismatch",
        "input_sha256": "a" * 64,
        "binary_sha256": "b" * 64,
        "runner_id": "linux-test-runner",
        "status": "completed",
    }
    gated = {
        **context,
        "gate_can_continue": False,
        "preflight_wall_seconds": 1,
        "preflight_cpu_seconds": 0.5,
        "preflight_peak_rss_kib": 100,
        "downstream_tasks": [],
    }
    ungated = {
        **context,
        "downstream_tasks": [{
            "name": "map_and_sort", "wall_seconds": 30, "cpu_seconds": 20,
            "requested_cpus": 2, "peak_rss_kib": 400,
        }],
    }
    return gated, ungated


class WorkflowSavingsBenchmarkTest(unittest.TestCase):
    def test_summarises_late_validation_work_avoided_by_preflight(self):
        gated, ungated = [], []
        for i in range(3):
            gate, baseline = observation_pair(f"repeat-{i + 1}")
            gate.update(
                preflight_wall_seconds=0.4 + i * 0.1,
                preflight_cpu_seconds=0.2 + i * 0.1,
                preflight_peak_rss_kib=100 + i * 10,
            )
            baseline["downstream_tasks"][0].update(
                wall_seconds=120 + i * 30, cpu_seconds=180 + i * 30,
                peak_rss_kib=800 + i * 100,
            )
            gated.append(gate)
            ungated.append(baseline)
        summary = summarise_repeats(gated, ungated)
        expected = {
            "summary_schema_version": 2, "repeat_count": 3,
            "median_preflight_wall_seconds": 0.5,
            "median_downstream_tasks_started_without_gate": 1,
            "median_allocated_cpu_hours_avoided": 300 / 3600,
            "median_actual_cpu_seconds_avoided": 209.7,
            "median_task_summed_wall_seconds_avoided": 149.5,
            "median_preflight_peak_rss_kib": 110,
            "median_downstream_peak_rss_kib_without_gate": 900,
            "estimate_kind": "observed", "timed_out_run_ids": [],
        }
        for field, value in expected.items():
            with self.subTest(field=field):
                self.assertEqual(summary[field], value)
        self.assertNotIn("median_wall_seconds_avoided", summary)
        self.assertIn("late validation", summary["interpretation"].lower())

    def test_pairs_by_run_id_without_mutating_input(self):
        gated, ungated = [], []
        for i, cost in enumerate((1, 10, 100)):
            gate, baseline = observation_pair(f"repeat-{i + 1}")
            gate.update(preflight_wall_seconds=cost, preflight_cpu_seconds=cost)
            baseline["downstream_tasks"][0].update(
                wall_seconds=cost * 6, cpu_seconds=cost * 5,
            )
            gated.append(gate)
            ungated.append(baseline)
        shuffled = [ungated[2], ungated[0], ungated[1]]
        original = copy.deepcopy((gated, shuffled))
        summary = summarise_repeats(gated, shuffled)
        self.assertEqual(summary["median_actual_cpu_seconds_avoided"], 40)
        self.assertEqual(summary["median_task_summed_wall_seconds_avoided"], 50)
        self.assertEqual(summary["run_ids"], ["repeat-1", "repeat-2", "repeat-3"])
        self.assertEqual(summary, summarise_repeats(gated[::-1], ungated))
        self.assertEqual((gated, shuffled), original)

    def test_rejects_missing_or_invalid_run_ids(self):
        for side in (0, 1):
            for value in (None, "", "  ", True, 1, [], {}):
                with self.subTest(side=side, value=value):
                    pair = observation_pair()
                    pair[side]["run_id"] = value
                    with self.assertRaisesRegex(ValueError, "run_id"):
                        summarise_repeats([pair[0]], [pair[1]])
            pair = observation_pair()
            del pair[side]["run_id"]
            with self.assertRaisesRegex(ValueError, "run_id"):
                summarise_repeats([pair[0]], [pair[1]])

    def test_rejects_duplicate_ids_and_unmatched_pairs(self):
        first, second = observation_pair("one"), observation_pair("two")
        for gated, ungated in (
            ([first[0], first[0]], [first[1], second[1]]),
            ([first[0], second[0]], [first[1], first[1]]),
            ([first[0]], [second[1]]),
            ([first[0]], [first[1], second[1]]),
        ):
            with self.subTest(gated=gated, ungated=ungated):
                with self.assertRaises(ValueError):
                    summarise_repeats(gated, ungated)

    def test_rejects_empty_or_malformed_observation_collections(self):
        gated, ungated = observation_pair()
        for bad in ([], {}, None, "observations", [None], [1], [[]]):
            with self.subTest(value=bad):
                with self.assertRaises(ValueError):
                    summarise_repeats(bad, [ungated])
                with self.assertRaises(ValueError):
                    summarise_repeats([gated], bad)

    def test_rejects_inconsistent_contexts_and_mixed_repeat_groups(self):
        replacements = {
            "engine": "snakemake", "case_id": "different-mismatch",
            "input_sha256": "c" * 64, "binary_sha256": "d" * 64,
            "runner_id": "different-runner",
        }
        for field, value in replacements.items():
            with self.subTest(field=field):
                gated, ungated = observation_pair()
                ungated[field] = value
                with self.assertRaisesRegex(ValueError, "context"):
                    summarise_repeats([gated], [ungated])
                other_gate, other_baseline = observation_pair("repeat-2")
                other_gate[field] = other_baseline[field] = value
                ungated[field] = gated[field]
                with self.assertRaisesRegex(ValueError, "context"):
                    summarise_repeats([gated, other_gate], [ungated, other_baseline])

    def test_rejects_missing_or_invalid_context(self):
        for field in ("engine", "case_id", "input_sha256", "binary_sha256", "runner_id"):
            for value in (None, "", "  ", True, 1, []):
                with self.subTest(field=field, value=value):
                    gated, ungated = observation_pair()
                    gated[field] = ungated[field] = value
                    with self.assertRaises(ValueError):
                        summarise_repeats([gated], [ungated])
            gated, ungated = observation_pair()
            del gated[field]
            with self.assertRaises(ValueError):
                summarise_repeats([gated], [ungated])
        for digest in ("abc", "g" * 64, "a" * 63, "a" * 65):
            gated, ungated = observation_pair()
            gated["input_sha256"] = ungated["input_sha256"] = digest
            with self.assertRaisesRegex(ValueError, "sha256"):
                summarise_repeats([gated], [ungated])

    def test_requires_completed_blocking_gate_with_no_downstream_work(self):
        for field, values in (
            ("gate_can_continue", (None, True, 0, "false")),
            ("status", (None, "timeout", "failed", True)),
            ("downstream_tasks", (None, {}, [{"name": "unexpected"}])),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    gated, ungated = observation_pair()
                    gated[field] = value
                    with self.assertRaises(ValueError):
                        summarise_repeats([gated], [ungated])

    def test_requires_measured_baseline_tasks_and_supported_status(self):
        for field, values in (
            ("status", (None, "failed", "cancelled", True)),
            ("downstream_tasks", ([], None, {}, [None], [1], [{}])),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    gated, ungated = observation_pair()
                    ungated[field] = value
                    with self.assertRaises(ValueError):
                        summarise_repeats([gated], [ungated])

    def test_rejects_invalid_measurements(self):
        locations = (
            ("gate", "preflight_wall_seconds"), ("gate", "preflight_cpu_seconds"),
            ("gate", "preflight_peak_rss_kib"), ("task", "wall_seconds"),
            ("task", "cpu_seconds"), ("task", "peak_rss_kib"),
        )
        for location, field in locations:
            for value in (-1, float("nan"), float("inf"), -float("inf"), True, "1", None, 10**400):
                with self.subTest(location=location, field=field, value=value):
                    gated, ungated = observation_pair()
                    target = gated if location == "gate" else ungated["downstream_tasks"][0]
                    target[field] = value
                    with self.assertRaises(ValueError):
                        summarise_repeats([gated], [ungated])
            gated, ungated = observation_pair()
            target = gated if location == "gate" else ungated["downstream_tasks"][0]
            del target[field]
            with self.assertRaises(ValueError):
                summarise_repeats([gated], [ungated])

    def test_requires_positive_integer_cpu_allocations_and_task_names(self):
        for field, values in (
            ("requested_cpus", (0, -1, 1.5, True, None, "2", float("inf"))),
            ("name", (None, "", "  ", True, 1)),
        ):
            for value in values:
                with self.subTest(field=field, value=value):
                    gated, ungated = observation_pair()
                    ungated["downstream_tasks"][0][field] = value
                    with self.assertRaises(ValueError):
                        summarise_repeats([gated], [ungated])

    def test_rejects_arithmetic_overflow(self):
        for field in ("wall_seconds", "cpu_seconds", "requested_cpus"):
            with self.subTest(field=field):
                gated, ungated = observation_pair()
                task = ungated["downstream_tasks"][0]
                task[field] = 10**400 if field == "requested_cpus" else 1e308
                ungated["downstream_tasks"].append({**task, "name": "second"})
                with self.assertRaisesRegex(ValueError, "finite"):
                    summarise_repeats([gated], [ungated])

    def test_preserves_negative_net_savings(self):
        gated, ungated = observation_pair()
        gated.update(preflight_wall_seconds=100, preflight_cpu_seconds=200)
        summary = summarise_repeats([gated], [ungated])
        self.assertEqual(summary["median_actual_cpu_seconds_avoided"], -180)
        self.assertEqual(summary["median_task_summed_wall_seconds_avoided"], -70)

    def test_separates_task_sums_from_whole_workflow_elapsed(self):
        gated, ungated = observation_pair()
        gated.update(preflight_wall_seconds=10, workflow_wall_seconds=12)
        ungated["workflow_wall_seconds"] = 35
        ungated["downstream_tasks"].append({
            **ungated["downstream_tasks"][0], "name": "parallel_task",
        })
        summary = summarise_repeats([gated], [ungated])
        self.assertEqual(summary["median_task_summed_wall_seconds_avoided"], 50)
        self.assertEqual(summary["median_workflow_wall_seconds_avoided"], 23)

    def test_cli_preserves_small_negative_savings(self):
        gated, ungated = observation_pair()
        gated.update(
            preflight_wall_seconds=0.00001, preflight_cpu_seconds=0.00001,
            workflow_wall_seconds=0.00001,
        )
        ungated["downstream_tasks"][0].update(wall_seconds=0, cpu_seconds=0)
        ungated["workflow_wall_seconds"] = 0
        completed, output = self.run_cli([gated], [ungated])
        self.assertEqual(completed.returncode, 0, completed.stderr)
        summary = json.loads(output)
        for field in (
            "median_actual_cpu_seconds_avoided",
            "median_task_summed_wall_seconds_avoided",
            "median_workflow_wall_seconds_avoided",
        ):
            with self.subTest(field=field):
                self.assertEqual(summary[field], -0.00001)

    def test_cli_does_not_round_up_timeout_lower_bounds(self):
        gated, ungated = observation_pair()
        ungated["status"] = "timeout"
        ungated["downstream_tasks"][0].update(wall_seconds=0.1)
        completed, output = self.run_cli([gated], [ungated])
        self.assertEqual(completed.returncode, 0, completed.stderr)
        summary = json.loads(output)
        self.assertEqual(summary["estimate_kind"], "lower_bound")
        self.assertEqual(summary["median_allocated_cpu_hours_avoided"], 0.2 / 3600)

    def test_does_not_infer_missing_workflow_elapsed(self):
        gated, ungated = observation_pair()
        summary = summarise_repeats([gated], [ungated])
        self.assertIsNone(summary["median_workflow_wall_seconds_avoided"])

    def test_requires_workflow_elapsed_for_all_pairs_or_none(self):
        for side in (0, 1):
            pair = observation_pair()
            pair[side]["workflow_wall_seconds"] = 100
            with self.assertRaisesRegex(ValueError, "workflow_wall_seconds"):
                summarise_repeats([pair[0]], [pair[1]])
        first, second = observation_pair("one"), observation_pair("two")
        first[0]["workflow_wall_seconds"] = 2
        first[1]["workflow_wall_seconds"] = 32
        with self.assertRaisesRegex(ValueError, "workflow_wall_seconds"):
            summarise_repeats([first[0], second[0]], [first[1], second[1]])

    def test_rejects_invalid_workflow_elapsed(self):
        for side in (0, 1):
            for value in (None, True, -1, float("nan"), float("inf"), "1"):
                pair = observation_pair()
                pair[0]["workflow_wall_seconds"] = 2
                pair[1]["workflow_wall_seconds"] = 32
                pair[side]["workflow_wall_seconds"] = value
                with self.subTest(side=side, value=value):
                    with self.assertRaises(ValueError):
                        summarise_repeats([pair[0]], [pair[1]])

    def test_timeout_is_censored_not_an_exact_saving(self):
        gated, ungated = observation_pair()
        ungated["status"] = "timeout"
        summary = summarise_repeats([gated], [ungated])
        self.assertEqual(summary["estimate_kind"], "lower_bound")
        self.assertEqual(summary["timed_out_run_ids"], ["repeat-1"])
        self.assertEqual(summary["completed_pair_count"], 0)
        self.assertEqual(summary["censored_pair_count"], 1)
        self.assertEqual(summary["median_task_summed_wall_seconds_avoided"], 29)
        self.assertIn("lower bound", summary["interpretation"].lower())

    def test_mixed_complete_and_timeout_repeats_retain_censoring(self):
        first, second = observation_pair("one"), observation_pair("two")
        second[1]["status"] = "timeout"
        summary = summarise_repeats([first[0], second[0]], [first[1], second[1]])
        self.assertEqual(summary["estimate_kind"], "lower_bound")
        self.assertEqual(summary["completed_pair_count"], 1)
        self.assertEqual(summary["censored_pair_count"], 1)
        self.assertEqual(summary["timed_out_run_ids"], ["two"])

    def run_cli(self, gated, ungated, existing_output=None, use_input_as_output=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gated_path, ungated_path = root / "gated.json", root / "ungated.json"
            gated_path.write_text(gated if isinstance(gated, str) else json.dumps(gated))
            ungated_path.write_text(json.dumps(ungated))
            output_path = gated_path if use_input_as_output else root / "summary.json"
            if existing_output is not None:
                output_path.write_text(existing_output)
            completed = subprocess.run(
                [sys.executable, str(SCRIPT), "--gated", str(gated_path),
                 "--ungated", str(ungated_path), "--out", str(output_path)],
                cwd=ROOT, capture_output=True, text=True,
            )
            output = output_path.read_text() if output_path.exists() else None
        return completed, output

    def test_cli_writes_portable_summary_from_matched_observations(self):
        gated, ungated = observation_pair()
        completed, output = self.run_cli([gated], [ungated])
        self.assertEqual(completed.returncode, 0, completed.stderr)
        summary = json.loads(output)
        self.assertEqual(summary, json.loads(completed.stdout))
        self.assertEqual(summary["median_allocated_cpu_hours_avoided"], 60 / 3600)
        self.assertEqual(summary["median_downstream_tasks_started_without_gate"], 1)
        self.assertEqual(summary["context"]["input_sha256"], "a" * 64)

    def test_cli_rejects_malformed_or_ambiguous_json_without_publishing(self):
        gated, ungated = observation_pair()
        raw = json.dumps([gated])
        invalid = (
            "not json", "{}", "[]", "[null]",
            raw.replace('"preflight_wall_seconds": 1', '"preflight_wall_seconds": NaN'),
            raw.replace('"preflight_wall_seconds": 1', '"preflight_wall_seconds": Infinity'),
            raw.replace('"preflight_wall_seconds": 1', '"preflight_wall_seconds": 1e999'),
            raw.replace('"run_id": "repeat-1"', '"run_id": "bad", "run_id": "repeat-1"'),
        )
        for document in invalid:
            with self.subTest(document=document):
                completed, output = self.run_cli(document, [ungated])
                self.assertEqual(completed.returncode, 2)
                self.assertEqual(completed.stdout, "")
                self.assertNotIn("Traceback", completed.stderr)
                self.assertIsNone(output)

    def test_cli_preserves_existing_summary_and_raw_observations(self):
        gated, ungated = observation_pair()
        for document in ([gated], "not json"):
            completed, output = self.run_cli(document, [ungated], existing_output="existing\n")
            self.assertEqual(completed.returncode, 2)
            self.assertNotIn("Traceback", completed.stderr)
            self.assertEqual(output, "existing\n")
        completed, output = self.run_cli([gated], [ungated], use_input_as_output=True)
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(json.loads(output), [gated])


if __name__ == "__main__":
    unittest.main()
