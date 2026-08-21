import re
import subprocess
import unittest
from pathlib import Path
from urllib.parse import urlparse

import yaml


ROOT = Path(__file__).resolve().parents[2]
REQUIRED_PATHS = [
    "CONTRIBUTING.md",
    "CODE_OF_CONDUCT.md",
    "SECURITY.md",
    ".github/ISSUE_TEMPLATE/bug_report.yml",
    ".github/ISSUE_TEMPLATE/feature_request.yml",
    ".github/ISSUE_TEMPLATE/config.yml",
    ".github/pull_request_template.md",
]
REQUIRED_ISSUE_DETAILS = {
    "FastaGuard version",
    "Operating system",
    "Installation method",
    "Exact command",
    "Minimal FASTA reproducer or safe description",
    "Expected result",
    "Actual result",
}
ASSISTANT_IDENTITY = r"(?:chatgpt|codex|claude(?:\s+code)?|openai|anthropic)"
GENERIC_AI_TOOL = r"(?:(?:an?\s+)?(?:ai|assistant)(?:\s+(?:tool|system|model))?)"
PROHIBITED_ATTRIBUTION_SOURCE = rf"(?:{ASSISTANT_IDENTITY}|{GENERIC_AI_TOOL})"
PROHIBITED_ATTRIBUTION_PATTERNS = [
    re.compile(rf"(?im)^co-authored-by:\s*{PROHIBITED_ATTRIBUTION_SOURCE}\b"),
    re.compile(
        rf"(?im)^generated(?:-|\s+)by\s*:?\s*{PROHIBITED_ATTRIBUTION_SOURCE}\b"
    ),
    re.compile(rf"(?i)written with {PROHIBITED_ATTRIBUTION_SOURCE}\b"),
    re.compile(
        r"(?i)\bai[- ]assisted\s+(?:contribution|change|code|content|documentation)\b"
    ),
    re.compile(r"(?im)^assistant\s+provenance\s*:"),
]
ATTRIBUTION_TRACE_SAMPLES = (
    "Generated" + "-by: Code" + "x",
    "generated" + " by Code" + "x",
    "Co-authored" + "-by: Claude" + " Code",
    "Generated" + " by Anthro" + "pic",
    "Generated" + " by an " + "AI tool",
    "AI" + "-assisted contribution",
    "Assistant " + "provenance: automation",
)


