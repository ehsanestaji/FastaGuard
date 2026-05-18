use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_preflight_positioning() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("FASTA preflight QC"));
}

#[test]
fn invalid_runtime_config_prints_fastaguard_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["input.fa", "--max-n-rate", "1.2"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("fastaguard error:"));
}
