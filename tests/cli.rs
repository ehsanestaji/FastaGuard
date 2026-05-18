use assert_cmd::Command;
use predicates::prelude::*;
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
