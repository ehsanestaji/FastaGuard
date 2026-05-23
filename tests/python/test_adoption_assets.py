import json
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "integrations" / "multiqc" / "src"))

from fastaguard_multiqc.parser import load_custom_content_summary


class AdoptionAssetsTest(unittest.TestCase):
    def test_multiqc_parser_reads_fastaguard_custom_content(self):
        fixture = ROOT / "examples" / "reports" / "assembly_pass" / "fastaguard_mqc.json"

        summary = load_custom_content_summary(fixture)

        self.assertEqual(set(summary), {"valid_assembly"})
        self.assertEqual(summary["valid_assembly"]["verdict"], "PASS")
        self.assertEqual(summary["valid_assembly"]["sequence_count"], 3)
        self.assertEqual(summary["valid_assembly"]["n50"], 16)

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
            row = summary["sample"]

            for field in (
                "verdict",
                "sequence_count",
                "total_length",
                "n50",
                "n90",
                "gc_percent",
                "n_percent",
                "duplicate_id_count",
                "invalid_sequence_count",
                "high_n_sequence_count",
                "tiny_contig_count",
                "max_gap_run",
                "gc_outlier_count",
                "length_outlier_count",
                "composite_anomaly_count",
                "finding_count",
            ):
                self.assertIn(field, row)

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
