import csv
import gzip
import json
import re
from pathlib import Path

from scripts.collect_evidence import gzip_fasta


ROOT = Path(__file__).resolve().parents[2]
RESULTS_DIR = ROOT / "docs" / "evidence" / "results" / "v0.6"
JSON_PATH = RESULTS_DIR / "evidence_summary.json"
TSV_PATH = RESULTS_DIR / "evidence_summary.tsv"
EXPECTED_CASES = {
    "synthetic_valid",
    "problem_fixture",
    "gzipped_valid",
    "ecoli_k12_mg1655",
    "neurospora_crassa_or74a",
}
EXPECTED_ACCESSIONS = {"GCF_000005845.2", "GCF_000182925.2"}
EXPECTED_SOURCE_COMMIT = "cf27295da0cb9b1a48318caa9e3b8739cfd0c104"
EXPECTED_BINARY_SHA256 = "6dec7b558d29b3e72a96f6b81f942947fc50f84e183bfe5a390c665b33d21103"
VALID_STATUSES = {"PASS", "WARN", "FAIL"}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
STABLE_CASE_FIELDS = (
    "id",
    "label",
    "category",
    "source",
    "accession",
    "source_url",
    "evidence_role",
    "expected_scale",
    "downstream_route",
    "input_bytes",
    "input_sha256",
    "exit_code",
    "verdict",
    "gate_mode",
    "gate_status",
    "gate_blocking_findings",
    "sequence_count",
    "total_length",
    "n50",
    "n90",
    "finding_count",
    "finding_ids",
)


def load_summaries():
    summary = json.loads(JSON_PATH.read_text(encoding="utf-8"))
    with TSV_PATH.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    return summary, rows


def test_portable_summaries_have_expected_cases_and_release_provenance():
    summary, rows = load_summaries()

    assert summary["fastaguard_version"] == "0.6.0"
    assert summary["source_commit"] == EXPECTED_SOURCE_COMMIT
    assert {case["id"] for case in summary["cases"]} == EXPECTED_CASES
    assert {row["id"] for row in rows} == EXPECTED_CASES
    assert {case["accession"] for case in summary["cases"] if case["accession"]} == (
        EXPECTED_ACCESSIONS
    )

    for case in summary["cases"]:
        assert SHA256.fullmatch(case["input_sha256"])
        assert case["verdict"] in VALID_STATUSES
        assert case["gate_status"] in VALID_STATUSES
        assert isinstance(case["finding_ids"], list)
        assert case["finding_count"] == len(case["finding_ids"])

    for row in rows:
        assert row["fastaguard_version"] == "0.6.0"
        assert row["source_commit"] == EXPECTED_SOURCE_COMMIT
        assert SHA256.fullmatch(row["input_sha256"])
        assert row["verdict"] in VALID_STATUSES
        assert row["gate_status"] in VALID_STATUSES


def test_portable_provenance_distinguishes_observed_binary_from_source_check():
    summary, rows = load_summaries()

    assert summary["binary_sha256"] == EXPECTED_BINARY_SHA256
    assert summary["binary_to_source_reproducibility_attested"] is False
    scope = summary["provenance_scope"].lower()
    assert "source-tree" in scope
    assert "observed executable" in scope
    assert "not independently attested" in scope

    for row in rows:
        assert row["binary_sha256"] == EXPECTED_BINARY_SHA256
        assert row["binary_to_source_reproducibility_attested"] == "false"
        assert row["provenance_scope"] == summary["provenance_scope"]


def test_standalone_tsv_carries_runtime_interpretation_context():
    _, rows = load_summaries()

    assert rows
    for row in rows:
        assert row["platform"] == "macOS-26.5.1-arm64-arm-64bit-Mach-O"
        assert row["python"] == "3.14.5"
        runtime_context = row["runtime_context"].lower()
        assert "contextual" in runtime_context
        assert "not cross-platform performance guarantees" in runtime_context


def test_json_and_tsv_agree_on_stable_case_data():
    summary, rows = load_summaries()
    rows_by_id = {row["id"]: row for row in rows}

    for case in summary["cases"]:
        row = rows_by_id[case["id"]]
        for field in STABLE_CASE_FIELDS:
            expected = case[field]
            if isinstance(expected, list):
                expected = ",".join(expected)
            elif expected is None:
                expected = ""
            else:
                expected = str(expected)
            assert row[field] == expected, f"{case['id']} differs for {field}"


def test_portable_tsv_uses_repository_line_format():
    content = TSV_PATH.read_bytes()
    assert b"\r" not in content
    assert all(not line.endswith((b"\t", b" ")) for line in content.splitlines())


def test_portable_summaries_exclude_local_paths_commands_and_artifacts():
    repository_path = str(ROOT)
    forbidden_fragments = (
        repository_path,
        "/tmp/",
        "/private/tmp/",
        "target/evidence",
        "ncbi_dataset.zip",
    )
    forbidden_keys = {"input_path", "artifacts", "command"}

    for path in (JSON_PATH, TSV_PATH):
        text = path.read_text(encoding="utf-8")
        for fragment in forbidden_fragments:
            assert fragment not in text

    summary, _ = load_summaries()
    pending = [summary]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            assert forbidden_keys.isdisjoint(value)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
        elif isinstance(value, str):
            assert not value.startswith("/"), f"absolute path leaked: {value}"


def test_gzip_evidence_fixture_has_a_reproducible_timestamp(tmp_path):
    source = tmp_path / "input.fa"
    destination = tmp_path / "input.fa.gz"
    source.write_text(">record\nACGT\n", encoding="utf-8")

    gzip_fasta(source, destination)

    assert destination.read_bytes()[4:8] == b"\x00\x00\x00\x00"
    with gzip.open(destination, "rt", encoding="utf-8") as handle:
        assert handle.read() == ">record\nACGT\n"
