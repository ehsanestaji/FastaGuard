use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const GOLDEN_PROVENANCE_TIMESTAMP: &str = "2026-05-23T00:00:00Z";
const COMPARE_GOLDEN_PROVENANCE_TIMESTAMP: &str = "2026-06-02T00:00:00Z";

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
        .stdout(predicate::str::contains(r#""schema_version": "0.3.0""#))
        .stdout(predicate::str::contains(r#""duplicate_ids""#))
        .stdout(predicate::str::contains(r#""invalid_fasta_structure""#))
        .stdout(predicate::str::contains(r#""gc_outliers""#))
        .stdout(predicate::str::contains(r#""length_outliers""#))
        .stdout(predicate::str::contains(r#""composite_anomalies""#))
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
fn contract_explain_finding_prints_outlier_catalog_entries() {
    for id in ["gc_outliers", "length_outliers", "composite_anomalies"] {
        let mut cmd = Command::cargo_bin("fastaguard").unwrap();
        cmd.args(["--explain-finding", id])
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(r#""id": "{id}""#)))
            .stdout(predicate::str::contains(r#""recommended_next_tools""#))
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn contract_explain_composite_anomalies_includes_taxonomy_and_signals() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["--explain-finding", "composite_anomalies"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""category": "composition""#))
        .stdout(predicate::str::contains(r#""confidence": "moderate""#))
        .stdout(predicate::str::contains(
            r#""requires_followup_tool": true"#,
        ))
        .stdout(predicate::str::contains(
            r#""findings[].evidence.records[].signals""#,
        ))
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
fn compare_requires_at_least_two_inputs() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["compare", "testdata/valid_assembly.fa"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "compare requires at least two FASTA inputs",
        ));
}

#[test]
fn compare_writes_json_with_mixed_status_samples() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "cohort");
    let multiqc = temp_dir.path().join("cohort_mqc.json");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "compare",
        "testdata/valid_assembly.fa",
        "testdata/problem_assembly.fa",
        "--gate",
        "pipeline",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&multiqc)
    .assert()
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["report_type"], json!("compare"));
    assert_eq!(report["schema_version"], json!("0.4.0"));
    assert_eq!(report["summary"]["sample_count"], json!(2));
    assert_eq!(report["summary"]["fail_count"], json!(1));
    let samples = report["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 2);
    assert!(samples.iter().any(|sample| {
        sample["recommended_next_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool == "seqkit")
    }));
    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    assert!(tsv.contains("sample_id\tinput_path\tverdict"), "{tsv}");
    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Readiness Matrix"), "{html}");
    let multiqc_report = read_json(&multiqc);
    assert_eq!(multiqc_report["plot_type"], json!("table"));
    assert!(
        multiqc_report["data"].get("valid_assembly").is_some(),
        "{multiqc_report}"
    );
    assert!(multiqc.exists(), "missing {}", multiqc.display());
}

#[test]
fn compare_golden_mixed_status_matches() {
    let paths = golden_output_paths("compare_mixed_status");
    let provenance_command = compare_golden_provenance_command("compare_mixed_status");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_compare_golden_provenance(&mut cmd, provenance_command);
    cmd.args([
        "compare",
        "testdata/valid_assembly.fa",
        "testdata/problem_assembly.fa",
        "--gate",
        "pipeline",
        "--json",
    ])
    .arg(&paths.json)
    .arg("--out")
    .arg(&paths.html)
    .arg("--tsv")
    .arg(&paths.tsv)
    .arg("--multiqc")
    .arg(&paths.multiqc)
    .assert()
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    assert_json_matches_golden(&paths.json, "tests/golden/compare_mixed_status.json");
}

#[test]
fn compare_golden_all_pass_matches() {
    let (first, second) = write_compare_all_pass_inputs();
    let paths = golden_output_paths("compare_all_pass");
    let provenance_command = compare_golden_provenance_command("compare_all_pass");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_compare_golden_provenance(&mut cmd, provenance_command);
    cmd.arg("compare")
        .arg(&first)
        .arg(&second)
        .arg("--json")
        .arg(&paths.json)
        .arg("--out")
        .arg(&paths.html)
        .arg("--tsv")
        .arg(&paths.tsv)
        .arg("--multiqc")
        .arg(&paths.multiqc)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    assert_json_matches_golden(&paths.json, "tests/golden/compare_all_pass.json");
}

#[test]
fn compare_rejects_duplicate_sample_ids() {
    let temp_dir = TempDir::new().unwrap();
    let first_dir = temp_dir.path().join("a");
    let second_dir = temp_dir.path().join("b");
    std::fs::create_dir(&first_dir).unwrap();
    std::fs::create_dir(&second_dir).unwrap();
    let first = first_dir.join("sample.fa");
    let second = second_dir.join("sample.fa");
    std::fs::write(&first, ">one\nACGT\n").unwrap();
    std::fs::write(&second, ">two\nACGT\n").unwrap();
    let outputs = output_paths(&temp_dir, "duplicate_sample");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("compare")
        .arg(&first)
        .arg(&second)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--out")
        .arg(&outputs.html)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "duplicate compare sample_id 'sample'",
        ));

    assert!(
        !outputs.html.exists(),
        "unexpected {}",
        outputs.html.display()
    );
    assert!(
        !outputs.json.exists(),
        "unexpected {}",
        outputs.json.display()
    );
    assert!(
        !outputs.tsv.exists(),
        "unexpected {}",
        outputs.tsv.display()
    );
    assert!(
        !outputs.multiqc.exists(),
        "unexpected {}",
        outputs.multiqc.display()
    );
}

#[test]
fn compare_includes_structurally_invalid_fasta_sample() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "invalid_cohort");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "compare",
        "testdata/valid_assembly.fa",
        "testdata/invalid_empty_record.fa",
        "--json",
    ])
    .arg(&outputs.json)
    .arg("--out")
    .arg(&outputs.html)
    .arg("--tsv")
    .arg(&outputs.tsv)
    .arg("--multiqc")
    .arg(&outputs.multiqc)
    .assert()
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    let samples = report["samples"].as_array().unwrap();
    let invalid_sample = samples
        .iter()
        .find(|sample| sample["sample_id"] == "invalid_empty_record")
        .unwrap_or_else(|| panic!("missing invalid sample: {report}"));
    assert_eq!(invalid_sample["verdict"], json!("FAIL"));
    assert_eq!(invalid_sample["gate_status"], json!("FAIL"));
    assert!(array_contains_string(
        &invalid_sample["finding_ids"],
        "invalid_fasta_structure"
    ));
}

#[test]
fn valid_assembly_writes_all_outputs_and_warns_for_terminal_ns() {
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
    .code(1)
    .stderr(predicate::str::is_empty());

    assert_all_outputs_exist(&outputs);
    let json = std::fs::read_to_string(&outputs.json).unwrap();
    assert!(json.contains(r#""status": "WARN""#), "{json}");
    assert!(json.contains(r#""terminal_ns""#), "{json}");
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
    .code(1)
    .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["machine_summary"]["verdict"], json!("WARN"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert_eq!(
        report["machine_summary"]["top_findings"],
        json!(["terminal_ns"])
    );
    assert!(array_contains_tool(
        &report["machine_summary"]["recommended_next_tools"],
        "seqkit"
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
fn report_includes_v0_4_provenance_and_routing_hints() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "v02_contract");

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
    .code(1);

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.4.0"));
    assert_eq!(report["gate"]["mode"], json!("none"));
    assert_eq!(report["gate"]["status"], json!("WARN"));
    assert_eq!(report["gate"]["blocking_findings"], json!([]));
    assert_eq!(report["gate"]["advisory_findings"], json!(["terminal_ns"]));
    assert!(report["provenance"]["command"]
        .as_str()
        .unwrap()
        .contains("fastaguard"));
    assert!(report["provenance"]["started_at"]
        .as_str()
        .unwrap()
        .ends_with('Z'));
    assert!(report["provenance"]["completed_at"]
        .as_str()
        .unwrap()
        .ends_with('Z'));
    assert!(report["provenance"]["duration_ms"].as_u64().is_some());
    assert!(report["provenance"]["input_size_bytes"].as_u64().unwrap() > 0);
    assert_eq!(
        report["provenance"]["input_sha256"],
        json!(sha256_file(Path::new("testdata/valid_assembly.fa")))
    );
    assert_eq!(
        report["machine_summary"]["routing_hints"][0]["condition"],
        json!("submission_readiness_warning")
    );
}

#[test]
fn valid_report_includes_plot_contract() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "valid_plots");

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
    .code(1)
    .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    let histogram = report["plots"]["length_histogram"].as_array().unwrap();
    assert!(!histogram.is_empty(), "{report}");
    assert_eq!(histogram[0]["min_length"], json!(15));
    assert_eq!(histogram[0]["sequence_count"], json!(1));

    let points = report["plots"]["gc_length_plot"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(points[0]["length"], json!(16));
    assert!(points[0]["flags"].as_array().unwrap().is_empty());

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Length Histogram"), "{html}");
    assert!(html.contains("GC vs Length"), "{html}");
    assert!(html.contains("<svg"), "{html}");
}

#[test]
fn gc_outlier_plot_flags_are_backed_by_warning_finding() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("gc_outlier.fa");
    std::fs::write(
        &input,
        [
            ">balanced_1\nAAAACCCC\n",
            ">balanced_2\nTTTTGGGG\n",
            ">balanced_3\nAAAAGGGG\n",
            ">balanced_4\nTTTTCCCC\n",
            ">balanced_5\nAACCGGTT\n",
            ">balanced_6\nAAGGCCTT\n",
            ">balanced_7\nACGTACGT\n",
            ">balanced_8\nAGCTAGCT\n",
            ">balanced_9\nATGCCGTA\n",
            ">balanced_10\nTACGGCAT\n",
            ">high_gc\nGGGGGGGG\n",
        ]
        .concat(),
    )
    .unwrap();
    let outputs = output_paths(&temp_dir, "gc_outlier");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .arg("--min-contig-length")
        .arg("1")
        .arg("--out")
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(1)
        .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "gc_outliers"
    ));
    let high_gc = report["plots"]["gc_length_plot"]
        .as_array()
        .unwrap()
        .iter()
        .find(|point| point["id"] == json!("high_gc"))
        .unwrap();
    assert!(array_contains_string(&high_gc["flags"], "gc_outlier"));
    assert!(high_gc["gc_zscore"].as_f64().unwrap() >= 3.0);
}

#[test]
fn assembly_outliers_are_promoted_to_findings_without_fail_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("assembly_outliers.fa");
    let mut fasta = String::new();
    for (index, length) in [
        900, 940, 980, 1_000, 1_020, 1_040, 1_060, 1_080, 1_100, 1_120, 1_140,
    ]
    .into_iter()
    .enumerate()
    {
        fasta.push_str(&format!(
            ">normal_{}\n{}\n",
            index + 1,
            balanced_sequence(length)
        ));
    }
    fasta.push_str(&format!(">long_high_gc\n{}\n", "G".repeat(10_000)));
    std::fs::write(&input, fasta).unwrap();
    let outputs = output_paths(&temp_dir, "assembly_outliers");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .arg("--min-contig-length")
        .arg("1")
        .arg("--out")
        .arg(&outputs.html)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc)
        .assert()
        .code(1)
        .stderr(predicate::str::is_empty());

    let report = read_json(&outputs.json);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "gc_outliers"
    ));
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "length_outliers"
    ));
    assert!(array_contains_string(
        &report["machine_summary"]["top_findings"],
        "composite_anomalies"
    ));
    assert_routing_hint(
        &report,
        "composition_anomaly",
        "contamination_or_cobiont_triage",
        true,
    );
    assert_routing_hint(&report, "length_outlier", "record_length_review", false);

    assert_finding_taxonomy(&report, "gc_outliers", "composition", "moderate", true);
    assert_finding_taxonomy(&report, "length_outliers", "structure", "moderate", false);
    assert_finding_taxonomy(
        &report,
        "composite_anomalies",
        "composition",
        "moderate",
        true,
    );

    let gc_outliers = finding_by_id(&report, "gc_outliers");
    assert_eq!(gc_outliers["evidence"]["truncated"], json!(false));
    assert_eq!(
        gc_outliers["evidence"]["records"][0]["id"],
        json!("long_high_gc")
    );
    assert_eq!(
        gc_outliers["evidence"]["records"][0]["gc_percent"],
        json!(100.0)
    );
    assert!(gc_outliers["evidence"]["records"][0]["n_fraction"].is_number());
    assert!(gc_outliers["evidence"]["records"][0]["n_percent"].is_number());
    assert!(gc_outliers["evidence"]["records"][0]["gc_zscore"].is_number());

    let length_outliers = finding_by_id(&report, "length_outliers");
    assert_eq!(
        length_outliers["evidence"]["records"][0]["id"],
        json!("long_high_gc")
    );
    assert_eq!(
        length_outliers["evidence"]["records"][0]["length"],
        json!(10_000)
    );
    assert!(length_outliers["evidence"]["records"][0]["gc_percent"].is_number());
    assert!(length_outliers["evidence"]["records"][0]["n_fraction"].is_number());

    let composite_anomalies = finding_by_id(&report, "composite_anomalies");
    let composite_record = &composite_anomalies["evidence"]["records"][0];
    assert_eq!(composite_record["id"], json!("long_high_gc"));
    assert!(array_contains_string(
        &composite_record["signals"],
        "gc_outlier"
    ));
    assert!(array_contains_string(
        &composite_record["signals"],
        "length_outlier"
    ));
}

#[test]
fn valid_assembly_json_matches_golden_contract() {
    let paths = golden_output_paths("valid_assembly");
    let provenance_command = golden_provenance_command("valid_assembly");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_golden_provenance(&mut cmd, provenance_command);
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
    .code(1)
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
fn pipeline_gate_report_lists_blocking_and_advisory_findings() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "pipeline_gate");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/problem_assembly.fa",
        "--gate",
        "pipeline",
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
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.4.0"));
    assert_eq!(report["gate"]["mode"], json!("pipeline"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "duplicate_ids"
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "invalid_chars"
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "high_n_rate"
    ));
    assert!(array_contains_string(
        &report["gate"]["advisory_findings"],
        "gap_runs"
    ));
    assert!(array_contains_string(
        &report["gate"]["fail_on"],
        "invalid_fasta_structure"
    ));
    assert_eq!(
        report["provenance"]["input_sha256"],
        json!(sha256_file(Path::new("testdata/problem_assembly.fa")))
    );
}

#[test]
fn report_includes_readiness_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let json = temp.path().join("report.json");
    let html = temp.path().join("report.html");
    let tsv = temp.path().join("report.tsv");
    let multiqc = temp.path().join("report_mqc.json");

    Command::cargo_bin("fastaguard")
        .unwrap()
        .args([
            "testdata/problem_assembly.fa",
            "--gate",
            "pipeline",
            "--json",
            json.to_str().unwrap(),
            "--out",
            html.to_str().unwrap(),
            "--tsv",
            tsv.to_str().unwrap(),
            "--multiqc",
            multiqc.to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(report["readiness"]["overall"]["status"], "FAIL");
    assert!(report["readiness"]["categories"]
        .as_array()
        .unwrap()
        .iter()
        .any(|category| { category["id"] == "index" && category["status"] == "FAIL" }));
    assert!(std::fs::read_to_string(html).unwrap().contains("Readiness"));
    assert!(std::fs::read_to_string(tsv)
        .unwrap()
        .contains("readiness_status\tFAIL"));
}

#[test]
fn html_report_shows_gate_decision() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "html_gate");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/problem_assembly.fa",
        "--gate",
        "pipeline",
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
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Gate Decision"), "{html}");
    assert!(html.contains("Blocking"), "{html}");
    assert!(html.contains("Advisory"), "{html}");
}

#[test]
fn gate_none_report_preserves_warning_behavior_and_checksum() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "gate_none");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/problem_assembly.fa",
        "--gate",
        "none",
        "--fail-on",
        "duplicate_ids,invalid_chars",
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
    .code(2)
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("none"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "duplicate_ids"
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "invalid_chars"
    ));
    assert!(array_contains_string(
        &report["gate"]["advisory_findings"],
        "high_n_rate"
    ));
    assert_eq!(
        report["gate"]["fail_on"],
        json!(["duplicate_ids", "invalid_chars"])
    );
    assert_eq!(
        report["provenance"]["input_sha256"],
        json!(sha256_file(Path::new("testdata/problem_assembly.fa")))
    );
}

#[test]
fn problem_assembly_json_matches_golden_contract() {
    let paths = golden_output_paths("problem_assembly");
    let provenance_command = golden_provenance_command("problem_assembly");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_golden_provenance(&mut cmd, provenance_command);
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
fn problem_report_includes_v0_2_finding_taxonomy() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "problem_taxonomy");

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
    assert_finding_taxonomy(&report, "duplicate_ids", "duplication", "high", false);
    assert_finding_taxonomy(&report, "invalid_chars", "validity", "high", false);
    assert_finding_taxonomy(&report, "high_n_rate", "composition", "high", false);
    assert_finding_taxonomy(&report, "tiny_contigs", "structure", "moderate", false);
    assert_finding_taxonomy(&report, "gap_runs", "structure", "high", false);
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
    let provenance_command = golden_provenance_command("invalid_empty_record");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_golden_provenance(&mut cmd, provenance_command);
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

#[test]
fn unknown_gate_value_is_cli_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/valid_assembly.fa", "--gate", "strict"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'strict'"));
}

