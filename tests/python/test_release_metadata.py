import json
import os
import re
import subprocess
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class ReleaseMetadataTest(unittest.TestCase):
    def test_package_and_bioconda_recipe_target_v0_2_0(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertEqual(cargo["package"]["version"], "0.2.0")
        self.assertIn('{% set version = "0.2.0" %}', recipe)

    def test_v0_2_0_release_notes_exist(self):
        notes = ROOT / "docs" / "releases" / "v0.2.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.2.0", text)
        self.assertIn("Assembly Trust", text)
        self.assertIn("Pipeline Adoption", text)
        self.assertIn("Once v0.2.0 is published on Bioconda", text)
        self.assertIn(
            "Bioconda currently serves the latest published package, v0.1.1",
            text,
        )
        self.assertIn("staged", text)

    def test_bioconda_recipe_is_staged_for_v0_2_0_until_archive_sha_is_known(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        marker = "REPLACE" + "_WITH_"
        placeholder = marker + "V0_2_0_SOURCE_ARCHIVE_SHA256"

        self.assertEqual(cargo["package"]["version"], "0.2.0")
        self.assertTrue((ROOT / "docs" / "releases" / "v0.2.0.md").exists())
        self.assertIn(
            "# Update sha256 after the v0.2.0 GitHub source archive is published.",
            recipe,
        )
        self.assertIn(f"sha256: {placeholder}", recipe)
        self.assertIn(marker, recipe)

    def test_release_ready_bioconda_recipe_requires_real_sha(self):
        if os.environ.get("FASTAGUARD_RELEASE_READY") != "1":
            self.skipTest("set FASTAGUARD_RELEASE_READY=1 to require publishable metadata")

        tracked_paths = subprocess.check_output(
            ["git", "ls-files"],
            cwd=ROOT,
            text=True,
        ).splitlines()
        marker = "REPLACE" + "_WITH_"
        placeholders = [
            path
            for path in tracked_paths
            if marker in (ROOT / path).read_text(errors="ignore")
        ]
        self.assertEqual(placeholders, [])

        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        match = re.search(r"sha256: ([a-f0-9]{64})", recipe)
        self.assertIsNotNone(match, recipe)
        self.assertNotIn(marker + "PUBLIC_SOURCE_ARCHIVE_SHA256", recipe)

    def test_committed_example_reports_match_cargo_package_version(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        package_version = cargo["package"]["version"]
        examples = [
            ROOT / "examples" / "reports" / "assembly_pass" / "fastaguard.json",
            ROOT / "examples" / "reports" / "assembly_fail" / "fastaguard.json",
        ]

        for path in examples:
            with self.subTest(path=path):
                report = json.loads(path.read_text())
                self.assertEqual(report["tool"]["version"], package_version)

    def test_bioconda_recipe_avoids_unneeded_runtime_zlib(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertNotIn("    - zlib", recipe)

    def test_bioconda_recipe_includes_required_lint_metadata(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertIn("run_exports:", recipe)
        self.assertIn('{{ pin_subpackage(\'fastaguard\', max_pin="x.x") }}', recipe)
        self.assertIn("{{ stdlib('c') }}", recipe)

    def test_bioconda_build_script_uses_portable_install(self):
        script = (ROOT / "packaging" / "bioconda" / "build.sh").read_text()

        self.assertIn('mkdir -p "${PREFIX}/share/${PKG_NAME}/schema"', script)
        self.assertNotIn("install -D", script)

    def test_docs_reference_published_bioconda_install(self):
        install_command = "mamba install -c conda-forge -c bioconda fastaguard"
        docs = [
            ROOT / "README.md",
            ROOT / "docs" / "packaging.md",
            ROOT / "docs" / "adoption-plan.md",
            ROOT / "packaging" / "bioconda" / "README.md",
        ]

        for path in docs:
            with self.subTest(path=path):
                text = path.read_text()
                self.assertIn(install_command, text)

        packaging = (ROOT / "docs" / "packaging.md").read_text()
        self.assertNotIn("GitHub repository is private", packaging)
        self.assertNotIn("placeholder SHA256", packaging)


if __name__ == "__main__":
    unittest.main()
