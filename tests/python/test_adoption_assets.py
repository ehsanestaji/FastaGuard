import json
import subprocess
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
                    {"id", "accession", "label", "category", "source_url"},
                )
                self.assertRegex(case["id"], r"^[a-z0-9][a-z0-9_-]+$")
                self.assertRegex(case["accession"], r"^GC[AF]_[0-9]+\.[0-9]+$")
                self.assertTrue(case["label"])
                self.assertIn(case["category"], {"bacterial", "fungal"})
                self.assertTrue(case["source_url"].startswith("https://"))

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
            case_ids = {case["id"] for case in summary["cases"]}
            self.assertEqual(
                case_ids,
                {"synthetic_valid", "problem_fixture", "gzipped_valid"},
            )
            self.assertTrue((out_dir / "evidence_summary.tsv").exists())
            for case in summary["cases"]:
                self.assertEqual(case["verdict"], "PASS")
                self.assertGreater(case["elapsed_seconds"], 0)
                self.assertIn("command", case)

    def test_deep_release_vision_is_documented_and_memorized(self):
        vision = (ROOT / "docs" / "vision-plan.md").read_text()
        memory = (ROOT / "AGENTS.md").read_text()
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

        self.assertIn("Deep Release Vision", memory)
        self.assertIn("FASTA preflight operating system", memory)
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
                "dependencies:",
                "  - fastaguard=0.1.1",
            ],
        )
        self.assertIn('conda: "environment.yaml"', snakefile.read_text())


if __name__ == "__main__":
    unittest.main()
