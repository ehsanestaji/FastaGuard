#!/usr/bin/env python3
"""Validate the FastaGuard Bioinformatics Application Note submission package.

Fails (exit 1) on manuscript TODOs/placeholders, forbidden framing, missing
Availability URLs, missing author emails, or evidence-table drift vs the frozen
v0.6 public-evidence TSV. Zenodo DOI absence is reported as a human blocker,
not a machine failure (no fake DOIs).

Usage:
  python3 paper/check_submission.py
  make -C paper check
"""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PAPER = ROOT / "paper"
MANIFEST = PAPER / "submission.yml"
MANUSCRIPT = PAPER / "main.tex"
EVIDENCE_TSV = ROOT / "docs" / "evidence" / "results" / "v0.6" / "evidence_summary.tsv"
FORBIDDEN = "FastQC for FASTA"
PLACEHOLDER_RES = (
    re.compile(r"\bTODO\b"),
    re.compile(r"\bTBD\b"),
    re.compile(r"\[institutional-email@", re.I),
    re.compile(r"\[Affiliation to be completed\]", re.I),
    re.compile(r"Funding:\s*\[to be completed\]", re.I),
    re.compile(r"example\.org", re.I),
)


def read(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"ERROR: missing required file: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def require_tokens(label: str, text: str, tokens: tuple[str, ...]) -> list[str]:
    return [f"{label} missing {token!r}" for token in tokens if token not in text]


def manifest_list(manifest: str, section: str) -> list[str]:
    pattern = rf"^{re.escape(section)}:\n((?:  - .+\n)+)"
    match = re.search(pattern, manifest, flags=re.MULTILINE)
    if not match:
        return []
    return [line.strip()[2:] for line in match.group(1).splitlines() if line.strip().startswith("- ")]


def load_evidence_rows() -> dict[str, dict[str, str]]:
    with EVIDENCE_TSV.open(encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        return {row["id"]: row for row in reader}


def main() -> int:
    errors: list[str] = []
    warnings: list[str] = []
    human_blockers: list[str] = []

    manifest = read(MANIFEST)
    manuscript = read(MANUSCRIPT)

    errors += require_tokens(
        "paper/submission.yml",
        manifest,
        (
            "target_journal: Bioinformatics",
            "article_type: Application Note",
            "tag: v1.0.0",
            "fastaguard_version: \"0.6.0\"",
            "forbidden_phrase: \"FastQC for FASTA\"",
            "performance_claim_source: docs/evidence_only",
            "ehsan.estaji@umu.se",
            "jianfeng.mao@umu.se",
        ),
    )

    errors += require_tokens(
        "paper/main.tex",
        manuscript,
        (
            r"\begin{abstract}",
            r"\textbf{Summary:}",
            r"\textbf{Availability and Implementation:}",
            r"\textbf{Contact:}",
            r"\textbf{Supplementary information:}",
            "https://github.com/ehsanestaji/FastaGuard",
            "ehsan.estaji@umu.se",
            "jianfeng.mao@umu.se",
            "0009-0005-0838-228X",
            "0000-0001-9735-8516",
            r"\section*{Funding}",
            r"\section*{Conflict of interest}",
            r"\section*{AI usage disclosure}",
            "WIFORCE",
            "v1.0.0",
            "0.0194",
            "0.0185",
            "0.0569",
            "0.3441",
            "GCF\\_000005845.2",
            "GCF\\_000182925.2",
        ),
    )

    if FORBIDDEN.lower() in manuscript.lower():
        # Allow negation contexts that explicitly reject the phrase.
        for match in re.finditer(re.escape(FORBIDDEN), manuscript, flags=re.IGNORECASE):
            window = manuscript[max(0, match.start() - 40) : match.end() + 10].lower()
            if "not" in window or "never" in window or "avoid" in window or "rather than" in window:
                continue
            errors.append(f"paper/main.tex contains forbidden phrase {FORBIDDEN!r}")
            break

    for pattern in PLACEHOLDER_RES:
        if pattern.search(manuscript):
            errors.append(f"paper/main.tex still has placeholder matching {pattern.pattern}")

    # Evidence freeze: every table timing/accession must match the TSV.
    if not EVIDENCE_TSV.exists():
        errors.append(f"missing evidence freeze file: {EVIDENCE_TSV.relative_to(ROOT)}")
    else:
        rows = load_evidence_rows()
        expected = {
            "synthetic_valid": ("0.0194", "PASS", "604", "4"),
            "problem_fixture": ("0.0185", "FAIL", "145", "5"),
            "ecoli_k12_mg1655": ("0.0569", "PASS", "4641652", "1"),
            "neurospora_crassa_or74a": ("0.3441", "WARN", "41102378", "21"),
        }
        for row_id, (elapsed, verdict, length, nseq) in expected.items():
            row = rows.get(row_id)
            if not row:
                errors.append(f"evidence TSV missing row id={row_id}")
                continue
            if row["elapsed_seconds"] != elapsed:
                errors.append(
                    f"evidence drift for {row_id}: elapsed_seconds={row['elapsed_seconds']!r} != {elapsed!r}"
                )
            if row["verdict"] != verdict or row["gate_status"] != verdict:
                errors.append(
                    f"evidence drift for {row_id}: verdict/gate={row['verdict']}/{row['gate_status']}"
                )
            if row["total_length"] != length or row["sequence_count"] != nseq:
                errors.append(f"evidence drift for {row_id}: length/count mismatch")
            if elapsed not in manuscript:
                errors.append(f"paper/main.tex missing frozen elapsed value {elapsed} for {row_id}")

    for rel in manifest_list(manifest, "submission_docs"):
        path = ROOT / rel
        if not path.exists():
            errors.append(f"manifest path does not exist: {rel}")

    for required in (
        PAPER / "cover-letter.md",
        PAPER / "ZENODO.md",
        PAPER / "biorxiv-checklist.md",
        ROOT / "docs" / "paper-readiness.md",
        ROOT / ".zenodo.json",
    ):
        if not required.exists():
            errors.append(f"missing required submission file: {required.relative_to(ROOT)}")

    # Rough word-count band for Application Note body.
    body = re.sub(r"\\begin\{tikzpicture\}.*?\\end\{tikzpicture\}", " ", manuscript, flags=re.S)
    body = re.sub(r"\\begin\{verbatim\}.*?\\end\{verbatim\}", " ", body, flags=re.S)
    plain = re.sub(r"\\[a-zA-Z]+\*?(?:\[[^\]]*\])?", " ", body)
    plain = re.sub(r"[{}]", " ", plain)
    words = re.findall(r"[A-Za-z0-9']+", plain)
    n_words = len(words)
    if n_words < 1800 or n_words > 3200:
        warnings.append(
            f"approximate manuscript token count {n_words} outside soft band 1800–3200 "
            "(Application Note target ~2000–2600 words)"
        )

    if "archive_doi: null" in manifest or "version_doi: null" in manifest:
        human_blockers.append("Zenodo DOI not yet minted (see paper/ZENODO.md) — USER-ACTION")
    human_blockers.append("ScholarOne upload — USER-ACTION")
    human_blockers.append("Co-author final confirmation — USER-ACTION")
    human_blockers.append("APC / Read & Publish / ISCB discount check — USER-ACTION")

    for warning in warnings:
        print(f"WARNING: {warning}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print("submission package machine checks PASSED")
    print(f"approximate word/token count: {n_words}")
    print("remaining USER-ACTION blockers (not machine failures):")
    for blocker in human_blockers:
        print(f"- {blocker}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
