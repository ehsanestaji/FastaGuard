import re
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

    def test_bioconda_recipe_sha256_is_real_hash(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        placeholder = "REPLACE_WITH_V0_2_0_SOURCE_ARCHIVE_SHA256"

        if cargo["package"]["version"] == "0.2.0" and placeholder in recipe:
            self.assertTrue((ROOT / "docs" / "releases" / "v0.2.0.md").exists())
            self.assertIn(
                "# Update sha256 after the v0.2.0 GitHub source archive is published.",
                recipe,
            )
            self.assertIn(f"sha256: {placeholder}", recipe)
            return

        match = re.search(r"sha256: ([a-f0-9]{64})", recipe)
        self.assertIsNotNone(match, recipe)
        self.assertNotIn("REPLACE_WITH_PUBLIC_SOURCE_ARCHIVE_SHA256", recipe)
        self.assertNotIn(placeholder, recipe)

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
