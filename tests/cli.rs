use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn help_mentions_preflight_positioning() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("FASTA preflight QC"));
}

#[test]
fn help_does_not_advertise_removed_warning_flag() {
    let removed_flag = ["--warn", "-on"].concat();
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(removed_flag).not());
}

#[test]
fn contract_schema_can_be_printed_without_input() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--schema")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""$schema""#))
        .stdout(predicate::str::contains(r#""FastaguardReport""#))
        .stdout(predicate::str::contains(r#""machine_summary""#))
        .stdout(predicate::str::contains(r#""provenance""#))
        .stdout(predicate::str::contains(r#""evidence""#))
        .stdout(predicate::str::contains(r#""actions""#))
        .stderr(predicate::str::is_empty());
}

#[test]
fn contract_finding_catalog_can_be_printed_without_input() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--finding-catalog")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""schema_version": "0.1.0""#))
        .stdout(predicate::str::contains(r#""duplicate_ids""#))
        .stdout(predicate::str::contains(r#""invalid_fasta_structure""#))
        .stderr(predicate::str::is_empty());
}

#[test]
fn contract_explain_finding_prints_single_catalog_entry() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["--explain-finding", "high_n_rate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""id": "high_n_rate""#))
        .stdout(predicate::str::contains(r#""recommended_next_tools""#))
        .stdout(predicate::str::contains(r#""id": "duplicate_ids""#).not())
        .stderr(predicate::str::is_empty());
}

#[test]
fn contract_unknown_finding_is_tool_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["--explain-finding", "unknown_rule"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "unknown finding id 'unknown_rule'",
        ));
}

#[test]
fn valid_assembly_writes_all_outputs_and_passes() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "valid");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--out",
    ])
    .arg(&outputs.html)
    .arg("--json")
    .arg(&outputs.json)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .success()
    .stderr(predicate::str::is_empty());

    assert_all_outputs_exist(&outputs);
    let json = std::fs::read_to_string(&outputs.json).unwrap();
    assert!(json.contains(r#""status": "PASS""#), "{json}");
}

#[test]
fn valid_report_includes_machine_summary_scope_and_provenance() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "valid_machine");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--out",
    ])
    .arg(&outputs.html)
    .arg("--json")
    .arg(&outputs.json)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .success()
    .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["machine_summary"]["verdict"], json!("PASS"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(true)
    );
    assert_eq!(report["machine_summary"]["top_findings"], json!([]));
    assert!(array_contains_tool(
        &report["machine_summary"]["recommended_next_tools"],
        "QUAST"
    ));
    assert!(array_contains_tool(
        &report["machine_summary"]["recommended_next_tools"],
        "BUSCO"
    ));
    assert_eq!(report["scope"]["level"], json!("fasta_preflight"));
    assert!(array_contains_string(
        &report["scope"]["can_conclude"],
        "FASTA parse validity"
    ));
    assert!(array_contains_string(
        &report["scope"]["cannot_conclude"],
        "biological completeness"
    ));
    assert_eq!(report["provenance"]["profile"], json!("assembly"));
    assert_eq!(report["provenance"]["threads"], json!(1));
    assert_eq!(
        report["provenance"]["thresholds"]["min_contig_length"],
        json!(1)
    );
    assert_eq!(
        report["provenance"]["thresholds"]["high_global_n_fraction"],
        json!(0.05)
    );
}

#[test]
fn valid_assembly_json_matches_golden_contract() {
    let paths = golden_output_paths("valid_assembly");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--out",
    ])
    .arg(&paths.html)
    .arg("--json")
    .arg(&paths.json)
    .arg("--tsv")
    .arg(&paths.tsv)
    .arg("--multiqc")
    .arg(&paths.multiqc)
    .assert()
    .success()
    .stderr(predicate::str::is_empty());

    assert_json_matches_golden(&paths.json, "tests/golden/valid_assembly.json");
}

#[test]
fn problem_assembly_returns_failure_for_default_critical_findings() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "problem");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    assert_all_outputs_exist(&outputs);
}

#[test]
fn problem_assembly_json_matches_golden_contract() {
    let paths = golden_output_paths("problem_assembly");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--out"])
        .arg(&paths.html)
        .arg("--json")
        .arg(&paths.json)
        .arg("--tsv")
        .arg(&paths.tsv)
        .arg("--multiqc")
        .arg(&paths.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    assert_json_matches_golden(&paths.json, "tests/golden/problem_assembly.json");
}

#[test]
fn problem_report_includes_structured_finding_actions() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "problem_machine");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["machine_summary"]["verdict"], json!("FAIL"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "duplicate_ids"
    ));

    let duplicate_ids = finding_by_id(&report, "duplicate_ids");
    assert_eq!(
        duplicate_ids["actions"][0]["action_type"],
        json!("rename_records")
    );
    assert_eq!(
        duplicate_ids["actions"][0]["requires_external_database"],
        json!(false)
    );
    assert_eq!(
        duplicate_ids["actions"][0]["recommended_tool"],
        json!("seqkit")
    );
}

#[test]
fn problem_report_includes_per_record_evidence() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "problem_evidence");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/problem_assembly.fa", "--out"])
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    let duplicate_ids = finding_by_id(&report, "duplicate_ids");
    assert_eq!(duplicate_ids["evidence"]["total_records"], json!(1));
    assert_eq!(duplicate_ids["evidence"]["truncated"], json!(false));
    assert_eq!(duplicate_ids["evidence"]["records"][0]["id"], json!("dup"));
    assert_eq!(
        duplicate_ids["evidence"]["records"][0]["reason"],
        json!("duplicate FASTA identifier")
    );

    let invalid_chars = finding_by_id(&report, "invalid_chars");
    assert_eq!(
        invalid_chars["evidence"]["records"][0]["id"],
        json!("bad_chars")
    );
    assert_eq!(
        invalid_chars["evidence"]["records"][0]["invalid_count"],
        json!(2)
    );

    let high_n_rate = finding_by_id(&report, "high_n_rate");
    assert!(array_contains_record_id(
        &high_n_rate["evidence"]["records"],
        "gap_rich"
    ));
    assert_eq!(
        high_n_rate["evidence"]["records"][0]["n_fraction"],
        json!(1.0)
    );

    let gap_runs = finding_by_id(&report, "gap_runs");
    assert_eq!(
        gap_runs["evidence"]["records"][0]["max_gap_run"],
        json!(101)
    );
}

