import email
import json
import subprocess
import sys
import tarfile
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
PACKAGE_ROOT = ROOT / "integrations" / "multiqc"
sys.path.insert(0, str(PACKAGE_ROOT / "src"))

from fastaguard_multiqc.parser import load_custom_content_summary


class MultiqcPluginReleaseTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build_dir = TemporaryDirectory(prefix="fastaguard-multiqc-build-")
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "build",
                "--outdir",
                cls.build_dir.name,
                str(PACKAGE_ROOT),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise AssertionError(
                "MultiQC plugin distribution build failed:\n"
                f"STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}"
            )

        cls.wheel = next(Path(cls.build_dir.name).glob("*.whl"))
        cls.sdist = next(Path(cls.build_dir.name).glob("*.tar.gz"))

    @classmethod
    def tearDownClass(cls):
        cls.build_dir.cleanup()

    def test_wheel_metadata_declares_release_contract(self):
        with zipfile.ZipFile(self.wheel) as archive:
            metadata_name = next(
                name for name in archive.namelist() if name.endswith(".dist-info/METADATA")
            )
            entry_points_name = next(
                name
                for name in archive.namelist()
                if name.endswith(".dist-info/entry_points.txt")
            )
            metadata = email.message_from_bytes(archive.read(metadata_name))
            entry_points = archive.read(entry_points_name).decode()

        self.assertEqual(metadata["Name"], "multiqc-fastaguard")
        self.assertEqual(metadata["Version"], "0.1.0")
        self.assertEqual(metadata["Requires-Python"], ">=3.10")
        self.assertEqual(metadata["License-Expression"], "MIT")
        self.assertIn("multiqc>=1.28", metadata.get_all("Requires-Dist", []))
        self.assertEqual(
            set(metadata.get_all("Project-URL", [])),
            {
                "Homepage, https://github.com/ehsanestaji/FastaGuard",
                "Issues, https://github.com/ehsanestaji/FastaGuard/issues",
                "Repository, https://github.com/ehsanestaji/FastaGuard",
            },
        )
        self.assertIn("[multiqc.modules.v1]", entry_points)
        self.assertIn("fastaguard = fastaguard_multiqc:MultiqcModule", entry_points)
        self.assertIn("[multiqc.hooks.v1]", entry_points)
        self.assertIn(
            "before_config = fastaguard_multiqc.parser:register_search_patterns",
            entry_points,
        )

    def test_build_archives_contain_plugin_modules_and_readme(self):
        with zipfile.ZipFile(self.wheel) as archive:
            wheel_names = set(archive.namelist())
            metadata_name = next(
                name for name in wheel_names if name.endswith(".dist-info/METADATA")
            )
            wheel_metadata = archive.read(metadata_name).decode()

        self.assertIn("fastaguard_multiqc/__init__.py", wheel_names)
        self.assertIn("fastaguard_multiqc/parser.py", wheel_names)
        self.assertIn("fastaguard_multiqc/multiqc_module.py", wheel_names)
        self.assertIn("# MultiQC FastaGuard Module", wheel_metadata)

        with tarfile.open(self.sdist, "r:gz") as archive:
            sdist_names = set(archive.getnames())

        root = "multiqc_fastaguard-0.1.0"
        self.assertIn(f"{root}/README.md", sdist_names)
        self.assertIn(f"{root}/src/fastaguard_multiqc/__init__.py", sdist_names)
        self.assertIn(f"{root}/src/fastaguard_multiqc/parser.py", sdist_names)
        self.assertIn(f"{root}/src/fastaguard_multiqc/multiqc_module.py", sdist_names)

    def test_parser_preserves_v0_6_pass_and_fail_fields(self):
        cases = {
            "assembly_pass": (
                "valid_assembly",
                {
                    "verdict": "WARN",
                    "gate_mode": "none",
                    "gate_status": "WARN",
                    "readiness_status": "WARN",
                    "submission_status": "WARN",
                    "submission_target": ".",
                    "unsafe_identifier_count": 0,
                    "long_identifier_count": 0,
                    "duplicate_first_token_id_count": 0,
                    "gap_like_n_run_count": 0,
                },
            ),
            "assembly_fail": (
                "problem_assembly",
                {
                    "verdict": "FAIL",
                    "gate_mode": "none",
                    "gate_status": "FAIL",
                    "readiness_status": "FAIL",
                    "submission_status": "FAIL",
                    "submission_target": ".",
                    "unsafe_identifier_count": 0,
                    "long_identifier_count": 0,
                    "duplicate_first_token_id_count": 1,
                    "gap_like_n_run_count": 0,
                },
            ),
        }

        for report_dir, (sample_name, expected) in cases.items():
            with self.subTest(report=report_dir):
                report = (
                    ROOT
                    / "examples"
                    / "reports"
                    / report_dir
                    / "fastaguard_mqc.json"
                )
                parsed = load_custom_content_summary(report)[sample_name]
                self.assertEqual(
                    {field: parsed[field] for field in expected},
                    expected,
                )

    def test_installed_plugin_runs_strict_multiqc_and_writes_v0_6_fields(self):
        multiqc = Path(sys.executable).parent / "multiqc"
        if not multiqc.exists():
            self.skipTest("strict integration requires the wheel-installed MultiQC dependency")

        with TemporaryDirectory(prefix="fastaguard-multiqc-strict-") as output_dir:
            result = subprocess.run(
                [
                    str(multiqc),
                    "--strict",
                    "--module",
                    "fastaguard",
                    "--outdir",
                    output_dir,
                    str(ROOT / "examples" / "reports"),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                result.returncode,
                0,
                f"STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
            )

            parsed_table = (
                Path(output_dir) / "multiqc_data" / "multiqc_fastaguard.txt"
            )
            self.assertTrue(
                parsed_table.exists(),
                f"missing parsed plugin data; STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
            )
            data_path = Path(output_dir) / "multiqc_data" / "multiqc_data.json"
            saved_data = json.loads(data_path.read_text())["report_saved_raw_data"]
            parsed = saved_data["multiqc_fastaguard"]

        self.assertEqual(parsed["valid_assembly"]["verdict"], "WARN")
        self.assertEqual(parsed["valid_assembly"]["submission_status"], "WARN")
        self.assertEqual(parsed["problem_assembly"]["verdict"], "FAIL")
        self.assertEqual(parsed["problem_assembly"]["gate_status"], "FAIL")
        self.assertEqual(parsed["problem_assembly"]["readiness_status"], "FAIL")
        self.assertEqual(parsed["problem_assembly"]["submission_status"], "FAIL")
        self.assertEqual(parsed["problem_assembly"]["submission_target"], ".")
        self.assertEqual(parsed["problem_assembly"]["unsafe_identifier_count"], 0)
        self.assertEqual(parsed["problem_assembly"]["long_identifier_count"], 0)
        self.assertEqual(
            parsed["problem_assembly"]["duplicate_first_token_id_count"], 1
        )
        self.assertEqual(parsed["problem_assembly"]["gap_like_n_run_count"], 0)


if __name__ == "__main__":
    unittest.main()
