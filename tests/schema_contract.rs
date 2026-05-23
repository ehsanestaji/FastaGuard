use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn committed_reports_validate_against_json_schema() {
    let schema = read_json(Path::new("schema/fastaguard.schema.json"));
    let validator =
        jsonschema::validator_for(&schema).expect("schema/fastaguard.schema.json should compile");

    for path in golden_report_paths() {
        let report = normalize_legacy_report_for_current_schema(read_json(path));
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
        Path::new("examples/reports/assembly_pass/fastaguard.json"),
        Path::new("examples/reports/assembly_fail/fastaguard.json"),
    ]
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap())
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn normalize_legacy_report_for_current_schema(mut report: Value) -> Value {
    if let Some(findings) = report["findings"].as_array_mut() {
        for finding in findings {
            if finding["category"].is_null() {
                finding["category"] = serde_json::json!("validity");
            }
            if finding["confidence"].is_null() {
                finding["confidence"] = serde_json::json!("high");
            }
            if finding["requires_followup_tool"].is_null() {
                finding["requires_followup_tool"] = serde_json::json!(false);
            }
        }
    }

    report
}

fn balanced_sequence(length: usize) -> String {
    "ACGT"
        .repeat(length.div_ceil(4))
        .chars()
        .take(length)
        .collect()
}
