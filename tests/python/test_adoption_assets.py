import json
import re
import subprocess
import sys
import types
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
NFCORE_PR = "https://github.com/nf-core/modules/pull/12239"
SNAKEMAKE_PR = "https://github.com/snakemake/snakemake-wrappers/pull/5436"
sys.path.insert(0, str(ROOT / "integrations" / "multiqc" / "src"))

import fastaguard_multiqc.parser as multiqc_parser
from fastaguard_multiqc.parser import load_custom_content_summary


class AdoptionAssetsTest(unittest.TestCase):
    def read(self, path):
        return (ROOT / path).read_text()

    def test_v0_3_gate_docs_and_examples_are_present(self):
        readme = (ROOT / "README.md").read_text()
        output_contract = (ROOT / "docs" / "output-contract.md").read_text()
        nf_core_module = (
            ROOT
            / "examples"
            / "nf-core"
            / "modules"
            / "local"
            / "fastaguard"
            / "main.nf"
        ).read_text()
        snakemake = (ROOT / "examples" / "snakemake" / "Snakefile").read_text()

        self.assertIn("--gate pipeline", readme)
        self.assertIn("FASTA preflight QC for modern bioinformatics pipelines.", readme)
        self.assertIn("Run FastaGuard first.", readme)
        self.assertIn('"gate"', output_contract)
        self.assertIn("provenance.input_sha256", output_contract)
        nf_core_test_config = self.read(
            "examples/nf-core/modules/local/fastaguard/tests/nextflow.config"
        )
        self.assertNotIn("--gate pipeline", nf_core_module)
        self.assertIn("--profile assembly --gate pipeline", nf_core_test_config)
        self.assertIn("--gate pipeline", snakemake)
        self.assertIn(
            '"blocking_findings": ["duplicate_ids", "invalid_chars", "high_n_rate"]',
            output_contract,
        )
        self.assertIn(
            '"command": "fastaguard sample.fa --profile assembly --gate pipeline"',
            output_contract,
        )
        self.assertIn('"duplicate_id_count": 1', output_contract)
        self.assertIn('"invalid_sequence_count": 1', output_contract)

    def test_v0_3_gate_examples_do_not_pin_v0_2_runtimes(self):
        nf_core_module = (
            ROOT
            / "examples"
            / "nf-core"
            / "modules"
            / "local"
            / "fastaguard"
            / "main.nf"
        ).read_text()
        wrapper_env = (
            ROOT / "examples" / "snakemake" / "wrapper" / "environment.yaml"
        ).read_text()
        wrapper_py = (
            ROOT / "examples" / "snakemake" / "wrapper" / "wrapper.py"
        ).read_text()
        nf_core_readme = (ROOT / "examples" / "nf-core" / "README.md").read_text()
        snakemake_readme = (
            ROOT / "examples" / "snakemake" / "wrapper" / "README.md"
        ).read_text()

        self.assertNotIn("0.2.0--", nf_core_module)
        self.assertIn(
            "quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0",
            nf_core_module,
        )
        self.assertNotIn("fastaguard=0.2.0", wrapper_env)
        self.assertNotIn("--gate {gate}", wrapper_py)
        self.assertIn('extra = snakemake.params.get("extra", "")', wrapper_py)
        self.assertIn('"{extra} "', wrapper_py)
        self.assertNotIn("fastaguard=0.5.0", nf_core_readme)
        self.assertNotIn("fastaguard:0.5.0--", nf_core_readme)
        self.assertIn("task.ext.args", nf_core_readme)
        self.assertNotIn("exit_code", nf_core_readme)
        self.assertIn("params.extra", snakemake_readme)
        self.assertIn("gate.status", snakemake_readme)

    def test_v0_4_docs_explain_preflight_readiness_and_compare_mode(self):
        readme = ROOT / "README.md"
        readiness = ROOT / "docs" / "preflight-readiness.md"
        compare = ROOT / "docs" / "compare-mode.md"
        value = ROOT / "docs" / "value-benchmark.md"
        benchmarking = ROOT / "docs" / "benchmarking.md"
        release = ROOT / "docs" / "releases" / "v0.4.0.md"

        for path in (readiness, compare, value):
            self.assertTrue(path.exists(), path)

        self.assertIn("v0.4 GitHub release", readme.read_text())
        self.assertIn("before interpretive QC tools", readiness.read_text())
        self.assertIn("Index readiness", readiness.read_text())
        self.assertIn("fastaguard compare", compare.read_text())
        self.assertIn("fastaguard_compare_mqc.json", compare.read_text())
        self.assertIn("0.98 seconds", value.read_text())
        self.assertIn("50 MB", value.read_text())
        self.assertIn("v0.3 single-file baseline", benchmarking.read_text())
        self.assertIn(
            "GitHub release artifacts are built from the v0.4.0 tag",
            release.read_text(),
        )

    def test_v0_4_examples_mention_compare_as_starter_pattern(self):
        paths = [
            ROOT / "examples" / "nf-core" / "README.md",
            ROOT / "examples" / "snakemake" / "Snakefile",
            ROOT / "examples" / "nextflow" / "main.nf",
        ]

        for path in paths:
            with self.subTest(path=path):
                text = path.read_text()
                lower = text.lower()
                self.assertIn("fastaguard compare", text)
                self.assertIn("starter", lower)
                self.assertIn("local", lower)
                self.assertIn("gate.status", text)

    def test_v0_5_submission_readiness_docs_are_present(self):
        readme = self.read("README.md")
        roadmap = self.read("docs/roadmap.md")
        evidence = self.read("docs/evidence/fastaguard-v0.5-submission-readiness.md")
        release = self.read("docs/releases/v0.5.0.md")

        for text in [readme, roadmap, evidence, release]:
            self.assertIn("--gate submission", text)
            self.assertIn("--submission-target", text)
            self.assertIn("official validators", text)

        self.assertIn("FastaGuard does not replace NCBI, ENA, DDBJ", roadmap)
        self.assertIn("repository acceptance", evidence)
        self.assertIn("mkdir -p target/evidence/v0.5", evidence)
        self.assertTrue((ROOT / "testdata" / "submission_warnings.fa").exists())

    def test_multiqc_parser_reads_fastaguard_custom_content(self):
        fixture = ROOT / "examples" / "reports" / "assembly_pass" / "fastaguard_mqc.json"

        summary = load_custom_content_summary(fixture)

        self.assertEqual(set(summary), {"valid_assembly"})
        self.assertEqual(summary["valid_assembly"]["verdict"], "WARN")
        self.assertEqual(summary["valid_assembly"]["gate_status"], "WARN")
        self.assertEqual(summary["valid_assembly"]["readiness_status"], "WARN")
        self.assertEqual(summary["valid_assembly"]["readiness_blockers"], "")
        self.assertEqual(summary["valid_assembly"]["submission_target"], ".")
        self.assertEqual(summary["valid_assembly"]["submission_status"], "WARN")
        self.assertEqual(summary["valid_assembly"]["duplicate_first_token_id_count"], 0)
        self.assertEqual(summary["valid_assembly"]["sequence_count"], 3)
        self.assertEqual(summary["valid_assembly"]["n50"], 16)

    def test_multiqc_parser_reads_expanded_fields_from_cli_example(self):
        fixture = ROOT / "examples" / "reports" / "assembly_fail" / "fastaguard_mqc.json"

        summary = load_custom_content_summary(fixture)

        self.assertEqual(
            summary["problem_assembly"],
            {
                "verdict": "FAIL",
                "gate_mode": "none",
                "gate_status": "FAIL",
                "gate_blocking_findings": "duplicate_ids,duplicate_first_token_ids,invalid_chars",
                "readiness_status": "FAIL",
                "readiness_blockers": "index.duplicate_ids,submission.duplicate_ids,index.duplicate_first_token_ids,submission.duplicate_first_token_ids,alphabet.invalid_chars,submission.invalid_chars",
                "submission_target": ".",
                "submission_status": "FAIL",
                "unsafe_identifier_count": 0,
                "long_identifier_count": 0,
                "duplicate_first_token_id_count": 1,
                "gap_like_n_run_count": 0,
                "sequence_count": 5,
                "total_length": 145,
                "n50": 110,
                "n90": 8,
                "gc_percent": 8.28,
                "n_percent": 80.69,
                "finding_count": 9,
                "duplicate_id_count": 1,
                "invalid_sequence_count": 1,
                "high_n_sequence_count": 2,
                "tiny_contig_count": 5,
                "max_gap_run": 101,
                "gc_outlier_count": 0,
                "length_outlier_count": 1,
                "composite_anomaly_count": 1,
            },
        )

    def test_multiqc_parser_reads_expanded_summary_fields(self):
        with TemporaryDirectory() as temp_dir:
            fixture = Path(temp_dir) / "fastaguard_mqc.json"
            fixture.write_text(
                json.dumps(
                    {
                        "id": "fastaguard",
                        "section_name": "FastaGuard",
                        "description": "FASTA preflight QC summary",
                        "plot_type": "table",
                        "pconfig": {"id": "fastaguard_summary", "title": "FastaGuard"},
                        "data": {
                            "sample": {
                                "verdict": "WARN",
                                "sequence_count": 8,
                                "total_length": 2000,
                                "n50": 500,
                                "n90": 100,
                                "gc_percent": 50.0,
                                "n_percent": 2.5,
                                "duplicate_id_count": 1,
                                "invalid_sequence_count": 0,
                                "high_n_sequence_count": 2,
                                "tiny_contig_count": 1,
                                "max_gap_run": 120,
                                "gc_outlier_count": 1,
                                "length_outlier_count": 1,
                                "composite_anomaly_count": 1,
                                "finding_count": 4,
                                "readiness_status": "WARN",
                                "readiness_blockers": "assembly.high_n_rate",
                                "submission_target": "ncbi",
                                "submission_status": "WARN",
                                "unsafe_identifier_count": 1,
                                "long_identifier_count": 1,
                                "duplicate_first_token_id_count": 2,
                                "gap_like_n_run_count": 3,
                            }
                        },
                    }
                )
            )

            summary = load_custom_content_summary(fixture)
            self.assertEqual(
                summary["sample"],
                {
                    "verdict": "WARN",
                    "sequence_count": 8,
                    "total_length": 2000,
                    "n50": 500,
                    "n90": 100,
                    "gc_percent": 50.0,
                    "n_percent": 2.5,
                    "finding_count": 4,
                    "duplicate_id_count": 1,
                    "unsafe_identifier_count": 1,
                    "long_identifier_count": 1,
                    "duplicate_first_token_id_count": 2,
                    "gap_like_n_run_count": 3,
                    "invalid_sequence_count": 0,
                    "high_n_sequence_count": 2,
                    "tiny_contig_count": 1,
                    "max_gap_run": 120,
                    "gc_outlier_count": 1,
                    "length_outlier_count": 1,
                    "composite_anomaly_count": 1,
                    "readiness_status": "WARN",
                    "readiness_blockers": "assembly.high_n_rate",
                    "submission_target": "ncbi",
                    "submission_status": "WARN",
                },
            )

    def test_multiqc_parser_preserves_gate_and_readiness_fields(self):
        with TemporaryDirectory() as temp_dir:
            fixture = Path(temp_dir) / "fastaguard_mqc.json"
            fixture.write_text(
                json.dumps(
                    {
                        "id": "fastaguard",
                        "section_name": "FastaGuard",
                        "description": "FASTA preflight QC summary",
                        "plot_type": "table",
                        "pconfig": {"id": "fastaguard_summary", "title": "FastaGuard"},
                        "data": {
                            "sample": {
                                "verdict": "FAIL",
                                "sequence_count": 8,
                                "total_length": 2000,
                                "n50": 500,
                                "n90": 100,
                                "gc_percent": 50.0,
                                "n_percent": 2.5,
                                "finding_count": 4,
                                "gate_mode": "pipeline",
                                "gate_status": "FAIL",
                                "gate_blocking_findings": "duplicate_ids,high_n_rate",
                                "readiness_status": "FAIL",
                                "readiness_blockers": "index.duplicate_ids,assembly.high_n_rate",
                            }
                        },
                    }
                )
            )

            summary = load_custom_content_summary(fixture)
            self.assertEqual(summary["sample"]["gate_mode"], "pipeline")
            self.assertEqual(summary["sample"]["gate_status"], "FAIL")
            self.assertEqual(
                summary["sample"]["gate_blocking_findings"],
                "duplicate_ids,high_n_rate",
            )
            self.assertEqual(summary["sample"]["readiness_status"], "FAIL")
            self.assertEqual(
                summary["sample"]["readiness_blockers"],
                "index.duplicate_ids,assembly.high_n_rate",
            )

    def test_multiqc_parser_rejects_missing_required_summary_fields(self):
        with TemporaryDirectory() as temp_dir:
            fixture = Path(temp_dir) / "fastaguard_mqc.json"
            fixture.write_text(
                json.dumps(
                    {
                        "id": "fastaguard",
                        "plot_type": "table",
                        "data": {
                            "sample": {
                                "verdict": "WARN",
                                "sequence_count": 8,
                                "total_length": 2000,
                                "n90": 100,
                                "gc_percent": 50.0,
                                "n_percent": 2.5,
                                "finding_count": 4,
                            }
                        },
                    }
                )
            )

            with self.assertRaisesRegex(ValueError, "missing required"):
                load_custom_content_summary(fixture)

    def test_multiqc_parser_omits_absent_optional_summary_fields(self):
        with TemporaryDirectory() as temp_dir:
            fixture = Path(temp_dir) / "fastaguard_mqc.json"
            fixture.write_text(
                json.dumps(
                    {
                        "id": "fastaguard",
                        "plot_type": "table",
                        "data": {
                            "sample": {
                                "verdict": "PASS",
                                "sequence_count": 3,
                                "total_length": 48,
                                "n50": 16,
                                "n90": 16,
                                "gc_percent": 50.0,
                                "n_percent": 0.0,
                                "finding_count": 0,
                            }
                        },
                    }
                )
            )

            summary = load_custom_content_summary(fixture)
            self.assertEqual(
                summary["sample"],
                {
                    "verdict": "PASS",
                    "sequence_count": 3,
                    "total_length": 48,
                    "n50": 16,
                    "n90": 16,
                    "gc_percent": 50.0,
                    "n_percent": 0.0,
                    "finding_count": 0,
                },
            )

    def test_multiqc_parser_rejects_non_fastaguard_custom_content(self):
        with TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "other_mqc.json"
            path.write_text(
                json.dumps(
                    {
                        "id": "other_tool",
                        "plot_type": "table",
                        "data": {"sample": {"verdict": "PASS"}},
                    }
                )
            )

            with self.assertRaisesRegex(ValueError, "not a FastaGuard"):
                load_custom_content_summary(path)

    def test_multiqc_plugin_declares_module_entry_point(self):
        pyproject = (ROOT / "integrations" / "multiqc" / "pyproject.toml").read_text()

        self.assertIn('[project.entry-points."multiqc.modules.v1"]', pyproject)
        self.assertIn('fastaguard = "fastaguard_multiqc:MultiqcModule"', pyproject)
        self.assertIn("multiqc", pyproject)

    def test_multiqc_plugin_summary_headers_include_gate_fields(self):
        module_source = (
            ROOT
            / "integrations"
            / "multiqc"
            / "src"
            / "fastaguard_multiqc"
            / "multiqc_module.py"
        ).read_text()

        self.assertIn('"gate_mode"', module_source)
        self.assertIn('"gate_status"', module_source)
        self.assertIn('"gate_blocking_findings"', module_source)
        self.assertIn('"readiness_status"', module_source)
        self.assertIn('"readiness_blockers"', module_source)
        self.assertIn('"submission_target"', module_source)
        self.assertIn('"submission_status"', module_source)
        self.assertIn('"unsafe_identifier_count"', module_source)
        self.assertIn('"long_identifier_count"', module_source)
        self.assertIn('"duplicate_first_token_id_count"', module_source)
        self.assertIn('"gap_like_n_run_count"', module_source)
        self.assertIn("Finding IDs blocking the FastaGuard gate", module_source)
        self.assertIn("FastaGuard readiness status", module_source)
        self.assertIn("FASTA-level submission readiness status", module_source)

    def test_multiqc_docs_describe_submission_fields(self):
        readme = (ROOT / "integrations" / "multiqc" / "README.md").read_text()

        self.assertIn("submission readiness", readme)
        self.assertIn("`submission_target`", readme)
        self.assertIn("`submission_status`", readme)
        self.assertIn("duplicate first-token", readme)

    def test_multiqc_plugin_registers_filename_first_fastaguard_search_pattern(self):
        patterns = getattr(multiqc_parser, "FASTAGUARD_SEARCH_PATTERN", {})
        fastaguard_patterns = patterns.get("fastaguard", [])
        filenames = [pattern.get("fn") for pattern in fastaguard_patterns]
        pyproject = (ROOT / "integrations" / "multiqc" / "pyproject.toml").read_text()

        self.assertEqual(filenames, ["fastaguard_mqc.json", "*.fastaguard_mqc.json"])
        for pattern in fastaguard_patterns:
            self.assertEqual(set(pattern), {"fn"})
            self.assertFalse(pattern.get("shared", False))
            self.assertNotIn("contents", pattern)
            self.assertNotIn("contents_re", pattern)
            self.assertNotIn("num_lines", pattern)
        self.assertIn('[project.entry-points."multiqc.hooks.v1"]', pyproject)
        self.assertIn('before_config = "fastaguard_multiqc.parser:register_search_patterns"', pyproject)

    def test_multiqc_plugin_prepends_fastaguard_search_pattern(self):
        original_modules = {
            name: sys.modules.get(name)
            for name in (
                "multiqc",
                "multiqc.utils",
                "multiqc.utils.util_functions",
            )
        }
        fake_config = types.SimpleNamespace(
            sp={"custom_content": {"fn_re": r".+_mqc\.(yaml|yml|json)"}}
        )
        fake_multiqc = types.ModuleType("multiqc")
        fake_multiqc.config = fake_config
        fake_utils = types.ModuleType("multiqc.utils")
        fake_util_functions = types.ModuleType("multiqc.utils.util_functions")

        def update_dict(target, source, none_only=False, add_in_the_beginning=False):
            for key, src_val in source.items():
                if isinstance(src_val, list):
                    target[key] = src_val.copy()
                elif add_in_the_beginning:
                    target = {key: src_val, **target}
                else:
                    target[key] = src_val
            return target

        fake_util_functions.update_dict = update_dict
        sys.modules["multiqc"] = fake_multiqc
        sys.modules["multiqc.utils"] = fake_utils
        sys.modules["multiqc.utils.util_functions"] = fake_util_functions
        try:
            multiqc_parser.register_search_patterns()
        finally:
            for name, module in original_modules.items():
                if module is None:
                    sys.modules.pop(name, None)
                else:
                    sys.modules[name] = module

        self.assertEqual(next(iter(fake_config.sp)), "fastaguard")
        self.assertIn("custom_content", fake_config.sp)

    def test_bioconda_recipe_declares_binary_and_contract_tests(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        build = (ROOT / "packaging" / "bioconda" / "build.sh").read_text()

        self.assertIn('{% set name = "fastaguard" %}', recipe)
        self.assertIn("{{ compiler('rust') }}", recipe)
        self.assertIn("cargo-bundle-licenses", recipe)
        self.assertIn("fastaguard --help", recipe)
        self.assertIn("fastaguard --schema", recipe)
        self.assertIn("fastaguard --finding-catalog", recipe)
        self.assertIn("cargo install", build)
        self.assertIn("--no-track", build)

    def test_workflow_docs_reference_bioconda_and_container_status(self):
        nfcore_readme = (ROOT / "examples" / "nf-core" / "README.md").read_text()
        nfcore_module = (
            ROOT / "examples" / "nf-core" / "modules" / "local" / "fastaguard" / "main.nf"
        ).read_text()
        snakemake_readme = (
            ROOT / "examples" / "snakemake" / "wrapper" / "README.md"
        ).read_text()

        install = "mamba install -c conda-forge -c bioconda fastaguard"
        self.assertIn(install, nfcore_readme)
        self.assertIn(install, snakemake_readme)
        self.assertIn(
            "quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0",
            nfcore_readme,
        )
        self.assertNotIn("0.2.0--", nfcore_module)
        self.assertIn(
            "quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0",
            snakemake_readme,
        )

    def test_current_integration_assets_have_no_v0_5_runtime_pins(self):
        stale_runtime = re.compile(
            r"(?:fastaguard=0\.5\.0|fastaguard:0\.5\.0--|"
            r'\{% set version = "0\.5\.0" %\})'
        )
        paths = subprocess.check_output(
            ["git", "ls-files", "examples", "packaging"],
            cwd=ROOT,
            text=True,
        ).splitlines()
        violations = [
            path
            for path in paths
            if stale_runtime.search((ROOT / path).read_text(errors="ignore"))
        ]

        self.assertEqual(violations, [])

    def test_workflow_readiness_plan_defines_upstream_submission_path(self):
        readme = (ROOT / "README.md").read_text()
        adoption = (ROOT / "docs" / "adoption-plan.md").read_text()
        readiness = (ROOT / "docs" / "workflow-readiness.md").read_text()
        nfcore_readme = (ROOT / "examples" / "nf-core" / "README.md").read_text()
        nfcore_environment = (
            ROOT
            / "examples"
            / "nf-core"
            / "modules"
            / "local"
            / "fastaguard"
            / "environment.yml"
        ).read_text()
        snakemake_readme = (
            ROOT / "examples" / "snakemake" / "wrapper" / "README.md"
        ).read_text()
        snakemake_meta = (
            ROOT / "examples" / "snakemake" / "wrapper" / "meta.yaml"
        ).read_text()

        self.assertIn("[Workflow readiness](docs/workflow-readiness.md)", readme)
        self.assertIn("Phase 5: Upstream workflow readiness", adoption)
        self.assertIn(NFCORE_PR, adoption)
        self.assertIn(SNAKEMAKE_PR, adoption)
        self.assertIn(NFCORE_PR, readiness)
        self.assertIn(SNAKEMAKE_PR, readiness)
        self.assertNotIn("not yet an upstream nf-core module", readiness)
        self.assertNotIn("not yet an official Snakemake wrapper", readiness)
        self.assertIn("collect-then-gate", readiness)
        self.assertIn("nf-core modules lint", readiness)
        self.assertIn("nf-core modules test", readiness)
        self.assertIn("topic channels", readiness)
        self.assertIn("environment.linux-64.pin.txt", readiness)
        self.assertIn("test_wrappers.py", readiness)
        self.assertIn("--gate submission", readiness)
        self.assertIn("quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0", readiness)
        self.assertIn("docs/workflow-readiness.md", nfcore_readme)
        self.assertIn("docs/workflow-readiness.md", snakemake_readme)
        self.assertIn("fastaguard=0.6.0", nfcore_environment)
        self.assertIn("name: fastaguard", snakemake_meta)
        self.assertIn("description:", snakemake_meta)
        self.assertIn("output:", snakemake_meta)

    def test_nf_core_starter_has_upstream_prep_test_layout(self):
        module = ROOT / "examples" / "nf-core" / "modules" / "local" / "fastaguard"
        main_nf = (module / "main.nf").read_text()
        meta_yml = (module / "meta.yml").read_text()
        nf_test = (module / "tests" / "main.nf.test").read_text()

        self.assertIn("emit: versions_fastaguard, topic: versions", main_nf)
        self.assertIn("versions_fastaguard:", meta_yml)
        self.assertIn("topic", meta_yml)
        self.assertIn('process "FASTAGUARD"', nf_test)
        self.assertIn("file(workDir.resolve", nf_test)
        self.assertIn("assertAll", nf_test)
        self.assertIn("process.success", nf_test)
        self.assertIn('unstableKeys: ["html", "json", "tsv", "mqc"]', nf_test)
        for name in ("pass.fa", "warn.fa", "fail.fa", "invalid.fa"):
            self.assertFalse((module / "tests" / "data" / name).exists(), name)

    def test_nf_core_starter_matches_current_nf_core_module_shape(self):
        module = ROOT / "examples" / "nf-core" / "modules" / "local" / "fastaguard"
        main_nf = (module / "main.nf").read_text()
        meta_yml = (module / "meta.yml").read_text()
        nf_test = (module / "tests" / "main.nf.test").read_text()

        self.assertIn('conda "${moduleDir}/environment.yml"', main_nf)
        self.assertIn("workflow.containerEngine in ['singularity', 'apptainer']", main_nf)
        self.assertIn(
            "https://depot.galaxyproject.org/singularity/fastaguard:0.6.0--hfa8f182_0",
            main_nf,
        )
        self.assertIn(
            "quay.io/biocontainers/fastaguard:0.6.0--hfa8f182_0",
            main_nf,
        )
        self.assertIn("\n    when:\n    task.ext.when == null || task.ext.when\n", main_nf)
        self.assertIn("emit: versions_fastaguard, topic: versions", main_nf)
        self.assertNotIn("emit: exit_code", main_nf)
        self.assertIn('def prefix = task.ext.prefix ?: "${meta.id}"', main_nf)
        self.assertIn("def args = task.ext.args ?: ''", main_nf)
        self.assertNotIn("set +e", main_nf)
        self.assertIn("output:\n  html:", meta_yml)
        self.assertNotIn("output:\n  - html:", meta_yml)
        self.assertIn("versions_fastaguard:", meta_yml)
        self.assertIn('fastaguard --version | cut -d " " -f 2:', meta_yml)
        self.assertNotIn("exit_code:", meta_yml)
        self.assertIn('- "@ehsanestaji"', meta_yml)
        self.assertIn('tag "modules"', nf_test)
        self.assertIn('tag "modules_nfcore"', nf_test)
        self.assertIn('file(workDir.resolve("pass.fa").toString())', nf_test)
        self.assertIn(
            'snapshot(sanitizeOutput(process.out, unstableKeys: ["html", "json", "tsv", "mqc"])).match()',
            nf_test,
        )

    def test_nf_core_starter_has_complete_sanitized_snapshot_baseline(self):
        snapshot_path = (
            ROOT
            / "examples"
            / "nf-core"
            / "modules"
            / "local"
            / "fastaguard"
            / "tests"
            / "main.nf.test.snap"
        )
        snapshot = json.loads(snapshot_path.read_text())
        cases = {
            "pass FASTA emits all reports": "pass",
            "warn FASTA preserves reports": "warn",
            "fail FASTA preserves reports for gate review": "fail",
            "invalid FASTA is represented in the evidence path": "invalid",
        }

        self.assertEqual(set(snapshot), set(cases))
        for test_name, sample in cases.items():
            with self.subTest(test_name=test_name):
                self.assertEqual(len(snapshot[test_name]["content"]), 1)
                outputs = snapshot[test_name]["content"][0]
                self.assertEqual(
                    set(outputs),
                    {"html", "json", "mqc", "tsv", "versions_fastaguard"},
                )
                self.assertEqual(
                    outputs["html"],
                    [[{"id": sample}, f"{sample}.fastaguard.html"]],
                )
                self.assertEqual(
                    outputs["json"],
                    [[{"id": sample}, f"{sample}.fastaguard.json"]],
                )
                self.assertEqual(
                    outputs["tsv"],
                    [[{"id": sample}, f"{sample}.fastaguard.tsv"]],
                )
                self.assertEqual(
                    outputs["mqc"],
                    [[{"id": sample}, f"{sample}.fastaguard_mqc.json"]],
                )
                self.assertEqual(
                    outputs["versions_fastaguard"],
                    [["FASTAGUARD", "fastaguard", "0.6.0"]],
                )

    def test_snakemake_wrapper_has_upstream_prep_test_layout(self):
        wrapper = ROOT / "examples" / "snakemake" / "wrapper"
        readme = (wrapper / "README.md").read_text()
        wrapper_py = (wrapper / "wrapper.py").read_text()
        starter_snakefile = (wrapper / "Snakefile").read_text()
        test_snakefile = (wrapper / "test" / "Snakefile").read_text()
        test_py = (wrapper / "test" / "test_wrappers.py").read_text()
        pin = (wrapper / "environment.linux-64.pin.txt").read_text()

        self.assertIn("safe local order", readme)
        self.assertIn("fastaguard", pin)
        self.assertIn("fastaguard {snakemake.input.fasta}", wrapper_py)
        self.assertNotIn("snakemake.output.exit_code", wrapper_py)
        self.assertNotIn("set +e", wrapper_py)
        self.assertIn('extra = snakemake.params.get("extra", "")', wrapper_py)
        self.assertIn('"{extra} "', wrapper_py)
        self.assertIn('"master/bio/fastaguard"', test_snakefile)
        self.assertNotIn('"file:../wrapper/fastaguard"', test_snakefile)
        self.assertNotIn("exit_code", test_snakefile)
        self.assertIn("rule fastaguard_pass", test_snakefile)
        self.assertIn("rule fastaguard_warn", test_snakefile)
        self.assertIn("rule fastaguard_fail", test_snakefile)
        self.assertIn("rule fastaguard_invalid", test_snakefile)
        self.assertIn("pytest", test_py)
        self.assertIn("snakemake", test_py)
        for target in (
            "pass/fastaguard.json",
            "warn/fastaguard.json",
            "fail/fastaguard.json",
            "invalid/fastaguard.json",
        ):
            with self.subTest(target=target):
                self.assertIn(f'"{target}"', test_py)
        self.assertNotIn("fastaguard.exit_code", test_py)
        self.assertIn('extra="--profile assembly --gate pipeline"', test_snakefile)
        self.assertFalse((wrapper / "wrapper" / "fastaguard" / "wrapper.py").exists())

        output_block = starter_snakefile.split("    output:\n", 1)[1].split(
            "    log:\n", 1
        )[0]
        outputs = dict(
            re.findall(r'^\s+(\w+)="([^"]+)",$', output_block, flags=re.MULTILINE)
        )
        self.assertEqual(
            outputs,
            {
                "html": "fastaguard_report.html",
                "json": "fastaguard.json",
                "tsv": "fastaguard.tsv",
                "multiqc": "fastaguard_mqc.json",
            },
        )
        params_block = starter_snakefile.split("    params:\n", 1)[1].split(
            "    wrapper:\n", 1
        )[0]
        self.assertEqual(
            params_block.strip(),
            'extra="--profile assembly --gate pipeline",',
        )
        self.assertNotIn("exit_code", starter_snakefile)

    def test_workflow_readiness_safe_order_is_explicit(self):
        readiness = (ROOT / "docs" / "workflow-readiness.md").read_text()
        adoption = (ROOT / "docs" / "adoption-plan.md").read_text()

        self.assertIn("Safe Order", readiness)
        self.assertIn("local repository tests first", readiness)
        self.assertIn("check_fastaguard_gate.py", readiness)
        self.assertIn(NFCORE_PR, readiness)
        self.assertIn(SNAKEMAKE_PR, readiness)
        self.assertIn("merged 2026-08-21", readiness)
        self.assertIn("2026-07-27", readiness)

    def test_workflow_docs_record_merged_upstream_integrations(self):
        adoption = (ROOT / "docs" / "adoption-plan.md").read_text()
        readiness = (ROOT / "docs" / "workflow-readiness.md").read_text()

        for text in (adoption, readiness):
            self.assertIn(NFCORE_PR, text)
            self.assertIn(SNAKEMAKE_PR, text)
            self.assertNotIn("prepare external PR branches", text)
            self.assertNotIn("Before an upstream nf-core module submission", text)
            self.assertNotIn("Before an official Snakemake wrapper submission", text)

    def test_current_workflow_assets_do_not_claim_a_default_gate(self):
        readme = (ROOT / "README.md").read_text()
        nfcore_module = self.read(
            "examples/nf-core/modules/local/fastaguard/main.nf"
        )
        snakemake_wrapper = self.read("examples/snakemake/wrapper/wrapper.py")

        self.assertNotIn("default workflow gate policy", readme)
        self.assertNotIn("--gate", nfcore_module)
        self.assertNotIn("--gate", snakemake_wrapper)

    def test_benchmarking_docs_include_v0_2_evidence_topics(self):
        text = (ROOT / "docs" / "benchmarking.md").read_text()

        self.assertIn("## Evidence Targets", text)
        self.assertIn("docs/evidence/fastaguard-v0.3-evidence.md", text)
        self.assertIn("duplicate IDs", text)
        self.assertIn("invalid characters", text)
        self.assertIn("high-N", text)
        self.assertIn("GC outliers", text)
        self.assertIn("QUAST", text)
        self.assertIn("BUSCO", text)
        self.assertIn("BlobToolKit", text)

    def test_public_evidence_manifest_declares_default_assemblies(self):
        manifest = json.loads(
            (ROOT / "docs" / "evidence" / "public_assemblies.json").read_text()
        )

        self.assertEqual(manifest["schema_version"], 1)
        cases = manifest["assemblies"]
        self.assertGreaterEqual(len(cases), 2)
        accessions = {case["accession"] for case in cases}
        self.assertIn("GCF_000005845.2", accessions)
        self.assertIn("GCF_000182925.2", accessions)

        for case in cases:
            with self.subTest(case=case):
                self.assertEqual(
                    set(case),
                    {
                        "id",
                        "accession",
                        "label",
                        "category",
                        "source_url",
                        "evidence_role",
                        "expected_scale",
                        "downstream_route",
                    },
                )
                self.assertRegex(case["id"], r"^[a-z0-9][a-z0-9_-]+$")
                self.assertRegex(case["accession"], r"^GC[AF]_[0-9]+\.[0-9]+$")
                self.assertTrue(case["label"])
                self.assertIn(case["category"], {"bacterial", "fungal"})
                self.assertTrue(case["source_url"].startswith("https://"))
                self.assertTrue(case["evidence_role"])
                self.assertTrue(case["expected_scale"])
                self.assertRegex(
                    case["downstream_route"], r"(QUAST|BUSCO|BlobToolKit|validator)"
                )

    def test_v0_5_public_evidence_doc_defines_benchmark_table(self):
        evidence = ROOT / "docs" / "evidence" / "fastaguard-v0.5-public-evidence.md"

        self.assertTrue(evidence.exists())
        text = evidence.read_text()
        self.assertIn("docs/evidence/public_assemblies.json", text)
        self.assertIn("python3 scripts/collect_evidence.py", text)
        self.assertIn("evidence_summary.tsv", text)
        self.assertIn("downstream_route", text)
        self.assertIn("not biological completeness", text)
        self.assertIn("not contamination confirmation", text)

    def test_evidence_docs_reference_local_and_public_workflows(self):
        evidence = (ROOT / "docs" / "evidence" / "fastaguard-v0.2-evidence.md")
        benchmarking = ROOT / "docs" / "benchmarking.md"
        readme = ROOT / "README.md"
        landscape = ROOT / "docs" / "tool-landscape.md"

        evidence_text = evidence.read_text()
        self.assertIn("python3 scripts/collect_evidence.py", evidence_text)
        self.assertIn("--local-only", evidence_text)
        self.assertIn("datasets download genome accession", evidence_text)
        self.assertIn("evidence_summary.json", evidence_text)
        self.assertIn("not biological completeness", evidence_text)
        self.assertIn("not contamination confirmation", evidence_text)

        for path in (benchmarking, readme, landscape):
            with self.subTest(path=path):
                self.assertIn(
                    "docs/evidence/fastaguard-v0.2-evidence.md", path.read_text()
                )

    def test_v0_3_evidence_docs_reference_gate_and_checksum(self):
        evidence = ROOT / "docs" / "evidence" / "fastaguard-v0.3-evidence.md"

        self.assertTrue(evidence.exists())
        evidence_text = evidence.read_text()
        self.assertIn("--gate pipeline", evidence_text)
        self.assertIn("input_sha256", evidence_text)
        self.assertIn("not biological completeness", evidence_text)
        self.assertIn("not contamination confirmation", evidence_text)
        self.assertIn("python3 scripts/collect_evidence.py", evidence_text)

    def test_collect_evidence_local_only_smoke_does_not_require_network(self):
        with TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "fake_fastaguard.py"
            fake_binary.write_text(
                """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

args = sys.argv[1:]
if args == ["--version"]:
    print("fastaguard 9.8.7 test-build")
    raise SystemExit(0)
input_path = Path(args[0])
if "--gate" not in args or args[args.index("--gate") + 1] != "pipeline":
    raise SystemExit("unexpected gate mode")

def option_path(flag):
    try:
        return Path(args[args.index(flag) + 1])
    except ValueError:
        return None

json_path = option_path("--json")
html_path = option_path("--out")
tsv_path = option_path("--tsv")
multiqc_path = option_path("--multiqc")
summary = {
    "sequence_count": 1,
    "total_length": input_path.stat().st_size,
    "n50": input_path.stat().st_size,
    "n90": input_path.stat().st_size,
}
report = {
    "tool": {"name": "fastaguard", "version": "test"},
    "verdict": {"status": "PASS"},
    "gate": {
        "mode": "pipeline",
        "status": "PASS",
        "blocking_findings": [],
    },
    "provenance": {"input_sha256": "0" * 64},
    "summary": summary,
    "findings": [],
}
json_path.write_text(json.dumps(report))
html_path.write_text("<html>fake</html>")
tsv_path.write_text("metric\\tvalue\\n")
multiqc_path.write_text(json.dumps({"id": "fastaguard", "data": {}}))
"""
            )
            fake_binary.chmod(fake_binary.stat().st_mode | 0o111)
            out_dir = temp_path / "evidence"

            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "scripts" / "collect_evidence.py"),
                    "--binary",
                    str(fake_binary),
                    "--out-dir",
                    str(out_dir),
                    "--local-only",
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertNotIn("datasets download", completed.stdout)
            summary_path = out_dir / "evidence_summary.json"
            self.assertTrue(summary_path.exists())
            summary = json.loads(summary_path.read_text())
            expected_git_commit = subprocess.run(
                ["git", "rev-parse", "--short", "HEAD"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            self.assertEqual(
                summary["fastaguard_version"], "fastaguard 9.8.7 test-build"
            )
            self.assertEqual(summary["git_commit"], expected_git_commit)
            self.assertNotIn("source_commit", summary)
            case_ids = {case["id"] for case in summary["cases"]}
            self.assertEqual(
                case_ids,
                {"synthetic_valid", "problem_fixture", "gzipped_valid"},
            )
            self.assertTrue((out_dir / "evidence_summary.tsv").exists())
            tsv_text = (out_dir / "evidence_summary.tsv").read_text()
            self.assertIn("evidence_role", tsv_text.splitlines()[0])
            self.assertIn("downstream_route", tsv_text.splitlines()[0])
            for case in summary["cases"]:
                self.assertEqual(case["verdict"], "PASS")
                self.assertEqual(case["gate_mode"], "pipeline")
                self.assertEqual(case["gate_status"], "PASS")
                self.assertEqual(case["gate_blocking_findings"], "")
                self.assertEqual(case["input_sha256"], "0" * 64)
                self.assertIn("evidence_role", case)
                self.assertIn("expected_scale", case)
                self.assertIn("downstream_route", case)
                self.assertIn("--gate pipeline", case["command"])
                self.assertGreater(case["elapsed_seconds"], 0)
                self.assertIn("command", case)

    def test_collect_evidence_rejects_reports_without_gate_contract(self):
        with TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "old_fastaguard.py"
            fake_binary.write_text(
                """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

args = sys.argv[1:]
input_path = Path(args[0])

def option_path(flag):
    try:
        return Path(args[args.index(flag) + 1])
    except ValueError:
        return None

json_path = option_path("--json")
html_path = option_path("--out")
tsv_path = option_path("--tsv")
multiqc_path = option_path("--multiqc")
summary = {
    "sequence_count": 1,
    "total_length": input_path.stat().st_size,
    "n50": input_path.stat().st_size,
    "n90": input_path.stat().st_size,
}
report = {
    "tool": {"name": "fastaguard", "version": "old"},
    "verdict": {"status": "PASS"},
    "summary": summary,
    "findings": [],
}
json_path.write_text(json.dumps(report))
html_path.write_text("<html>fake</html>")
tsv_path.write_text("metric\\tvalue\\n")
multiqc_path.write_text(json.dumps({"id": "fastaguard", "data": {}}))
"""
            )
            fake_binary.chmod(fake_binary.stat().st_mode | 0o111)

            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "scripts" / "collect_evidence.py"),
                    "--binary",
                    str(fake_binary),
                    "--out-dir",
                    str(temp_path / "evidence"),
                    "--local-only",
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("missing gate", completed.stderr)

    def test_collect_evidence_rejects_non_pipeline_gate_reports(self):
        with TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            fake_binary = temp_path / "wrong_gate_fastaguard.py"
            fake_binary.write_text(
                """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

args = sys.argv[1:]
input_path = Path(args[0])

def option_path(flag):
    try:
        return Path(args[args.index(flag) + 1])
    except ValueError:
        return None

json_path = option_path("--json")
html_path = option_path("--out")
tsv_path = option_path("--tsv")
multiqc_path = option_path("--multiqc")
summary = {
    "sequence_count": 1,
    "total_length": input_path.stat().st_size,
    "n50": input_path.stat().st_size,
    "n90": input_path.stat().st_size,
}
report = {
    "tool": {"name": "fastaguard", "version": "test"},
    "verdict": {"status": "PASS"},
    "gate": {
        "mode": "none",
        "status": "PASS",
        "blocking_findings": [],
    },
    "provenance": {"input_sha256": "0" * 64},
    "summary": summary,
    "findings": [],
}
json_path.write_text(json.dumps(report))
html_path.write_text("<html>fake</html>")
tsv_path.write_text("metric\\tvalue\\n")
multiqc_path.write_text(json.dumps({"id": "fastaguard", "data": {}}))
"""
            )
            fake_binary.chmod(fake_binary.stat().st_mode | 0o111)

            completed = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "scripts" / "collect_evidence.py"),
                    "--binary",
                    str(fake_binary),
                    "--out-dir",
                    str(temp_path / "evidence"),
                    "--local-only",
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("expected gate.mode pipeline", completed.stderr)

    def test_deep_release_vision_is_documented_in_project_docs(self):
        vision = (ROOT / "docs" / "vision-plan.md").read_text()
        readme = (ROOT / "README.md").read_text()

        required_phrases = [
            "FASTA preflight operating system",
            "evidence before expansion",
            "assembly gate",
            "compare mode",
            "transcriptome",
            "protein",
            "reference-panel",
            "MCP",
            "machine-actionable",
            "local-metrics-only",
        ]

        for phrase in required_phrases:
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, vision)

        self.assertIn("docs/vision-plan.md", readme)

    def test_snakemake_wrapper_declares_bioconda_environment(self):
        environment = (
            ROOT / "examples" / "snakemake" / "wrapper" / "environment.yaml"
        )
        snakefile = (ROOT / "examples" / "snakemake" / "wrapper" / "Snakefile")

        self.assertTrue(environment.exists())
        text = environment.read_text()
        self.assertEqual(
            text.splitlines(),
            [
                "channels:",
                "  - conda-forge",
                "  - bioconda",
                "  - nodefaults",
                "dependencies:",
                "  - fastaguard=0.6.0",
            ],
        )
        self.assertIn("conda:\n        \"environment.yaml\"", snakefile.read_text())


if __name__ == "__main__":
    unittest.main()
