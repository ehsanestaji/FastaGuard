use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanCase {
    id: String,
    fixture: String,
    expected_ncbi_blocking_findings: Vec<String>,
}

#[test]
fn clean_corpus_has_no_ncbi_submission_blockers() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/clean_corpus");
    let manifest_path = corpus_dir.join("clean_cases.json");
    let manifest: Vec<CleanCase> = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display())),
    )
    .unwrap_or_else(|error| panic!("failed to parse {}: {error}", manifest_path.display()));

    assert!(!manifest.is_empty(), "clean_cases.json must not be empty");
    let ids = manifest
        .iter()
        .map(|case| case.id.as_str())
        .collect::<Vec<_>>();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids, "clean case ids must be sorted");
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>().len(),
        manifest.len(),
        "clean case ids must be unique"
    );

    let manifest_fixtures = manifest
        .iter()
        .map(|case| case.fixture.as_str())
        .collect::<BTreeSet<_>>();
    let corpus_fixtures = fs::read_dir(&corpus_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", corpus_dir.display()))
        .map(|entry| entry.expect("failed to inspect clean corpus").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "fa"))
        .map(|path| {
            path.file_name()
                .expect("fixture must have a file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_fixtures,
        corpus_fixtures.iter().map(String::as_str).collect(),
        "clean_cases.json must cover every clean FASTA fixture exactly once"
    );

    let output_root = TempDir::new().expect("failed to create output directory");
    for case in manifest {
        assert!(
            case.expected_ncbi_blocking_findings.is_empty(),
            "{} is not a clean-corpus case",
            case.id
        );
        let case_dir = output_root.path().join(&case.id);
        fs::create_dir(&case_dir).expect("failed to create case output directory");
        let json_path = case_dir.join("report.fastaguard.json");
        let output = Command::new(env!("CARGO_BIN_EXE_fastaguard"))
            .arg(corpus_dir.join(&case.fixture))
            .args(["--gate", "submission", "--submission-target", "ncbi"])
            .arg("--outdir")
            .arg(&case_dir)
            .args(["--prefix", "report"])
            .output()
            .expect("failed to execute fastaguard");
        assert!(
            output.status.success(),
            "{} exited with {}\nstdout:\n{}\nstderr:\n{}",
            case.id,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let report: Value = serde_json::from_slice(
            &fs::read(&json_path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", json_path.display())),
        )
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", json_path.display()));
        assert_eq!(
            report["gate"]["blocking_findings"],
            serde_json::json!(case.expected_ncbi_blocking_findings),
            "{} NCBI blockers",
            case.id
        );
        assert_eq!(report["gate"]["can_continue"], true, "{} gate", case.id);
    }
}
