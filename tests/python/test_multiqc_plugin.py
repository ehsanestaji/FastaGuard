import email
import json
import os
import subprocess
import sys
import tarfile
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[2]
PACKAGE_ROOT = ROOT / "integrations" / "multiqc"
sys.path.insert(0, str(PACKAGE_ROOT / "src"))

from fastaguard_multiqc.parser import load_custom_content_summary


class MultiqcPluginReleaseTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.test_dir = TemporaryDirectory(prefix="fastaguard-multiqc-release-")
        cls.test_path = Path(cls.test_dir.name)
        cls.build_path = cls.test_path / "dist"
        cls.subprocess_env = os.environ.copy()
        cls.subprocess_env.pop("PYTHONHOME", None)
        cls.subprocess_env.pop("PYTHONPATH", None)
        cls.subprocess_env.pop("VIRTUAL_ENV", None)
        cls.subprocess_env.setdefault(
            "PIP_CACHE_DIR", str(cls.test_path / "pip-cache")
        )
        cls.subprocess_env["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "build",
                "--outdir",
                str(cls.build_path),
                str(PACKAGE_ROOT),
            ],
            cwd=ROOT,
            env=cls.subprocess_env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise AssertionError(
                "MultiQC plugin distribution build failed:\n"
                f"STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}"
            )

        cls.wheel = next(cls.build_path.glob("*.whl"))
        cls.sdist = next(cls.build_path.glob("*.tar.gz"))

    @classmethod
    def tearDownClass(cls):
        cls.test_dir.cleanup()

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
            wheel_readme_names = [
                name
                for name in wheel_names
                if name.endswith(".data/data/share/doc/multiqc-fastaguard/README.md")
            ]
            self.assertEqual(len(wheel_readme_names), 1, sorted(wheel_names))
            wheel_readme = archive.read(wheel_readme_names[0]).decode()

        self.assertIn("fastaguard_multiqc/__init__.py", wheel_names)
        self.assertIn("fastaguard_multiqc/parser.py", wheel_names)
        self.assertIn("fastaguard_multiqc/multiqc_module.py", wheel_names)
        self.assertIn("# MultiQC FastaGuard Module", wheel_readme)
        self.assertIn("This phase validates the package locally only.", wheel_readme)

        with tarfile.open(self.sdist, "r:gz") as archive:
            sdist_names = set(archive.getnames())
            root = "multiqc_fastaguard-0.1.0"
            sdist_readme = archive.extractfile(f"{root}/README.md").read().decode()

        self.assertIn(f"{root}/README.md", sdist_names)
        self.assertIn("# MultiQC FastaGuard Module", sdist_readme)
        self.assertIn("This phase validates the package locally only.", sdist_readme)
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
        multiqc_version = os.environ.get("FASTAGUARD_MULTIQC_VERSION", "1.35")
        environment = self.test_path / f"multiqc-{multiqc_version}"
        create = subprocess.run(
            [sys.executable, "-m", "venv", str(environment)],
            cwd=ROOT,
            env=self.subprocess_env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            create.returncode,
            0,
            f"STDOUT:\n{create.stdout}\nSTDERR:\n{create.stderr}",
        )

        bin_dir = environment / ("Scripts" if os.name == "nt" else "bin")
        python = bin_dir / ("python.exe" if os.name == "nt" else "python")
        multiqc = bin_dir / ("multiqc.exe" if os.name == "nt" else "multiqc")
        install = subprocess.run(
            [
                str(python),
                "-m",
                "pip",
                "install",
                str(self.wheel),
                f"multiqc=={multiqc_version}",
            ],
            cwd=ROOT,
            env=self.subprocess_env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            install.returncode,
            0,
            f"STDOUT:\n{install.stdout}\nSTDERR:\n{install.stderr}",
        )

        inspect = subprocess.run(
            [
                str(python),
                "-c",
                (
                    "import json; import fastaguard_multiqc; "
                    "from importlib.metadata import distribution, version; "
                    "dist = distribution('multiqc-fastaguard'); "
                    "print(json.dumps({'direct_url': json.loads(dist.read_text('direct_url.json')), "
                    "'module_file': fastaguard_multiqc.__file__, "
                    "'multiqc_version': version('multiqc'), "
                    "'plugin_version': version('multiqc-fastaguard')}))"
                ),
            ],
            cwd=ROOT,
            env=self.subprocess_env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            inspect.returncode,
            0,
            f"STDOUT:\n{inspect.stdout}\nSTDERR:\n{inspect.stderr}",
        )
        installed = json.loads(inspect.stdout)
        installed_wheel = Path(
            unquote(urlsplit(installed["direct_url"]["url"]).path)
        ).resolve()
        self.assertEqual(installed_wheel, self.wheel.resolve())
        self.assertEqual(installed["multiqc_version"], multiqc_version)
        self.assertEqual(installed["plugin_version"], "0.1.0")
        self.assertTrue(
            Path(installed["module_file"])
            .resolve()
            .is_relative_to(environment.resolve())
        )

        output_dir = self.test_path / f"report-{multiqc_version}"
        result = subprocess.run(
            [
                str(multiqc),
                "--strict",
                "--module",
                "fastaguard",
                "--outdir",
                str(output_dir),
                str(ROOT / "examples" / "reports"),
            ],
            cwd=ROOT,
            env=self.subprocess_env,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
        )

        parsed_table = output_dir / "multiqc_data" / "multiqc_fastaguard.txt"
        self.assertTrue(
            parsed_table.exists(),
            "missing parsed plugin data; "
            f"STDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
        )
        data_path = output_dir / "multiqc_data" / "multiqc_data.json"
        saved_data = json.loads(data_path.read_text())["report_saved_raw_data"]
        parsed = saved_data["multiqc_fastaguard"]

        expected = {
            "valid_assembly": {
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
            "problem_assembly": {
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
        }
        for sample, sample_expected in expected.items():
            self.assertEqual(
                {field: parsed[sample][field] for field in sample_expected},
                sample_expected,
            )


if __name__ == "__main__":
    unittest.main()
