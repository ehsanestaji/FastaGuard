use serde_json::Value;
use std::path::Path;

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
