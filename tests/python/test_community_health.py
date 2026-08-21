import re
import subprocess
import unittest
from pathlib import Path

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
PROHIBITED_ATTRIBUTION_PATTERNS = [
    re.compile(r"(?im)^co-authored-by:\s*(?:chatgpt|codex|openai)\b"),
    re.compile(r"(?im)^generated-by:\s*(?:chatgpt|codex|openai)\b"),
    re.compile(r"(?i)written with (?:chatgpt|codex|openai)"),
]


class CommunityHealthTest(unittest.TestCase):
    def tracked_paths(self):
        return subprocess.check_output(
            ["git", "ls-files"], cwd=ROOT, text=True
        ).splitlines()

    def test_required_community_files_exist(self):
        missing = [path for path in REQUIRED_PATHS if not (ROOT / path).is_file()]

        self.assertEqual(missing, [])

    def test_contributing_documents_project_verification_and_dco(self):
        text = (ROOT / "CONTRIBUTING.md").read_text()

        self.assertIn("cargo test --locked", text)
        self.assertIn("python3 -m pytest -q", text)
        self.assertRegex(text, r"(?i)signed-off-by")

    def test_security_policy_uses_private_github_security_advisories(self):
        text = (ROOT / "SECURITY.md").read_text()

        self.assertRegex(text, r"(?is)private.*?GitHub Security\s+Advisories")

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
            path for path in self.tracked_paths() if path.startswith("docs/superpowers/")
        ]

        self.assertEqual(planning_paths, [])

    def test_tracked_tree_has_no_prohibited_assistant_attribution_traces(self):
        violations = []
        for path in self.tracked_paths():
            text = (ROOT / path).read_text(errors="ignore")
            if any(pattern.search(text) for pattern in PROHIBITED_ATTRIBUTION_PATTERNS):
                violations.append(path)

        self.assertEqual(violations, [])


if __name__ == "__main__":
    unittest.main()
