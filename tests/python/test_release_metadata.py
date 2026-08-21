import json
import re
import subprocess
import tarfile
import tomllib
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
CURRENT_VERSION = "0.6.0"
CURRENT_CONTAINER = "0.6.0--hfa8f182_0"
CURRENT_SOURCE_SHA256 = (
    "b57e967ccaa03ef5e70c14b96247a644750d8bd492360633988f2f53fe84184b"
)


class ReleaseMetadataTest(unittest.TestCase):
    def test_package_targets_v0_7_0(self):
        cargo = tomllib.loads((ROOT / "Cargo.toml").read_text())

        self.assertEqual(cargo["package"]["version"], "0.7.0")

    def test_v0_7_0_release_notes_define_operational_trust_contract(self):
        notes = ROOT / "docs" / "releases" / "v0.7.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        for expected in [
            "FastaGuard v0.7.0",
            "Operational Trust",
            "ncbi_genome",
            "FASTA preflight only",
            "machine_summary.safe_for_downstream",
            "gate.can_continue",
            "--outdir",
            "--force",
        ]:
            with self.subTest(expected=expected):
                self.assertIn(expected, text)

    def test_release_archive_contains_runtime_and_contract_assets(self):
        with TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            binary = temp_path / "fastaguard"
            binary.write_text("#!/usr/bin/env sh\nexit 0\n")
            binary.chmod(0o755)
            dist = temp_path / "dist"

            completed = subprocess.run(
                [
                    str(ROOT / "scripts" / "package_release_artifact.sh"),
                    "test-target",
                    "0.7.0",
                    str(binary),
                    str(dist),
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(completed.returncode, 0, completed.stderr)
            archive = Path(completed.stdout.strip())
            if not archive.is_absolute():
                archive = ROOT / archive
            self.assertTrue(archive.exists(), archive)

            with tarfile.open(archive, "r:gz") as package:
                names = set(package.getnames())

            top_levels = {name.split("/", 1)[0] for name in names}
            self.assertEqual(top_levels, {"fastaguard-0.7.0-test-target"})
            root = "fastaguard-0.7.0-test-target"
            for expected in [
                f"{root}/fastaguard",
                f"{root}/README.md",
                f"{root}/LICENSE",
                f"{root}/schema/fastaguard.schema.json",
                f"{root}/schema/finding-catalog.json",
            ]:
                with self.subTest(expected=expected):
                    self.assertIn(expected, names)

    def test_bioconda_recipe_tracks_published_v0_6_0_archive(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()

        self.assertIn('{% set version = "0.6.0" %}', recipe)
        self.assertIn("fastaguard --version | grep {{ version }}", recipe)

    def test_v0_2_0_release_notes_exist(self):
        notes = ROOT / "docs" / "releases" / "v0.2.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.2.0", text)
        self.assertIn("Assembly Trust", text)
        self.assertIn("Pipeline Adoption", text)
        self.assertIn("Install the v0.2.0 Bioconda package", text)
        self.assertIn("v0.2.0 GitHub release binaries and source archive", text)
        self.assertIn("quay.io/biocontainers/fastaguard:0.2.0--hfa8f182_0", text)

    def test_v0_3_0_release_notes_exist(self):
        notes = ROOT / "docs" / "releases" / "v0.3.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.3.0", text)
        self.assertIn("Evidence And Assembly Gate", text)
        self.assertIn("--gate pipeline", text)
        self.assertIn("input_sha256", text)

    def test_v0_5_0_release_notes_exist(self):
        notes = ROOT / "docs" / "releases" / "v0.5.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.5.0", text)
        self.assertIn("Submission Readiness Gate", text)
        self.assertIn("--gate submission", text)
        self.assertIn("--submission-target generic|ncbi", text)

    def test_v0_6_0_release_notes_define_conventional_exit_contract(self):
        notes = ROOT / "docs" / "releases" / "v0.6.0.md"

        self.assertTrue(notes.exists())
        text = notes.read_text()
        self.assertIn("FastaGuard v0.6.0", text)
        self.assertIn("Successful report generation exits with code `0`", text)
        self.assertIn("`input_path`", text)
        self.assertIn("`gate.status`", text)

    def test_v0_6_exit_contract_docs_include_output_write_failures(self):
        contract_docs = [
            ROOT / "README.md",
            ROOT / "docs" / "mvp-spec.md",
            ROOT / "docs" / "output-contract.md",
            ROOT / "docs" / "releases" / "v0.6.0.md",
            ROOT / "docs" / "roadmap.md",
            ROOT / "docs" / "vision-plan.md",
        ]

        for path in contract_docs:
            with self.subTest(path=path):
                text = " ".join(path.read_text().split())
                starts = [
                    match.start()
                    for match in re.finditer("configuration, input-access", text)
                ]
                self.assertTrue(starts, path)
                for start in starts:
                    description = text[start : start + 120]
                    self.assertIn("output-write", description, description)

    def test_bioconda_recipe_has_publishable_v0_6_0_source_sha(self):
        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        marker = "REPLACE" + "_WITH_"

        self.assertTrue((ROOT / "docs" / "releases" / "v0.6.0.md").exists())
        self.assertIn('{% set version = "0.6.0" %}', recipe)
        self.assertNotIn(marker, recipe)

        match = re.search(r"sha256: ([a-f0-9]{64})", recipe)
        self.assertIsNotNone(match, recipe)
        self.assertEqual(
            match.group(1),
            CURRENT_SOURCE_SHA256,
        )

    def test_release_ready_bioconda_recipe_requires_real_sha(self):
        tracked_paths = subprocess.check_output(
            ["git", "ls-files"],
            cwd=ROOT,
            text=True,
        ).splitlines()
        marker = "REPLACE" + "_WITH_"
        placeholders = [
            path
            for path in tracked_paths
            if (ROOT / path).exists()
            and marker in (ROOT / path).read_text(errors="ignore")
        ]
        self.assertEqual(placeholders, [])

        recipe = (ROOT / "packaging" / "bioconda" / "meta.yaml").read_text()
        match = re.search(r"sha256: ([a-f0-9]{64})", recipe)
        self.assertIsNotNone(match, recipe)
        self.assertEqual(
            match.group(1),
            CURRENT_SOURCE_SHA256,
        )
        self.assertNotIn(marker + "PUBLIC_SOURCE_ARCHIVE_SHA256", recipe)

    def test_committed_example_reports_match_current_published_version(self):
        examples = [
            ROOT / "examples" / "reports" / "assembly_pass" / "fastaguard.json",
            ROOT / "examples" / "reports" / "assembly_fail" / "fastaguard.json",
        ]

        for path in examples:
            with self.subTest(path=path):
                report = json.loads(path.read_text())
                self.assertEqual(report["tool"]["version"], CURRENT_VERSION)

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
        pinned_install = install_command + f"={CURRENT_VERSION}"
        container = f"quay.io/biocontainers/fastaguard:{CURRENT_CONTAINER}"
        docs = [
            ROOT / "README.md",
            ROOT / "docs" / "packaging.md",
            ROOT / "docs" / "adoption-plan.md",
            ROOT / "docs" / "workflow-readiness.md",
        ]

        for path in docs:
            with self.subTest(path=path):
                text = path.read_text()
                self.assertIn(install_command, text)
                self.assertIn(pinned_install, text)
                self.assertIn(container, text)
                self.assertNotIn("under Bioconda review", text)
                self.assertNotIn("does not include v0.3 gate behavior yet", text)

        packaging = (ROOT / "docs" / "packaging.md").read_text()
        self.assertNotIn("GitHub repository is private", packaging)
        self.assertNotIn("placeholder SHA256", packaging)
        self.assertIn('release_version="X.Y.Z"', packaging)
        self.assertIn('git tag "v${release_version}"', packaging)
        self.assertNotIn("git tag v0.6.0", packaging)

    def test_current_release_docs_do_not_present_v0_5_as_latest(self):
        current_docs = [
            ROOT / "README.md",
            ROOT / "docs" / "adoption-plan.md",
            ROOT / "docs" / "workflow-readiness.md",
            ROOT / "docs" / "packaging.md",
            ROOT / "docs" / "roadmap.md",
            ROOT / "docs" / "tool-landscape.md",
            ROOT / "docs" / "vision-plan.md",
        ]
        stale_claims = [
            "v0.5.0 remains the latest tag",
            "v0.5.0 is the latest tagged GitHub release",
            "until a\nv0.6 package and container are published",
        ]

        for path in current_docs:
            with self.subTest(path=path):
                text = path.read_text()
                self.assertIn(CURRENT_VERSION, text)
                for claim in stale_claims:
                    self.assertNotIn(claim, text)


if __name__ == "__main__":
    unittest.main()