#[test]
fn invalid_fasta_json_matches_golden_contract() {
    let paths = golden_output_paths("invalid_empty_record");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/invalid_empty_record.fa")
        .arg("--out")
        .arg(&paths.html)
        .arg("--json")
        .arg(&paths.json)
        .arg("--tsv")
        .arg(&paths.tsv)
        .arg("--multiqc")
        .arg(&paths.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    assert_json_matches_golden(&paths.json, "tests/golden/invalid_empty_record.json");
}

#[test]
fn structurally_invalid_fasta_returns_failure_report() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("invalid.fa");
    std::fs::write(&input, ">empty\n").unwrap();
    let outputs = output_paths(&temp_dir, "invalid");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .arg("--out")
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    assert_all_outputs_exist(&outputs);
    let json = std::fs::read_to_string(&outputs.json).unwrap();
    assert!(json.contains(r#""status": "FAIL""#), "{json}");
    assert!(json.contains("invalid_fasta_structure"), "{json}");
}

#[test]
fn invalid_fasta_report_includes_machine_contract_fields() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("invalid.fa");
    std::fs::write(&input, ">empty\n").unwrap();
    let outputs = output_paths(&temp_dir, "invalid_machine");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .arg("--out")
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["machine_summary"]["verdict"], json!("FAIL"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "invalid_fasta_structure"
    ));
    assert_eq!(report["scope"]["level"], json!("fasta_preflight"));
    assert_eq!(
        report["provenance"]["thresholds"]["min_contig_length"],
        json!(200)
    );

    let invalid_structure = finding_by_id(&report, "invalid_fasta_structure");
    assert_eq!(
        invalid_structure["actions"][0]["action_type"],
        json!("repair_fasta_structure")
    );
}

#[test]
fn missing_input_file_is_tool_error() {
    let temp_dir = TempDir::new().unwrap();
    let missing = temp_dir.path().join("missing.fa");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&missing)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("failed to open"));
}

#[test]
fn unsupported_profile_is_tool_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/valid_assembly.fa", "--profile", "reads"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unsupported profile"));
}

struct OutputPaths {
    html: std::path::PathBuf,
    json: std::path::PathBuf,
    tsv: std::path::PathBuf,
    multiqc: std::path::PathBuf,
}

fn output_paths(temp_dir: &TempDir, stem: &str) -> OutputPaths {
    OutputPaths {
        html: temp_dir.path().join(format!("{stem}.html")),
        json: temp_dir.path().join(format!("{stem}.json")),
        tsv: temp_dir.path().join(format!("{stem}.tsv")),
        multiqc: temp_dir.path().join(format!("{stem}_multiqc.json")),
    }
}

fn assert_all_outputs_exist(outputs: &OutputPaths) {
    assert!(outputs.html.exists(), "missing {}", outputs.html.display());
    assert!(outputs.json.exists(), "missing {}", outputs.json.display());
    assert!(outputs.tsv.exists(), "missing {}", outputs.tsv.display());
    assert!(
        outputs.multiqc.exists(),
        "missing {}",
        outputs.multiqc.display()
    );
}

fn golden_output_paths(stem: &str) -> OutputPaths {
    let dir = Path::new("target").join("fastaguard-golden-runtime");
    std::fs::create_dir_all(&dir).unwrap();
    OutputPaths {
        html: dir.join(format!("{stem}.html")),
        json: dir.join(format!("{stem}.json")),
        tsv: dir.join(format!("{stem}.tsv")),
        multiqc: dir.join(format!("{stem}_multiqc.json")),
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn assert_json_matches_golden(actual_path: &Path, golden_path: &str) {
    let actual = read_json(actual_path);
    let golden_path = PathBuf::from(golden_path);
    let golden = read_json(&golden_path);

    assert_eq!(
        actual,
        golden,
        "actual JSON at {} differed from golden {}",
        actual_path.display(),
        golden_path.display()
    );
}

fn finding_by_id<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["id"] == json!(id))
        .unwrap_or_else(|| panic!("missing finding {id}: {report}"))
}

fn array_contains_string(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == expected)
}

fn array_contains_tool(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["tool"] == json!(expected))
}

fn array_contains_record_id(value: &Value, expected: &str) -> bool {
    value
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == json!(expected))
}
