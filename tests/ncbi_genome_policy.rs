use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const EXPECTED_KEYS: [&str; 6] = [
    "description",
    "expect_can_continue",
    "expect_findings",
    "fixture",
    "id",
    "source_scope",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyCase {
    id: String,
    fixture: String,
    expect_findings: Vec<String>,
    expect_can_continue: bool,
    source_scope: String,
    description: String,
}

#[test]
fn ncbi_genome_policy_corpus_matches_manifest() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/ncbi_genome");
    let manifest_path = corpus_dir.join("policy_cases.json");
    let manifest_text = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let manifest_value: Value = serde_json::from_str(&manifest_text)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", manifest_path.display()));
    let entries = manifest_value
        .as_array()
        .expect("policy_cases.json must contain a JSON array");
    assert!(!entries.is_empty(), "policy_cases.json must not be empty");

    let expected_keys = EXPECTED_KEYS.into_iter().collect::<BTreeSet<_>>();
    for entry in entries {
        let object = entry
            .as_object()
            .expect("each policy_cases.json entry must be an object");
        let actual_keys = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
        assert_eq!(
            actual_keys, expected_keys,
            "unexpected manifest object keys"
        );
    }

    let cases: Vec<PolicyCase> = serde_json::from_value(manifest_value)
        .expect("policy_cases.json entries must match the policy case contract");
    let ids = cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<Vec<_>>();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids, "policy cases must be sorted by id");
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>().len(),
        cases.len(),
        "policy case ids must be unique"
    );

    let manifest_fixtures = cases
        .iter()
        .map(|case| case.fixture.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_fixtures.len(),
        cases.len(),
        "each fixture must appear exactly once in policy_cases.json"
    );
    let corpus_fixtures = fs::read_dir(&corpus_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", corpus_dir.display()))
        .map(|entry| entry.expect("failed to inspect corpus directory").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "fa"))
        .map(|path| {
            path.file_name()
                .expect("fixture path must have a file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_fixtures,
        corpus_fixtures.iter().map(String::as_str).collect(),
        "policy_cases.json must cover every FASTA fixture exactly once"
    );
    for fixture in &corpus_fixtures {
        let bytes = fs::read(corpus_dir.join(fixture))
            .unwrap_or_else(|error| panic!("failed to read fixture {fixture}: {error}"));
        if fixture == "seqid_unicode.fa" {
            assert!(
                !bytes.is_ascii(),
                "seqid_unicode.fa must exercise a non-ASCII SeqID"
            );
        } else {
            assert!(bytes.is_ascii(), "{fixture} must contain only ASCII bytes");
        }
    }

    let output_root = TempDir::new().expect("failed to create temporary output directory");
    for case in cases {
        assert!(
            matches!(
                case.source_scope.as_str(),
                "table2asn_fasta_overlap" | "fastaguard_structural_extension"
            ),
            "{} has unsupported source_scope {}",
            case.id,
            case.source_scope
        );
        assert!(
            !case.description.trim().is_empty(),
            "{} must have a non-empty description",
            case.id
        );
        let mut sorted_findings = case.expect_findings.clone();
        sorted_findings.sort();
        assert_eq!(
            case.expect_findings, sorted_findings,
            "{} expect_findings must be sorted",
            case.id
        );
        if case.fixture == "seqid_unicode.fa" {
            assert_eq!(
                case.expect_findings,
                ["ncbi_genome_seqid"],
                "the Unicode fixture must exercise NCBI SeqID rejection"
            );
            assert!(
                !case.expect_can_continue,
                "the Unicode SeqID case must block continuation"
            );
        }

        let case_output_dir = output_root.path().join(&case.id);
        fs::create_dir(&case_output_dir).unwrap_or_else(|error| {
            panic!(
                "failed to create output directory {}: {error}",
                case_output_dir.display()
            )
        });
        let json_path = case_output_dir.join("report.json");
        let html_path = case_output_dir.join("report.html");
        let tsv_path = case_output_dir.join("report.tsv");
        let multiqc_path = case_output_dir.join("report_mqc.json");
        let output = run_case(
            &corpus_dir.join(&case.fixture),
            &json_path,
            &html_path,
            &tsv_path,
            &multiqc_path,
        );
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
        let mut actual_findings = report["findings"]
            .as_array()
            .expect("report findings must be an array")
            .iter()
            .map(|finding| {
                finding["id"]
                    .as_str()
                    .expect("finding id must be a string")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        actual_findings.sort();

        assert_eq!(
            actual_findings, case.expect_findings,
            "{} findings",
            case.id
        );
        assert_eq!(
            report["gate"]["can_continue"], case.expect_can_continue,
            "{} continuation decision",
            case.id
        );
        assert_eq!(
            report["provenance"]["submission_policy"]["id"], "ncbi_genome",
            "{} policy provenance",
            case.id
        );
    }
}

fn run_case(
    input: &Path,
    json_path: &Path,
    html_path: &Path,
    tsv_path: &Path,
    multiqc_path: &Path,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fastaguard"));
    command
        .arg(input)
        .args(["--gate", "submission", "--submission-target", "ncbi"])
        .arg("--json")
        .arg(json_path)
        .arg("--out")
        .arg(html_path)
        .arg("--tsv")
        .arg(tsv_path)
        .arg("--multiqc")
        .arg(multiqc_path);
    command.output().expect("failed to execute fastaguard")
}
