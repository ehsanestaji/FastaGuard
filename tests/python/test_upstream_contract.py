import csv
import json
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
GATE_HELPER = ROOT / "examples" / "workflows" / "check_fastaguard_gate.py"

FIXTURES = {
    "pass": (
        ">clean\n"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT\n"
    ),
    "warn": (
        ">long\n"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT"
        "ACGTACGTACGTACGTACGTACGTACGTACGTACGTACGT\n"
        ">tiny\nACGT\n"
    ),
    "fail": ">dup\nACGTACGT\n>dup\nACGTACGT\n>bad\nACGTXYZ\n",
    "invalid": ">empty_record\n>next_record\nACGT\n",
}

EXPECTED = {
    "pass": ("PASS", [], True),
    "warn": ("WARN", [], True),
    "fail": (
        "FAIL",
        ["duplicate_ids", "duplicate_first_token_ids", "invalid_chars"],
        False,
    ),
    "invalid": ("FAIL", ["invalid_fasta_structure"], False),
}


class UpstreamContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run(
            ["cargo", "build", "--quiet", "--bin", "fastaguard"],
            cwd=ROOT,
            check=True,
        )
        cls.binary = ROOT / "target" / "debug" / "fastaguard"
        schema_result = subprocess.run(
            [str(cls.binary), "--schema"],
            cwd=ROOT,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        )
        schema = json.loads(schema_result.stdout)
        validator_class = jsonschema.validators.validator_for(schema)
        validator_class.check_schema(schema)
        cls.schema_validator = validator_class(schema)

    def run_fastaguard(self, *args):
        return subprocess.run(
            [str(self.binary), *map(str, args)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def assert_report_contract(self, case, extra_args):
        expected_status, expected_blockers, expected_can_continue = EXPECTED[case]
        with TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            fasta = output_dir / f"{case}.fa"
            fasta.write_text(FIXTURES[case])
            outputs = {
                "html": output_dir / f"{case}.html",
                "json": output_dir / f"{case}.json",
                "tsv": output_dir / f"{case}.tsv",
                "multiqc": output_dir / f"{case}_mqc.json",
            }
            result = self.run_fastaguard(
                fasta,
                *extra_args,
                "--out",
                outputs["html"],
                "--json",
                outputs["json"],
                "--tsv",
                outputs["tsv"],
                "--multiqc",
                outputs["multiqc"],
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            for path in outputs.values():
                with self.subTest(case=case, output=path.name):
                    self.assertTrue(path.is_file(), path)
                    self.assertGreater(path.stat().st_size, 0, path)

            report = json.loads(outputs["json"].read_text())
            self.schema_validator.validate(report)
            self.assertEqual(report["verdict"]["status"], expected_status)
            self.assertEqual(report["gate"]["status"], expected_status)
            self.assertIs(report["gate"]["can_continue"], expected_can_continue)
            self.assertEqual(report["gate"]["blocking_findings"], expected_blockers)

            with outputs["tsv"].open(newline="") as handle:
                metrics = {
                    row["metric"]: row["value"]
                    for row in csv.DictReader(handle, delimiter="\t")
                }
            self.assertEqual(metrics["input_path"], str(fasta))
            self.assertEqual(metrics["verdict"], expected_status)
            self.assertEqual(metrics["gate_status"], expected_status)

            multiqc = json.loads(outputs["multiqc"].read_text())
            self.assertEqual(multiqc["id"], "fastaguard")
            self.assertEqual(multiqc["plot_type"], "table")
            self.assertEqual(len(multiqc["data"]), 1)
            summary = next(iter(multiqc["data"].values()))
            self.assertEqual(summary["verdict"], expected_status)
            self.assertEqual(summary["gate_status"], expected_status)

    def test_default_command_shape_reports_pass_warn_fail_and_invalid(self):
        for case in FIXTURES:
            with self.subTest(case=case):
                self.assert_report_contract(case, [])

    def test_explicit_assembly_pipeline_shape_reports_all_cases(self):
        for case in FIXTURES:
            with self.subTest(case=case):
                self.assert_report_contract(
                    case, ["--profile", "assembly", "--gate", "pipeline"]
                )

    def test_version_output_is_stable_and_semver_parseable(self):
        result = self.run_fastaguard("--version")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertRegex(
            result.stdout.strip(),
            r"^fastaguard [0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$",
        )

    def test_cli_parse_errors_exit_two(self):
        result = self.run_fastaguard("--not-a-fastaguard-option")

        self.assertEqual(result.returncode, 2, result.stderr)

    def test_missing_input_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            result = self.run_fastaguard(
                output_dir / "missing.fa",
                "--out",
                output_dir / "report.html",
                "--json",
                output_dir / "report.json",
                "--tsv",
                output_dir / "report.tsv",
                "--multiqc",
                output_dir / "report_mqc.json",
            )

        self.assertEqual(result.returncode, 3, result.stderr)

    def test_output_parent_that_is_a_file_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            fasta = output_dir / "pass.fa"
            fasta.write_text(FIXTURES["pass"])
            blocked_parent = output_dir / "regular-file"
            blocked_parent.write_text("not a directory\n")
            result = self.run_fastaguard(
                fasta,
                "--out",
                blocked_parent / "report.html",
                "--json",
                output_dir / "report.json",
                "--tsv",
                output_dir / "report.tsv",
                "--multiqc",
                output_dir / "report_mqc.json",
            )

        self.assertEqual(result.returncode, 3, result.stderr)

    def test_benchmark_workflow_can_replace_its_owned_outputs_on_repeat_runs(self):
        with TemporaryDirectory() as temp_dir:
            command = [
                "python3",
                str(ROOT / "scripts" / "benchmark_large_fasta.py"),
                "--records",
                "2",
                "--length",
                "32",
                "--binary",
                str(self.binary),
                "--out-dir",
                temp_dir,
            ]

            first = subprocess.run(
                command,
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            second = subprocess.run(
                command,
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(json.loads(first.stdout)["reported_total_length"], 64)
        self.assertEqual(json.loads(second.stdout)["reported_total_length"], 64)

    def test_downstream_gate_can_reject_a_collected_fail_report(self):
        with TemporaryDirectory() as temp_dir:
            output_dir = Path(temp_dir)
            fasta = output_dir / "fail.fa"
            fasta.write_text(FIXTURES["fail"])
            json_report = output_dir / "fail.json"
            report_result = self.run_fastaguard(
                fasta,
                "--profile",
                "assembly",
                "--gate",
                "pipeline",
                "--out",
                output_dir / "fail.html",
                "--json",
                json_report,
                "--tsv",
                output_dir / "fail.tsv",
                "--multiqc",
                output_dir / "fail_mqc.json",
            )
            gate_result = subprocess.run(
                ["python3", str(GATE_HELPER), str(json_report)],
                cwd=ROOT,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

        self.assertEqual(report_result.returncode, 0, report_result.stderr)
        self.assertEqual(gate_result.returncode, 2, gate_result.stderr)
        self.assertIn("verdict=FAIL", gate_result.stdout)
        self.assertIn("gate.status=FAIL", gate_result.stdout)
        self.assertIn("gate.can_continue=false", gate_result.stdout)
        self.assertIn("gate.mode=pipeline", gate_result.stdout)
        self.assertIn(
            'gate.blocking_findings=["duplicate_ids","duplicate_first_token_ids",'
            '"invalid_chars"]',
            gate_result.stdout,
        )


if __name__ == "__main__":
    unittest.main()
