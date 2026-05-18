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


if __name__ == "__main__":
    unittest.main()