class CommunityHealthTest(unittest.TestCase):
    def tracked_paths(self):
        paths = subprocess.check_output(
            ["git", "ls-files"], cwd=ROOT, text=True
        ).splitlines()
        return [path for path in paths if (ROOT / path).is_file()]

    @staticmethod
    def has_security_policy_contact(config):
        for contact in config.get("contact_links", []):
            if not isinstance(contact, dict):
                continue

            destination = urlparse(contact.get("url", ""))
            context = " ".join(
                str(contact.get(field, "")) for field in ("name", "about")
            ).lower()
            if (
                destination.scheme == "https"
                and destination.netloc.lower() == "github.com"
                and destination.path.endswith("/security/policy")
                and "security" in context
            ):
                return True

        return False

    def test_required_community_files_exist(self):
        missing = [path for path in REQUIRED_PATHS if not (ROOT / path).is_file()]

        self.assertEqual(missing, [])

    def test_contributing_documents_project_verification_and_dco(self):
        text = (ROOT / "CONTRIBUTING.md").read_text()

        self.assertIn("cargo test --locked", text)
        self.assertRegex(text, r"(?i)signed-off-by")

    def test_contributing_uses_ignored_venv_with_declared_python_dependencies(self):
        text = (ROOT / "CONTRIBUTING.md").read_text()
        create = re.search(
            r"(?m)^python3\s+-m\s+venv\s+(target/[^\s]+)\s*$",
            text,
        )

        self.assertIsNotNone(create)
        environment = re.escape(create.group(1))
        install = re.search(
            rf"(?m)^{environment}/bin/python\s+-m\s+pip\s+install\s+"
            r"--requirement\s+requirements-test\.txt\s*$",
            text,
        )
        test = re.search(
            rf"(?m)^{environment}/bin/python\s+-m\s+pytest\s+-q\s+"
            r"tests/python\s*$",
            text,
        )

        self.assertIsNotNone(install)
        self.assertIsNotNone(test)
        self.assertLess(create.start(), install.start())
        self.assertLess(install.start(), test.start())
        self.assertTrue((ROOT / "requirements-test.txt").is_file())
        self.assertIn("target/", (ROOT / ".gitignore").read_text().splitlines())

    def test_ci_installs_dependencies_and_runs_full_python_suite(self):
        workflow = yaml.safe_load(
            (ROOT / ".github" / "workflows" / "ci.yml").read_text()
        )
        steps = workflow["jobs"]["rust"]["steps"]
        commands = [step.get("run", "") for step in steps]
        python_setups = [
            step
            for step in steps
            if step.get("uses", "").startswith("actions/setup-python@")
        ]
        requirements = (ROOT / "requirements-test.txt").read_text().splitlines()

        self.assertIn(
            "python3 -m pip install --requirement requirements-test.txt",
            commands,
        )
        self.assertEqual(len(python_setups), 1)
        self.assertEqual(
            python_setups[0].get("with", {}).get("python-version"), "3.12"
        )
        self.assertIn("python3 -m pytest -q tests/python", commands)
        self.assertEqual(
            requirements,
            [
                "build==1.5.0",
                "jsonschema==4.26.0",
                "pytest==9.1.1",
                "PyYAML==6.0.3",
            ],
        )

    def test_ci_checks_multiqc_plugin_python_and_multiqc_compatibility(self):
        workflow = yaml.safe_load(
            (ROOT / ".github" / "workflows" / "ci.yml").read_text()
        )
        job = workflow["jobs"]["multiqc-plugin"]
        matrix = job["strategy"]["matrix"]
        steps = job["steps"]
        test_step = next(
            step for step in steps if step["name"] == "Test built MultiQC plugin"
        )

        self.assertEqual(matrix["python"], ["3.10", "3.14"])
        self.assertEqual(matrix["multiqc"], ["1.28", "1.35"])
        self.assertEqual(
            test_step["env"]["FASTAGUARD_MULTIQC_VERSION"],
            "${{ matrix.multiqc }}",
        )
        self.assertEqual(
            test_step["run"],
            "python3 -m pytest -q tests/python/test_multiqc_plugin.py",
        )

    def test_security_policy_uses_functional_private_email(self):
        text = (ROOT / "SECURITY.md").read_text()

        self.assertIn("mailto:ehsan.estaji@umu.se", text)
        self.assertRegex(text, r"(?is)private.*?ehsan\.estaji@umu\.se")
        self.assertNotIn("/security/advisories/new", text)
        self.assertNotIn("issues/new", text)

    def test_code_of_conduct_uses_confidential_maintainer_email(self):
        text = (ROOT / "CODE_OF_CONDUCT.md").read_text()

        self.assertIn("mailto:ehsan.estaji@umu.se", text)
        self.assertRegex(text, r"(?is)confidential.*?ehsan\.estaji@umu\.se")
        self.assertNotIn("issues/new", text)

    def test_issue_forms_are_parseable_and_collect_reproducible_context(self):
        for path in (
            ".github/ISSUE_TEMPLATE/bug_report.yml",
            ".github/ISSUE_TEMPLATE/feature_request.yml",
        ):
            with self.subTest(path=path):
                form = yaml.safe_load((ROOT / path).read_text())
                self.assertIsInstance(form, dict)
                self.assertTrue(form.get("name"))
                self.assertTrue(form.get("description"))
                self.assertIsInstance(form.get("body"), list)

                labels = {
                    field.get("attributes", {}).get("label")
                    for field in form["body"]
                    if isinstance(field, dict)
                }
                self.assertTrue(REQUIRED_ISSUE_DETAILS <= labels)

    def test_issue_template_config_is_parseable_and_structured(self):
        config = yaml.safe_load(
            (ROOT / ".github/ISSUE_TEMPLATE/config.yml").read_text()
        )

        self.assertIsInstance(config, dict)
        self.assertIs(config.get("blank_issues_enabled"), False)
        self.assertIsInstance(config.get("contact_links"), list)
        self.assertTrue(self.has_security_policy_contact(config))
        config_text = (
            ROOT / ".github" / "ISSUE_TEMPLATE" / "config.yml"
        ).read_text()
        self.assertNotIn("/security/advisories/new", config_text)

    def test_security_contact_rejects_public_issue_destination(self):
        inadequate_config = {
            "contact_links": [
                {
                    "name": "Security vulnerability report",
                    "url": "https://github.com/ehsanestaji/FastaGuard/issues/new",
                    "about": "Use this link for security reports.",
                }
            ]
        }

        self.assertFalse(self.has_security_policy_contact(inadequate_config))

        disabled_advisory_config = {
            "contact_links": [
                {
                    "name": "Security vulnerability report",
                    "url": "https://github.com/ehsanestaji/FastaGuard/security/advisories/new",
                    "about": "Use this link for security reports.",
                }
            ]
        }
        self.assertFalse(self.has_security_policy_contact(disabled_advisory_config))

    def test_pull_request_template_covers_review_mechanisms(self):
        text = (ROOT / ".github/pull_request_template.md").read_text()

        for mechanism in (
            "Scoped change description",
            "Tests",
            "Contract impact",
            "Documentation impact",
            "DCO",
            "Attribution-trace review",
        ):
            with self.subTest(mechanism=mechanism):
                self.assertIn(mechanism, text)

    def test_tracked_tree_excludes_tool_specific_planning_paths(self):
        planning_paths = [
            path
            for path in self.tracked_paths()
            if path == "AGENTS.md" or path.startswith("docs/superpowers/")
        ]

        self.assertEqual(planning_paths, [])

    def test_public_documentation_excludes_tool_specific_planning_references(self):
        violations = []
        for path in self.tracked_paths():
            if path == "README.md" or path.startswith("docs/"):
                text = (ROOT / path).read_text(errors="ignore")
                if "docs/superpowers" in text:
                    violations.append(path)

        self.assertEqual(violations, [])

    def test_tracked_tree_has_no_prohibited_assistant_attribution_traces(self):
        violations = []
        for path in self.tracked_paths():
            text = (ROOT / path).read_text(errors="ignore")
            if any(pattern.search(text) for pattern in PROHIBITED_ATTRIBUTION_PATTERNS):
                violations.append(path)

        self.assertEqual(violations, [])

    def test_attribution_trace_patterns_cover_common_provenance_forms(self):
        for trace in ATTRIBUTION_TRACE_SAMPLES:
            with self.subTest(trace=trace):
                self.assertTrue(
                    any(
                        pattern.search(trace)
                        for pattern in PROHIBITED_ATTRIBUTION_PATTERNS
                    )
                )

    def test_attribution_patterns_allow_non_assistant_provenance(self):
        unrelated_provenance = "Generated-by: release tooling"

        self.assertFalse(
            any(
                pattern.search(unrelated_provenance)
                for pattern in PROHIBITED_ATTRIBUTION_PATTERNS
            )
        )


if __name__ == "__main__":
    unittest.main()
