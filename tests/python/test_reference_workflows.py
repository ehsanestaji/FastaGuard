import json
import os
import shlex
import shutil
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

import jsonschema


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "examples/reference/fixtures"
MANIFEST = FIXTURES / "workflow-cases.json"
NEXTFLOW = ROOT / "examples/reference/nextflow/main.nf"
SNAKEMAKE = ROOT / "examples/reference/snakemake/Snakefile"


class ReferenceWorkflowTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if (
            os.environ.get("FASTAGUARD_REQUIRE_WORKFLOW_ENGINES") == "1"
            and os.environ.get("FASTAGUARD_WORKFLOW_CASE")
        ):
            raise RuntimeError("Required workflow tests must run every fixture case")
        cls.cases = json.loads(MANIFEST.read_text())["cases"]
        cls.binary = Path(
            os.environ.get("FASTAGUARD_BINARY", ROOT / "target/debug/fastaguard")
        ).resolve(strict=True)
        cls.schema = json.loads(
            subprocess.check_output([str(cls.binary), "--schema"], text=True)
        )

    def file(self, name):
        return str(FIXTURES / name)

    def config_for(self, case, root, mode="enforce"):
        config = {
            "fasta": self.file(case.get("fasta", "reference.fa")),
            "output_dir": str(root / "reports with spaces"),
            "run_id": case["id"],
            "mode": mode,
            "policy": case.get("policy", "coordinate"),
            "require": case.get("require", []),
            "write_lock": case.get("write_lock", False),
            "binary": str(self.binary),
        }
        for key in ("fai", "dict", "alias_map"):
            if key in case:
                config[key] = self.file(case[key])
        for key in ("alignments", "variants", "annotations"):
            config[key] = [self.file(value) for value in case.get(key, [])]
        return config

    def cli_args(self, config, json_path):
        args = [str(self.binary), "reference", config["fasta"]]
        for key, flag in (
            ("fai", "--fai"), ("dict", "--dict"),
            ("alias_map", "--alias-map"),
        ):
            if config.get(key):
                args.extend([flag, config[key]])
        for key, flag in (
            ("alignments", "--alignment"),
            ("variants", "--variants"),
            ("annotations", "--annotation"),
        ):
            for value in config.get(key, []):
                args.extend([flag, value])
        args.extend(["--policy", config["policy"]])
        if config["require"]:
            args.extend(["--require", ",".join(config["require"])])
        args.extend(["--format", "json", "--json", str(json_path)])
        return args

    def assert_reports(self, directory, expected, diagnostic=""):
        files = [
            directory / f"reference.fastaguard.{suffix}"
            for suffix in ("html", "json", "tsv")
        ] + [directory / "reference.fastaguard_mqc.json"]
        self.assertTrue(
            all(path.is_file() and path.stat().st_size for path in files),
            diagnostic,
        )
        report = json.loads(files[1].read_text())
        jsonschema.validate(report, self.schema)
        self.assertIs(report["gate"]["can_continue"], expected)
        return report

    def test_manifest_matches_the_published_cli(self):
        self.assertEqual(len(self.cases), 8)
        with TemporaryDirectory() as temp:
            root = Path(temp)
            for case in self.cases:
                with self.subTest(case=case["id"]):
                    config = self.config_for(case, root)
                    report_path = root / f"{case['id']}.json"
                    result = subprocess.run(
                        self.cli_args(config, report_path),
                        text=True, capture_output=True,
                    )
                    if case["reportable"]:
                        self.assertEqual(result.returncode, 0, result.stderr)
                        report = json.loads(report_path.read_text())
                        jsonschema.validate(report, self.schema)
                        self.assertIs(
                            report["gate"]["can_continue"], case["can_continue"]
                        )
                    else:
                        self.assertNotEqual(result.returncode, 0)
                        self.assertFalse(report_path.exists())

    def nextflow_command(self):
        jar = os.environ.get("FASTAGUARD_NEXTFLOW_JAR")
        if jar:
            return [
                os.environ.get("FASTAGUARD_JAVA", "java"), "-jar", jar,
            ]
        executable = shutil.which("nextflow")
        if executable:
            return [executable]
        if os.environ.get("FASTAGUARD_REQUIRE_WORKFLOW_ENGINES") == "1":
            self.fail("Nextflow executable unavailable")
        self.skipTest("Nextflow executable unavailable")

    def snakemake_command(self):
        executable = os.environ.get("FASTAGUARD_SNAKEMAKE") or shutil.which(
            "snakemake"
        )
        if executable:
            return [executable]
        if os.environ.get("FASTAGUARD_REQUIRE_WORKFLOW_ENGINES") == "1":
            self.fail("Snakemake executable unavailable")
        self.skipTest("Snakemake executable unavailable")

    def run_engine(self, engine, config, root, resume=False, extra_args=(), env=None):
        params_file = root / "parameters.json"
        params_file.write_text(json.dumps(config))
        if engine == "nextflow":
            command = self.nextflow_command() + [
                "run", str(NEXTFLOW), "-offline", "-ansi-log", "false",
                "-work-dir", str(root / "nextflow-work"),
                "-params-file", str(params_file),
            ]
            if resume:
                command.append("-resume")
            command.extend(extra_args)
            cwd = root
        else:
            workspace = root / "snakemake-work"
            workspace.mkdir(exist_ok=True)
            command = self.snakemake_command() + [
                "--snakefile", str(SNAKEMAKE), "--cores", "1",
                "--directory", str(workspace),
                "--configfile", str(params_file), "--quiet",
            ]
            command.extend(extra_args)
            cwd = root
        return subprocess.run(command, cwd=cwd, text=True, capture_output=True, env=env)

    def wrapper(self, root, name="fastaguard-wrapper"):
        wrapper = root / name
        wrapper.write_text(
            "#!/bin/sh\n"
            "if [ \"${FASTAGUARD_TEST_FAIL:-0}\" = 1 ]; then exit 7; fi\n"
            f"exec {shlex.quote(str(self.binary))} \"$@\"\n"
        )
        wrapper.chmod(0o755)
        return wrapper

    def check_engine_matrix(self, engine):
        selected = os.environ.get("FASTAGUARD_WORKFLOW_CASE")
        for case in self.cases:
            if selected and case["id"] != selected:
                continue
            with self.subTest(engine=engine, case=case["id"]):
                with TemporaryDirectory(prefix=f"{engine} reference ") as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    result = self.run_engine(engine, config, root)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    marker = directory / "downstream.started"
                    if case["reportable"]:
                        report = self.assert_reports(
                            directory, case["can_continue"],
                            result.stdout + result.stderr,
                        )
                        self.assertEqual(report["report_type"], "reference")
                        self.assertEqual(
                            (directory / "reference.fastaguard.lock.json").exists(),
                            case.get("write_lock", False),
                        )
                        self.assertEqual(
                            result.returncode == 0, case["can_continue"],
                            result.stdout + result.stderr,
                        )
                        self.assertEqual(marker.exists(), case["can_continue"])
                        if not case["can_continue"]:
                            self.assertIn("gate.can_continue=false", result.stdout + result.stderr)
                    else:
                        self.assertNotEqual(result.returncode, 0)
                        self.assertFalse((directory / "reference.fastaguard.json").exists())
                        self.assertFalse(marker.exists())

    def test_nextflow_matrix(self):
        self.check_engine_matrix("nextflow")

    def test_snakemake_matrix(self):
        self.check_engine_matrix("snakemake")

    def test_report_only_preserves_blocked_evidence(self):
        chosen = {case["id"]: case for case in self.cases}
        for engine in ("nextflow", "snakemake"):
            for name in ("exact", "length-coordinate"):
                with self.subTest(engine=engine, case=name):
                    with TemporaryDirectory() as temp:
                        root = Path(temp)
                        case = chosen[name]
                        config = self.config_for(case, root, mode="report")
                        result = self.run_engine(engine, config, root)
                        directory = Path(config["output_dir"]) / config["run_id"]
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        self.assert_reports(directory, case["can_continue"])
                        self.assertEqual(
                            (directory / "reference.fastaguard.lock.json").exists(),
                            case.get("write_lock", False),
                        )
                        self.assertFalse((directory / "downstream.started").exists())
                        self.assertFalse((directory / "reference.accepted.sha256").exists())

    def test_unsafe_run_ids_fail_before_reporting(self):
        case = self.cases[0]
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    config["run_id"] = "../escape"
                    result = self.run_engine(engine, config, root)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertFalse((root / "escape").exists())
                    self.assertFalse((Path(config["output_dir"]) / "escape").exists())

    def test_repeated_companions_and_aliases(self):
        case = json.loads(MANIFEST.read_text())["repeated"]
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory(prefix="repeated reference ") as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    executable = root / "tool with spaces"
                    executable.symlink_to(self.binary)
                    config["binary"] = str(executable)
                    result = self.run_engine(engine, config, root)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    report = self.assert_reports(directory, True, result.stdout + result.stderr)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    kinds = [item["kind"] for item in report["comparisons"]]
                    for kind in ("fai", "dict", "alignment", "annotation"):
                        self.assertIn(kind, kinds)
                    self.assertEqual(kinds.count("variants"), 2)
                    self.assertTrue((directory / "downstream.started").exists())

    def test_changed_inputs_do_not_reuse_an_old_acceptance(self):
        original = {case["id"]: case for case in self.cases}["exact"]
        case = dict(original, write_lock=False)
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    dictionary = root / "reference copy.dict"
                    dictionary.write_bytes((FIXTURES / "reference.dict").read_bytes())
                    config = self.config_for(case, root)
                    config["dict"] = str(dictionary)
                    first = self.run_engine(engine, config, root)
                    initial_dir = Path(config["output_dir"]) / config["run_id"]
                    self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                    self.assert_reports(initial_dir, True)
                    original_report = (
                        initial_dir / "reference.fastaguard.json"
                    ).read_bytes()
                    original_marker = (initial_dir / "downstream.started").read_bytes()

                    dictionary.write_bytes(
                        (FIXTURES / "copied-length-mismatch.dict").read_bytes()
                    )
                    config["run_id"] = "changed-input"
                    second = self.run_engine(engine, config, root, resume=True)
                    changed_dir = Path(config["output_dir"]) / config["run_id"]
                    self.assertNotEqual(second.returncode, 0)
                    self.assert_reports(
                        changed_dir, False, second.stdout + second.stderr
                    )
                    self.assertFalse((changed_dir / "downstream.started").exists())
                    self.assertEqual(
                        (initial_dir / "reference.fastaguard.json").read_bytes(),
                        original_report,
                    )
                    self.assertEqual(
                        (initial_dir / "downstream.started").read_bytes(),
                        original_marker,
                    )

    def test_same_input_resume_keeps_published_reports(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    first = self.run_engine(engine, config, root)
                    self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    before = {
                        path.name: path.read_bytes()
                        for path in directory.iterdir() if path.is_file()
                    }
                    second = self.run_engine(engine, config, root, resume=True)
                    self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
                    self.assertEqual(
                        before,
                        {
                            path.name: path.read_bytes()
                            for path in directory.iterdir() if path.is_file()
                        },
                    )

    def test_changed_input_cannot_reuse_the_same_run_id(self):
        original = {item["id"]: item for item in self.cases}["exact"]
        case = dict(original, write_lock=False)
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    dictionary = root / "mutable.dict"
                    dictionary.write_bytes((FIXTURES / "reference.dict").read_bytes())
                    config = self.config_for(case, root)
                    config["dict"] = str(dictionary)
                    first = self.run_engine(engine, config, root)
                    self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    before = {
                        path.name: path.read_bytes()
                        for path in directory.iterdir() if path.is_file()
                    }
                    dictionary.write_bytes(
                        (FIXTURES / "copied-length-mismatch.dict").read_bytes()
                    )
                    second = self.run_engine(engine, config, root, resume=True)
                    self.assertNotEqual(second.returncode, 0)
                    self.assertEqual(
                        before,
                        {
                            path.name: path.read_bytes()
                            for path in directory.iterdir() if path.is_file()
                        },
                        second.stdout + second.stderr,
                    )

    def test_failed_snakemake_rerun_preserves_published_reports(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            config = self.config_for(case, root)
            config["binary"] = str(self.wrapper(root))
            first = self.run_engine("snakemake", config, root)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            directory = Path(config["output_dir"]) / config["run_id"]
            before = {
                path.name: path.read_bytes()
                for path in directory.glob("reference.fastaguard*")
            }
            marker = (directory / "downstream.started").read_bytes()
            env = dict(os.environ, FASTAGUARD_TEST_FAIL="1")
            failed = self.run_engine(
                "snakemake", config, root,
                extra_args=("--forcerun", "reference_report"), env=env,
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(
                before,
                {
                    path.name: path.read_bytes()
                    for path in directory.glob("reference.fastaguard*")
                },
                failed.stdout + failed.stderr,
            )
            self.assertEqual((directory / "downstream.started").read_bytes(), marker)
            retry = self.run_engine("snakemake", config, root)
            self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)

    def test_snakemake_retries_after_first_cli_failure(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            config = self.config_for(case, root)
            config["binary"] = str(self.wrapper(root))
            failed = self.run_engine(
                "snakemake", config, root,
                env=dict(os.environ, FASTAGUARD_TEST_FAIL="1"),
            )
            self.assertNotEqual(failed.returncode, 0)
            retry = self.run_engine("snakemake", config, root)
            self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)
            self.assert_reports(Path(config["output_dir"]) / config["run_id"], True)

    def test_nextflow_rejects_changed_vcf_records_with_same_declaration(self):
        case = json.loads(MANIFEST.read_text())["repeated"]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            config = self.config_for(case, root)
            variant = root / "sites.vcf"
            variant.write_bytes((FIXTURES / "variants/a/sites.vcf").read_bytes())
            config["variants"][0] = str(variant)
            first = self.run_engine("nextflow", config, root)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            directory = Path(config["output_dir"]) / config["run_id"]
            before = (directory / "reference.fastaguard.json").read_bytes()
            variant.write_text(variant.read_text() + "chr1\t1\t.\tA\tC\t.\tPASS\t.\n")
            changed = self.run_engine("nextflow", config, root, resume=True)
            self.assertNotEqual(changed.returncode, 0, changed.stdout + changed.stderr)
            self.assertEqual((directory / "reference.fastaguard.json").read_bytes(), before)

    def test_nextflow_tracks_a_path_resolved_executable(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            wrapper = self.wrapper(root, "fastaguard-test")
            config = self.config_for(case, root)
            config["binary"] = wrapper.name
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}")
            first = self.run_engine("nextflow", config, root, env=env)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            directory = Path(config["output_dir"]) / config["run_id"]
            before = (directory / "reference.fastaguard.json").read_bytes()
            wrapper.write_text(wrapper.read_text() + "# executable changed\n")
            changed = self.run_engine("nextflow", config, root, resume=True, env=env)
            self.assertNotEqual(changed.returncode, 0, changed.stdout + changed.stderr)
            self.assertEqual((directory / "reference.fastaguard.json").read_bytes(), before)

    def test_changed_mode_requires_a_new_run_id(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        for engine in ("nextflow", "snakemake"):
            with self.subTest(engine=engine):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    first = self.run_engine(engine, config, root)
                    self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                    config["mode"] = "report"
                    changed = self.run_engine(engine, config, root, resume=True)
                    self.assertNotEqual(changed.returncode, 0)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    self.assertTrue((directory / "downstream.started").is_file())

    def test_snakemake_detects_missing_or_modified_published_report(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        for damage in ("missing", "modified"):
            with self.subTest(damage=damage):
                with TemporaryDirectory() as temp:
                    root = Path(temp)
                    config = self.config_for(case, root)
                    first = self.run_engine("snakemake", config, root)
                    self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
                    directory = Path(config["output_dir"]) / config["run_id"]
                    report = directory / "reference.fastaguard.tsv"
                    if damage == "missing":
                        report.unlink()
                    else:
                        report.write_text(report.read_text() + "altered\n")
                    rerun = self.run_engine("snakemake", config, root)
                    self.assertNotEqual(rerun.returncode, 0, rerun.stdout + rerun.stderr)

    def test_failed_snakemake_republish_can_retry(self):
        case = {item["id"]: item for item in self.cases}["exact"]
        with TemporaryDirectory() as temp:
            root = Path(temp)
            config = self.config_for(case, root)
            first = self.run_engine("snakemake", config, root)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            directory = Path(config["output_dir"]) / config["run_id"]
            published = directory / "reference.fastaguard.html"
            original = published.read_bytes()
            staged = directory / ".staging/reference.fastaguard.html"
            staged.write_bytes(original + b"changed\n")
            failed = self.run_engine(
                "snakemake", config, root,
                extra_args=("--forcerun", "publish_reports"),
            )
            self.assertNotEqual(failed.returncode, 0)
            self.assertEqual(published.read_bytes(), original)
            self.assertTrue((directory / ".reference-inputs.sha256").is_file())
            staged.write_bytes(original)
            retry = self.run_engine("snakemake", config, root)
            self.assertEqual(retry.returncode, 0, retry.stdout + retry.stderr)
            self.assertTrue(
                (directory / ".reference-published").is_file(),
                retry.stdout + retry.stderr,
            )


class WorkflowEngineRequirementsTest(unittest.TestCase):
    def test_missing_engines_skip_when_optional(self):
        case = ReferenceWorkflowTest("test_nextflow_matrix")
        with patch.dict(os.environ, {}, clear=True), patch("shutil.which", return_value=None):
            for command in (case.nextflow_command, case.snakemake_command):
                with self.subTest(engine=command.__name__):
                    with self.assertRaises(unittest.SkipTest):
                        command()

    def test_missing_engines_fail_when_required(self):
        case = ReferenceWorkflowTest("test_nextflow_matrix")
        with patch.dict(
            os.environ, {"FASTAGUARD_REQUIRE_WORKFLOW_ENGINES": "1"}, clear=True
        ), patch("shutil.which", return_value=None):
            for command in (case.nextflow_command, case.snakemake_command):
                with self.subTest(engine=command.__name__):
                    with self.assertRaises(Exception) as raised:
                        command()
                    self.assertIsInstance(raised.exception, AssertionError)

    def test_required_matrix_rejects_a_single_case_filter(self):
        with patch.dict(os.environ, {
            "FASTAGUARD_REQUIRE_WORKFLOW_ENGINES": "1",
            "FASTAGUARD_WORKFLOW_CASE": "exact",
        }, clear=True):
            with self.assertRaisesRegex(RuntimeError, "every fixture case"):
                ReferenceWorkflowTest.setUpClass()


if __name__ == "__main__":
    unittest.main()
