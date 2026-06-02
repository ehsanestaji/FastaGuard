use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn committed_reports_validate_against_json_schema() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let validator =
        jsonschema::validator_for(&schema).expect("schema/fastaguard.schema.json should compile");

    for path in golden_report_paths() {
        let report = read_json(path);
        let errors = validator
            .iter_errors(&report)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();

        assert!(
            errors.is_empty(),
            "{} did not validate against schema/fastaguard.schema.json:\n{}",
            path.display(),
            errors.join("\n")
        );
    }
}

#[test]
fn schema_requires_emitted_finding_taxonomy_fields() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let required = schema["$defs"]["finding"]["required"].as_array().unwrap();

    assert!(required.contains(&serde_json::json!("category")));
    assert!(required.contains(&serde_json::json!("confidence")));
    assert!(required.contains(&serde_json::json!("requires_followup_tool")));
}

#[test]
fn schema_requires_gate_and_input_sha256() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let single_report = &schema["$defs"]["single_report"];
    let report_required = single_report["required"].as_array().unwrap();
    let gate_required = single_report["properties"]["gate"]["required"]
        .as_array()
        .unwrap();
    let provenance_required = single_report["properties"]["provenance"]["required"]
        .as_array()
        .unwrap();

    assert_eq!(
        single_report["properties"]["schema_version"]["const"],
        "0.4.0"
    );
    assert!(report_required.contains(&serde_json::json!("gate")));
    assert!(gate_required.contains(&serde_json::json!("mode")));
    assert!(gate_required.contains(&serde_json::json!("status")));
    assert!(gate_required.contains(&serde_json::json!("blocking_findings")));
    assert!(gate_required.contains(&serde_json::json!("advisory_findings")));
    assert!(gate_required.contains(&serde_json::json!("fail_on")));
    assert!(provenance_required.contains(&serde_json::json!("input_sha256")));
    assert_eq!(
        single_report["properties"]["provenance"]["properties"]["input_sha256"]["pattern"],
        "^[a-f0-9]{64}$"
    );
}

#[test]
fn schema_requires_readiness_for_single_reports() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();
    let single_report = &schema["$defs"]["single_report"];

    assert!(single_report["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "readiness"));
    assert_eq!(
        single_report["properties"]["schema_version"]["const"],
        "0.4.0"
    );
}

#[test]
fn schema_supports_compare_reports() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();
    let compare_report = &schema["$defs"]["compare_report"];
    let compare_sample = &schema["$defs"]["compare_sample"];

    assert!(compare_report.is_object(), "{schema}");
    assert!(compare_report["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "samples"));
    assert!(compare_sample["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "readiness_categories"));
    assert_eq!(
        compare_sample["properties"]["readiness_categories"]["items"]["$ref"],
        "#/$defs/readiness_category"
    );
}

#[test]
fn schema_constrains_compare_counts_and_cohort_evidence() {
    let schema: serde_json::Value =
        serde_json::from_str(fastaguard::contract::schema_json()).unwrap();

    assert_eq!(
        schema["$defs"]["compare_input_info"]["properties"]["sample_count"]["minimum"],
        2
    );
    assert_eq!(
        schema["$defs"]["compare_summary"]["properties"]["sample_count"]["minimum"],
        2
    );
    assert_eq!(schema["$defs"]["cohort_finding_evidence"]["type"], "object");
    assert!(schema["$defs"]["cohort_finding_evidence"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "records"));
}

#[test]
fn schema_validates_compare_report_with_cohort_finding() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let validator =
        jsonschema::validator_for(&schema).expect("schema/fastaguard.schema.json should compile");
    let mut report = read_json(Path::new("tests/golden/compare_all_pass.json"));
    report["cohort_findings"] = serde_json::json!([
        {
            "id": "cohort_total_length_outliers",
            "severity": "minor",
            "affected_count": 1,
            "evidence": {
                "records": [
                    {
                        "sample_id": "clean_beta",
                        "total_length": 240,
                        "reason": "total length is unusual relative to the cohort"
                    }
                ]
            }
        }
    ]);

    let errors = validator
        .iter_errors(&report)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    assert!(
        errors.is_empty(),
        "compare report with cohort finding did not validate:\n{}",
        errors.join("\n")
    );
}

#[test]
fn freshly_generated_outlier_report_validates_against_json_schema() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("outliers.fa");
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

    let html = temp_dir.path().join("outliers.html");
    let json = temp_dir.path().join("outliers.json");
    let tsv = temp_dir.path().join("outliers.tsv");
    let multiqc = temp_dir.path().join("outliers_multiqc.json");
    let mut cmd = assert_cmd::Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .arg("--min-contig-length")
        .arg("1")
        .arg("--out")
        .arg(&html)
        .arg("--json")
        .arg(&json)
        .arg("--tsv")
        .arg(&tsv)
        .arg("--multiqc")
        .arg(&multiqc)
        .assert()
        .code(1);

    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let report = read_json(&json);
    let validator =
        jsonschema::validator_for(&schema).expect("schema/fastaguard.schema.json should compile");
    let errors = validator
        .iter_errors(&report)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    assert!(
        errors.is_empty(),
        "fresh outlier report did not validate:\n{}",
        errors.join("\n")
    );
}

fn golden_report_paths() -> Vec<&'static Path> {
    vec![
        Path::new("tests/golden/valid_assembly.json"),
        Path::new("tests/golden/problem_assembly.json"),
        Path::new("tests/golden/invalid_empty_record.json"),
        Path::new("tests/golden/compare_mixed_status.json"),
        Path::new("tests/golden/compare_all_pass.json"),
        Path::new("examples/reports/assembly_pass/fastaguard.json"),
        Path::new("examples/reports/assembly_fail/fastaguard.json"),
    ]
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap())
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn balanced_sequence(length: usize) -> String {
    "ACGT"
        .repeat(length.div_ceil(4))
        .chars()
        .take(length)
        .collect()
}