#[test]
fn invalid_provenance_timestamp_override_is_tool_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.env("FASTAGUARD_PROVENANCE_TIMESTAMP", "now")
        .arg("testdata/valid_assembly.fa")
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "FASTAGUARD_PROVENANCE_TIMESTAMP must be a valid RFC3339 date-time",
        ));
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

fn with_golden_provenance(cmd: &mut Command, command: &str) {
    // Fixture-only deterministic provenance; not intended as security-grade audit data.
    cmd.env("FASTAGUARD_PROVENANCE_COMMAND", command).env(
        "FASTAGUARD_PROVENANCE_TIMESTAMP",
        GOLDEN_PROVENANCE_TIMESTAMP,
    );
}

fn with_compare_golden_provenance(cmd: &mut Command, command: &str) {
    cmd.env("FASTAGUARD_PROVENANCE_COMMAND", command).env(
        "FASTAGUARD_PROVENANCE_TIMESTAMP",
        COMPARE_GOLDEN_PROVENANCE_TIMESTAMP,
    );
}

fn golden_provenance_command(stem: &str) -> &'static str {
    match stem {
        "valid_assembly" => {
            "fastaguard testdata/valid_assembly.fa --min-contig-length 1 --out target/fastaguard-golden-runtime/valid_assembly.html --json target/fastaguard-golden-runtime/valid_assembly.json --tsv target/fastaguard-golden-runtime/valid_assembly.tsv --multiqc target/fastaguard-golden-runtime/valid_assembly_multiqc.json"
        }
        "problem_assembly" => {
            "fastaguard testdata/problem_assembly.fa --out target/fastaguard-golden-runtime/problem_assembly.html --json target/fastaguard-golden-runtime/problem_assembly.json --tsv target/fastaguard-golden-runtime/problem_assembly.tsv --multiqc target/fastaguard-golden-runtime/problem_assembly_multiqc.json"
        }
        "invalid_empty_record" => {
            "fastaguard testdata/invalid_empty_record.fa --out target/fastaguard-golden-runtime/invalid_empty_record.html --json target/fastaguard-golden-runtime/invalid_empty_record.json --tsv target/fastaguard-golden-runtime/invalid_empty_record.tsv --multiqc target/fastaguard-golden-runtime/invalid_empty_record_multiqc.json"
        }
        _ => "fastaguard",
    }
}

