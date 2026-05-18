import re
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class ReleaseMetadataTest(unittest.TestCase):
    def test_package_and_bioconda_recipe_target_v0_1_1(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertEqual(cargo["package"]["version"], "0.1.1")
        self.assertIn('{% set version = "0.1.1" %}', recipe)

    def test_v0_1_1_release_notes_exist(self):
        notes = ROOT / "docs" / "releases" / "v0.1.1.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.1.1", text)
        self.assertIn("packaging metadata", text)

    def test_bioconda_recipe_sha256_is_real_hash(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        match = re.search(r"sha256: ([a-f0-9]{64})", recipe)
        self.assertIsNotNone(match, recipe)
        self.assertNotIn("REPLACE_WITH_PUBLIC_SOURCE_ARCHIVE_SHA256", recipe)

    def test_bioconda_recipe_avoids_unneeded_runtime_zlib(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertNotIn("    - zlib", recipe)

    def test_bioconda_build_script_uses_portable_install(self):
        script = (ROOT / "packaging" / "bioconda" / "build.sh").read_text()

        self.assertIn('mkdir -p "${PREFIX}/share/${PKG_NAME}/schema"', script)
        self.assertNotIn("install -D", script)


if __name__ == "__main__":
    unittest.main()
