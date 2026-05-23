import json
import sys
import types
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "integrations" / "multiqc" / "src"))

import fastaguard_multiqc.parser as multiqc_parser
from fastaguard_multiqc.parser import load_custom_content_summary


class AdoptionAssetsTest(unittest.TestCase):
    def test_multiqc_parser_reads_fastaguard_custom_content(self):
        fixture = ROOT / "examples" / "reports" / "assembly_pass" / "fastaguard_mqc.json"

        summary = load_custom_content_summary(fixture)

        self.assertEqual(set(summary), {"valid_assembly"})
        self.assertEqual(summary["valid_assembly"]["verdict"], "PASS")
        self.assertEqual(summary["valid_assembly"]["sequence_count"], 3)
        self.assertEqual(summary["valid_assembly"]["n50"], 16)

    def test_multiqc_parser_reads_expanded_fields_from_cli_example(self):
        fixture = ROOT / "examples" / "reports" / "assembly_fail" / "fastaguard_mqc.json"

        summary = load_custom_content_summary(fixture)

        self.assertEqual(
            summary["problem_assembly"],
            {
                "verdict": "FAIL",
                "sequence_count": 5,
                "total_length": 145,
                "n50": 110,
                "n90": 8,
                "gc_percent": 8.28,
                "n_percent": 80.69,
                "finding_count": 7,
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
                    "invalid_sequence_count": 0,
                    "high_n_sequence_count": 2,
                    "tiny_contig_count": 1,
                    "max_gap_run": 120,
                    "gc_outlier_count": 1,
                    "length_outlier_count": 1,
                    "composite_anomaly_count": 1,
                },
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
        snakemake_readme = (
            ROOT / "examples" / "snakemake" / "wrapper" / "README.md"
        ).read_text()

        install = "mamba install -c conda-forge -c bioconda fastaguard"
        self.assertIn(install, nfcore_readme)
        self.assertIn(install, snakemake_readme)
        self.assertIn(
            "Once a BioContainers image is confirmed, the module can add a pinned container directive.",
            nfcore_readme,
        )

    def test_benchmarking_docs_include_v0_2_evidence_topics(self):
        text = (ROOT / "docs" / "benchmarking.md").read_text()

        self.assertIn("## v0.2 Evidence Targets", text)
        self.assertIn("duplicate IDs", text)
        self.assertIn("invalid characters", text)
        self.assertIn("high-N", text)
        self.assertIn("GC outliers", text)
        self.assertIn("QUAST", text)
        self.assertIn("BUSCO", text)
        self.assertIn("BlobToolKit", text)

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
                "dependencies:",
                "  - fastaguard=0.1.1",
            ],
        )
        self.assertIn('conda: "environment.yaml"', snakefile.read_text())


if __name__ == "__main__":
    unittest.main()