fn compare_golden_provenance_command(stem: &str) -> &'static str {
    match stem {
        "compare_mixed_status" => {
            "fastaguard compare testdata/valid_assembly.fa testdata/problem_assembly.fa --gate pipeline --json target/fastaguard-golden-runtime/compare_mixed_status.json --out target/fastaguard-golden-runtime/compare_mixed_status.html --tsv target/fastaguard-golden-runtime/compare_mixed_status.tsv --multiqc target/fastaguard-golden-runtime/compare_mixed_status_multiqc.json"
        }
        "compare_all_pass" => {
            "fastaguard compare target/fastaguard-golden-runtime/clean_alpha.fa target/fastaguard-golden-runtime/clean_beta.fa --json target/fastaguard-golden-runtime/compare_all_pass.json --out target/fastaguard-golden-runtime/compare_all_pass.html --tsv target/fastaguard-golden-runtime/compare_all_pass.tsv --multiqc target/fastaguard-golden-runtime/compare_all_pass_multiqc.json"
        }
        _ => "fastaguard compare",
    }
}

fn write_compare_all_pass_inputs() -> (PathBuf, PathBuf) {
    let dir = Path::new("target").join("fastaguard-golden-runtime");
    std::fs::create_dir_all(&dir).unwrap();
    let first = dir.join("clean_alpha.fa");
    let second = dir.join("clean_beta.fa");
    std::fs::write(&first, format!(">alpha_contig\n{}\n", "ACGT".repeat(60))).unwrap();
    std::fs::write(
        &second,
        format!(">beta_contig\n{}\n", "AACCGGTT".repeat(30)),
    )
    .unwrap();
    (first, second)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn sha256_file(path: &Path) -> String {
    let mut hasher = Sha256::new();
    let bytes = std::fs::read(path).unwrap();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn balanced_sequence(length: usize) -> String {
    "ACGT"
        .repeat(length.div_ceil(4))
        .chars()
        .take(length)
        .collect()
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

fn assert_finding_taxonomy(
    report: &Value,
    id: &str,
    category: &str,
    confidence: &str,
    requires_followup_tool: bool,
) {
    let finding = finding_by_id(report, id);
    assert_eq!(finding["category"], json!(category));
    assert_eq!(finding["confidence"], json!(confidence));
    assert_eq!(
        finding["requires_followup_tool"],
        json!(requires_followup_tool)
    );
}

fn assert_routing_hint(
    report: &Value,
    condition: &str,
    suggested_route: &str,
    requires_external_database: bool,
) {
    let hints = report["machine_summary"]["routing_hints"]
        .as_array()
        .unwrap();
    assert!(
        hints.iter().any(|hint| {
            hint["condition"] == json!(condition)
                && hint["suggested_route"] == json!(suggested_route)
                && hint["requires_external_database"] == json!(requires_external_database)
        }),
        "missing routing hint {condition}/{suggested_route}: {hints:?}"
    );
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
