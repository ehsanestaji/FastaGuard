use assert_cmd::Command;
use noodles::{bam, bcf, cram, sam, vcf};
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
fn help_describes_gate_options_as_report_policy() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Gate preset for report blocking policy",
        ))
        .stdout(predicate::str::contains("mark the report as FAIL"))
        .stdout(predicate::str::contains("failure behavior").not())
        .stdout(predicate::str::contains("fail the run").not());
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
        .stdout(predicate::str::contains(r#""schema_version": "0.7.0""#))
        .stdout(predicate::str::contains(r#""catalog_version": "0.7.0""#))
        .stdout(predicate::str::contains(r#""duplicate_ids""#))
        .stdout(predicate::str::contains(r#""invalid_fasta_structure""#))
        .stdout(predicate::str::contains(r#""gc_outliers""#))
        .stdout(predicate::str::contains(r#""length_outliers""#))
        .stdout(predicate::str::contains(r#""composite_anomalies""#))
        .stdout(predicate::str::contains(
            r#""id": "cohort_total_length_outliers""#,
        ))
        .stdout(predicate::str::contains(r#""id": "cohort_gc_outliers""#))
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
    for id in [
        "gc_outliers",
        "length_outliers",
        "composite_anomalies",
        "cohort_total_length_outliers",
        "cohort_gc_outliers",
    ] {
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
fn reference_requires_a_canonical_fasta() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Usage: fastaguard reference"));
}

#[test]
fn reference_writes_a_coordinate_policy_json_report() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1 example\nACGT\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.env(
        "FASTAGUARD_PROVENANCE_TIMESTAMP",
        GOLDEN_PROVENANCE_TIMESTAMP,
    )
    .arg("reference")
    .arg(&fasta)
    .args(["--format", "json", "--json"])
    .arg(&report_path)
    .assert()
    .success();

    let report = read_json(&report_path);
    assert_eq!(report["schema_version"], json!("1.0.0"));
    assert_eq!(report["report_type"], json!("reference"));
    assert_eq!(report["gate"]["mode"], json!("reference"));
    assert_eq!(
        report["gate"]["reference_policy"]["id"],
        json!("coordinate")
    );
    assert_eq!(
        report["canonical_reference"]["sequences"][0]["id"],
        json!("chr1")
    );
    assert_eq!(
        report["canonical_reference"]["sequences"][0]["length"],
        json!(4)
    );
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
}

#[test]
fn reference_matching_fai_is_exact() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let fai = temp_dir.path().join("reference.fa.fai");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(&fai, "chr1\t4\t6\t4\t5\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--fai")
        .arg(&fai)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("PASS"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(true)
    );
    assert_eq!(report["comparisons"][0]["kind"], json!("fai"));
    assert_eq!(report["comparisons"][0]["relationship"], json!("exact"));
}

#[test]
fn reference_rejects_duplicate_names_in_a_required_fai() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let fai = temp_dir.path().join("reference.fa.fai");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(&fai, "chr1\t4\t6\t4\t5\nchr1\t4\t6\t4\t5\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--fai")
        .arg(&fai)
        .args(["--require", "fai", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("required_fai_invalid")));
}

#[test]
fn reference_matching_dictionary_md5_is_exact() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let dictionary = temp_dir.path().join("reference.dict");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &dictionary,
        "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:4\tM5:f1f8f4bf413b16ad135722aa4591043e\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--dict")
        .arg(&dictionary)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("PASS"));
    assert_eq!(report["comparisons"][0]["kind"], json!("dict"));
    assert_eq!(report["comparisons"][0]["relationship"], json!("exact"));
}

#[test]
fn reference_matching_bam_header_is_coordinate_compatible() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let bam_path = temp_dir.path().join("sample.bam");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    let header: sam::Header = "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:4\n".parse().unwrap();
    let mut writer = bam::io::Writer::new(std::fs::File::create(&bam_path).unwrap());
    writer.write_header(&header).unwrap();
    writer.try_finish().unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--alignment")
        .arg(&bam_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(report["gate"]["can_continue"], json!(true));
    assert_eq!(report["comparisons"][0]["kind"], json!("alignment"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("coordinate_compatible")
    );
    assert!(report["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("header_asserted")));
}

#[test]
fn reference_matching_cram_header_is_exact_when_md5_is_available() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let cram_path = temp_dir.path().join("sample.cram");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    let header: sam::Header = concat!(
        "@HD\tVN:1.6\n",
        "@SQ\tSN:chr1\tLN:4\tM5:f1f8f4bf413b16ad135722aa4591043e\n",
    )
    .parse()
    .unwrap();
    let mut writer = cram::io::Writer::new(std::fs::File::create(&cram_path).unwrap());
    writer.write_header(&header).unwrap();
    writer.try_finish(&header).unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--alignment")
        .arg(&cram_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("PASS"));
    assert_eq!(report["comparisons"][0]["kind"], json!("alignment"));
    assert_eq!(report["comparisons"][0]["relationship"], json!("exact"));
    assert!(report["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("content_verified")));
}

#[test]
fn reference_matching_vcf_header_subset_is_compatible_without_reading_records() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("sites.vcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=4>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
            "this record is deliberately not valid VCF\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(report["comparisons"][0]["kind"], json!("variants"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("subset_compatible")
    );
    assert!(report["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("header_asserted")));
}

#[test]
fn reference_matching_bcf_header_subset_is_compatible() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let bcf_path = temp_dir.path().join("sites.bcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    let header: vcf::Header = concat!(
        "##fileformat=VCFv4.3\n",
        "##contig=<ID=chr1,length=4>\n",
        "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
    )
    .parse()
    .unwrap();
    let mut writer = bcf::io::Writer::new(std::fs::File::create(&bcf_path).unwrap());
    writer.write_header(&header).unwrap();
    writer.try_finish().unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&bcf_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("subset_compatible")
    );
}

#[test]
fn reference_advisory_policy_reports_an_incompatible_variant_without_blocking_continuation() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("mismatch.vcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=5>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--policy", "advisory", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert_eq!(report["gate"]["can_continue"], json!(true));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
}

#[test]
fn reference_advisory_mismatch_has_a_stable_finding_and_provenance() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("mismatch.vcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=5>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--policy", "advisory", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(
        report["findings"][0]["id"],
        json!("reference_length_mismatch")
    );
    assert_eq!(report["findings"][0]["original_name"], json!("chr1"));
    assert_eq!(report["findings"][0]["resolved_name"], json!("chr1"));
    assert_eq!(report["findings"][0]["expected_value"], json!("4"));
    assert_eq!(report["findings"][0]["observed_value"], json!("5"));
    assert_eq!(
        report["findings"][0]["examples"][0]["original_name"],
        json!("chr1")
    );
    assert_eq!(
        report["gate"]["advisory_findings"],
        json!(["reference_length_mismatch"])
    );
    assert_eq!(
        report["readiness"]["categories"][0]["id"],
        json!("reference_compatibility")
    );
    assert_eq!(
        report["provenance"]["companions"][0]["path"],
        json!(vcf_path.display().to_string())
    );
}

#[test]
fn reference_reports_surface_findings_without_putting_paths_in_multiqc() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("mismatch.vcf");
    let output_dir = temp_dir.path().join("reports");
    let html_path = output_dir.join("contract.fastaguard.html");
    let tsv_path = output_dir.join("contract.fastaguard.tsv");
    let multiqc_path = output_dir.join("contract.fastaguard_mqc.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=5>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--policy", "advisory", "--format", "html,tsv,multiqc"])
        .arg("--outdir")
        .arg(&output_dir)
        .args(["--prefix", "contract"])
        .assert()
        .success();

    let html = std::fs::read_to_string(&html_path).unwrap();
    assert!(html.contains("reference_length_mismatch"), "{html}");
    assert!(html.contains("Use a companion generated"), "{html}");

    let tsv = std::fs::read_to_string(&tsv_path).unwrap();
    assert!(
        tsv.contains("finding\tvariants\t\t\treference_length_mismatch"),
        "{tsv}"
    );
    assert!(tsv.contains("Use a companion generated"), "{tsv}");

    let multiqc = std::fs::read_to_string(&multiqc_path).unwrap();
    assert!(
        !multiqc.contains(&vcf_path.display().to_string()),
        "{multiqc}"
    );
}

#[test]
fn reference_required_vcf_without_contig_declarations_is_a_reported_failure() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("sites.vcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &vcf_path,
        "##fileformat=VCFv4.3\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--require", "variants", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("required_variants_unusable")));
    let malformed = finding_by_id(&report, "reference_malformed_declaration");
    assert_eq!(malformed["artifact_kind"], json!("variants"));
}

#[test]
fn reference_matching_gff3_subset_is_coordinate_compatible() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let gff3_path = temp_dir.path().join("genes.gff3");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    std::fs::write(
        &gff3_path,
        concat!(
            "##gff-version 3\n",
            "##sequence-region chr1 1 4\n",
            "chr1\tFastaGuard\tgene\t1\t4\t.\t+\t.\tID=gene1\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--annotation")
        .arg(&gff3_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(report["comparisons"][0]["kind"], json!("annotation"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("subset_compatible")
    );
    assert!(report["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("metadata_verified")));
}

#[test]
fn reference_accepts_one_origin_crossing_gff3_feature_after_an_explicit_circular_landmark() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let gff3_path = temp_dir.path().join("circular.gff3");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chrM\nACGTACGTAC\n").unwrap();
    std::fs::write(
        &gff3_path,
        concat!(
            "##gff-version 3\n",
            "##sequence-region chrM 1 10\n",
            "chrM\tFastaGuard\tregion\t1\t10\t.\t+\t.\tIs_circular=true\n",
            "chrM\tFastaGuard\tgene\t8\t12\t.\t+\t.\tID=gene1\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--annotation")
        .arg(&gff3_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("coordinate_compatible")
    );
}

#[test]
fn reference_rejects_gtf_as_a_required_gff3_annotation() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let gtf_path = temp_dir.path().join("genes.gtf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &gtf_path,
        "chr1\tFastaGuard\tgene\t1\t4\t.\t+\t.\tgene_id \"gene1\";\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--annotation")
        .arg(&gtf_path)
        .args(["--require", "annotation", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("required_annotation_invalid")));
}

#[test]
fn reference_rejects_gff3_features_that_cross_a_non_circular_origin() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let gff3_path = temp_dir.path().join("linear.gff3");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGTACGTAC\n").unwrap();
    std::fs::write(
        &gff3_path,
        concat!(
            "##gff-version 3\n",
            "##sequence-region chr1 1 10\n",
            "chr1\tFastaGuard\tgene\t8\t12\t.\t+\t.\tID=gene1\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--annotation")
        .arg(&gff3_path)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("incompatible")
    );
}

#[test]
fn reference_reports_companion_declarations_in_the_manifest_and_report_family() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("sites.vcf");
    let json_path = temp_dir.path().join("reference.json");
    let tsv_path = temp_dir.path().join("reference.tsv");
    let multiqc_path = temp_dir.path().join("reference_mqc.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=4>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--format", "json,tsv,multiqc", "--json"])
        .arg(&json_path)
        .arg("--tsv")
        .arg(&tsv_path)
        .arg("--multiqc")
        .arg(&multiqc_path)
        .assert()
        .success();

    let report = read_json(&json_path);
    assert_eq!(
        report["reference_manifest"]["artifacts"][0]["kind"],
        json!("variants")
    );
    assert!(
        report["reference_manifest"]["artifacts"][0]["declaration_digest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert_eq!(
        report["comparisons"][0]["declaration_digest"],
        report["reference_manifest"]["artifacts"][0]["declaration_digest"]
    );
    assert_eq!(
        report["gate"]["reference_policy"]["version"],
        json!("1.0.0")
    );

    let tsv = std::fs::read_to_string(&tsv_path).unwrap();
    assert!(tsv.contains(vcf_path.to_str().unwrap()));
    assert!(tsv.contains("comparison\tvariants"));

    let multiqc = read_json(&multiqc_path);
    assert_eq!(multiqc["data"].as_object().unwrap().len(), 1);
    let summary = multiqc["data"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(summary["supplied_artifact_count"], json!(1));
    assert_eq!(summary["subset_compatible_count"], json!(1));
}

#[test]
fn reference_strict_policy_does_not_mislabel_a_valid_subset_as_missing_declarations() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let vcf_path = temp_dir.path().join("sites.vcf");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    std::fs::write(
        &vcf_path,
        concat!(
            "##fileformat=VCFv4.3\n",
            "##contig=<ID=chr1,length=4>\n",
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--variants")
        .arg(&vcf_path)
        .args(["--policy", "strict", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("subset_compatible")
    );
    assert_eq!(report["gate"]["can_continue"], json!(false));
    assert!(!report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("no_reference_declarations")));
}

#[test]
fn reference_reordered_dictionary_with_matching_md5_is_content_equivalent() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let dictionary = temp_dir.path().join("reference.dict");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n>chr2\nTGCA\n").unwrap();
    std::fs::write(
        &dictionary,
        concat!(
            "@HD\tVN:1.6\n",
            "@SQ\tSN:chr2\tLN:4\tM5:5c15f97a88433c48f8bf76745d9da437\n",
            "@SQ\tSN:chr1\tLN:4\tM5:f1f8f4bf413b16ad135722aa4591043e\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--dict")
        .arg(&dictionary)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(
        report["comparisons"][0]["relationship"],
        json!("content_equivalent")
    );
    assert_eq!(report["gate"]["can_continue"], json!(false));
    assert_eq!(
        report["gate"]["blocking_findings"],
        json!(["reference_declaration_mismatch"])
    );
}

#[test]
fn reference_alias_map_resolves_dictionary_names_without_changing_md5_evidence() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let dictionary = temp_dir.path().join("reference.dict");
    let aliases = temp_dir.path().join("aliases.tsv");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(
        &dictionary,
        "@HD\tVN:1.6\n@SQ\tSN:1\tLN:4\tM5:f1f8f4bf413b16ad135722aa4591043e\n",
    )
    .unwrap();
    std::fs::write(&aliases, "declared_name\treference_name\n1\tchr1\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--dict")
        .arg(&dictionary)
        .arg("--alias-map")
        .arg(&aliases)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("PASS"));
    assert_eq!(report["comparisons"][0]["relationship"], json!("exact"));
    assert!(report["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("explicit_alias_mapping")));
}

#[test]
fn reference_alias_map_resolves_fai_names_without_changing_layout_evidence() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let fai = temp_dir.path().join("reference.fa.fai");
    let aliases = temp_dir.path().join("aliases.tsv");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(&fai, "1\t4\t6\t4\t5\n").unwrap();
    std::fs::write(&aliases, "declared_name\treference_name\n1\tchr1\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--fai")
        .arg(&fai)
        .arg("--alias-map")
        .arg(&aliases)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("PASS"));
    assert_eq!(report["comparisons"][0]["relationship"], json!("exact"));
}

#[test]
fn reference_missing_required_dictionary_is_a_reported_failure() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .args(["--require", "dict", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("required_dict_missing")));
}

#[test]
fn reference_write_lock_emits_a_manifest_with_a_semantic_digest() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let report_path = temp_dir.path().join("reference.json");
    let lock_path = temp_dir.path().join("reference.lock.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .args(["--write-lock", "--lock"])
        .arg(&lock_path)
        .assert()
        .success();

    let lock = read_json(&lock_path);
    assert_eq!(lock["manifest_version"], json!("1.0.0"));
    assert_eq!(
        lock["canonical_reference"]["sequences"][0]["id"],
        json!("chr1")
    );
    assert!(lock["semantic_digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(lock["canonical_reference"]["physical_sha256"].is_null());

    let report = read_json(&report_path);
    assert_eq!(
        report["reference_manifest"]["manifest_version"],
        json!("1.0.0")
    );
    assert_eq!(
        report["reference_manifest"]["canonical_reference"]["physical_sha256"],
        report["canonical_reference"]["physical_sha256"]
    );
    assert_eq!(
        report["reference_manifest"]["semantic_digest"],
        lock["semantic_digest"]
    );
}

#[test]
fn reference_lock_digest_changes_when_the_policy_changes() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let fai = temp_dir.path().join("reference.fa.fai");
    let coordinate_report = temp_dir.path().join("coordinate.json");
    let coordinate_lock = temp_dir.path().join("coordinate.lock.json");
    let strict_report = temp_dir.path().join("strict.json");
    let strict_lock = temp_dir.path().join("strict.lock.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(&fai, "chr1\t4\t6\t4\t5\n").unwrap();

    for (policy, report, lock) in [
        ("coordinate", &coordinate_report, &coordinate_lock),
        ("strict", &strict_report, &strict_lock),
    ] {
        let mut cmd = Command::cargo_bin("fastaguard").unwrap();
        cmd.arg("reference")
            .arg(&fasta)
            .arg("--fai")
            .arg(&fai)
            .args(["--policy", policy, "--format", "json", "--json"])
            .arg(report)
            .args(["--write-lock", "--lock"])
            .arg(lock)
            .assert()
            .success();
    }

    assert_ne!(
        read_json(&coordinate_lock)["semantic_digest"],
        read_json(&strict_lock)["semantic_digest"]
    );
}

#[test]
fn reference_default_bundle_writes_html_tsv_and_one_multiqc_table() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let outdir = temp_dir.path().join("reports");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--outdir")
        .arg(&outdir)
        .assert()
        .success();

    let html = std::fs::read_to_string(outdir.join("reference.fastaguard.html")).unwrap();
    assert!(html.contains("FastaGuard Reference"));
    let tsv = std::fs::read_to_string(outdir.join("reference.fastaguard.tsv")).unwrap();
    assert!(tsv.starts_with("record_type\t"));
    let multiqc = read_json(&outdir.join("reference.fastaguard_mqc.json"));
    assert_eq!(multiqc["id"], json!("fastaguard_reference"));
    assert_eq!(multiqc["plot_type"], json!("table"));
    assert_eq!(multiqc["data"].as_object().unwrap().len(), 1);
    assert!(outdir.join("reference.fastaguard.json").exists());
}

#[test]
fn reference_bundle_collision_publishes_no_partial_reports() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let outdir = temp_dir.path().join("reports");
    std::fs::create_dir(&outdir).unwrap();
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(outdir.join("reference.fastaguard.json"), "existing report").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--outdir")
        .arg(&outdir)
        .assert()
        .code(3);

    assert!(!outdir.join("reference.fastaguard.html").exists());
    assert!(!outdir.join("reference.fastaguard.tsv").exists());
    assert!(!outdir.join("reference.fastaguard_mqc.json").exists());
    assert_eq!(
        std::fs::read_to_string(outdir.join("reference.fastaguard.json")).unwrap(),
        "existing report"
    );
}

#[test]
fn reference_normalises_equivalent_output_paths_before_publication() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let aliases = temp_dir.path().join("aliases");
    let json_path = temp_dir.path().join("./reference.json");
    let lock_path = aliases.join("../reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::create_dir(&aliases).unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .args(["--format", "json", "--json"])
        .arg(&json_path)
        .args(["--write-lock", "--lock"])
        .arg(&lock_path)
        .assert()
        .code(3)
        .stderr(predicate::str::contains("duplicate reference output path"));

    assert!(!json_path.exists());
}

#[test]
fn reference_malformed_required_fai_is_a_reported_failure() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let fai = temp_dir.path().join("reference.fa.fai");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGT\n").unwrap();
    std::fs::write(&fai, "not a valid FAI\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .arg("--fai")
        .arg(&fai)
        .args(["--require", "fai", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("required_fai_invalid")));
}

#[test]
fn reference_invalid_canonical_sequence_fails_even_with_advisory_policy() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGTX\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .args(["--policy", "advisory", "--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert!(report["verdict"]["reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("canonical_invalid_sequence_symbols")));
    assert_eq!(
        report["findings"][0]["id"],
        json!("reference_canonical_reference_invalid")
    );
    assert_eq!(
        report["gate"]["blocking_findings"],
        json!(["reference_canonical_reference_invalid"])
    );
    assert!(report["canonical_reference"]["sequences"][0]["sam_md5"].is_null());
    assert!(report["canonical_reference"]["sequences"][0]["refget_id"].is_null());
}

#[test]
fn reference_seqcol_identities_are_present_and_invariant_to_fasta_rewrapping() {
    let temp_dir = TempDir::new().unwrap();
    let first_fasta = temp_dir.path().join("first.fa");
    let second_fasta = temp_dir.path().join("second.fa");
    let first_report = temp_dir.path().join("first.json");
    let second_report = temp_dir.path().join("second.json");
    std::fs::write(&first_fasta, ">chr1\nACGTACGT\n").unwrap();
    std::fs::write(&second_fasta, ">chr1 rewrapped\nACGT\nACGT\n").unwrap();

    for (fasta, report) in [
        (&first_fasta, &first_report),
        (&second_fasta, &second_report),
    ] {
        let mut cmd = Command::cargo_bin("fastaguard").unwrap();
        cmd.arg("reference")
            .arg(fasta)
            .args(["--format", "json", "--json"])
            .arg(report)
            .assert()
            .success();
    }

    let first = read_json(&first_report);
    let second = read_json(&second_report);
    for field in [
        "seqcol_digest",
        "name_length_pairs_digest",
        "sorted_name_length_pairs_digest",
    ] {
        assert_eq!(
            first["canonical_reference"][field].as_str().unwrap().len(),
            32
        );
        assert_eq!(
            first["canonical_reference"][field],
            second["canonical_reference"][field]
        );
    }
    assert_ne!(
        first["canonical_reference"]["physical_sha256"],
        second["canonical_reference"]["physical_sha256"]
    );
}

#[test]
fn invalid_canonical_reference_does_not_claim_seqcol_digests() {
    let temp_dir = TempDir::new().unwrap();
    let fasta = temp_dir.path().join("reference.fa");
    let report_path = temp_dir.path().join("reference.json");
    std::fs::write(&fasta, ">chr1\nACGTX\n").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("reference")
        .arg(&fasta)
        .args(["--format", "json", "--json"])
        .arg(&report_path)
        .assert()
        .success();

    let report = read_json(&report_path);
    assert!(report["canonical_reference"]["seqcol_digest"].is_null());
    assert!(report["canonical_reference"]["name_length_pairs_digest"].is_null());
    assert!(report["canonical_reference"]["sorted_name_length_pairs_digest"].is_null());
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
    .success()
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["report_type"], json!("compare"));
    assert_eq!(report["schema_version"], json!("0.7.0"));
    assert_eq!(report["summary"]["sample_count"], json!(2));
    assert_eq!(report["summary"]["fail_count"], json!(1));
    let samples = report["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 2);
    for sample in samples {
        let readiness_categories = sample["readiness_categories"].as_array().unwrap();
        assert!(readiness_categories.iter().any(|category| {
            category["id"] == "index" && category["label"] == "Index readiness"
        }));
        assert!(readiness_categories.iter().any(|category| {
            category["id"] == "machine" && category["label"] == "Machine readiness"
        }));
    }
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
    assert!(html.contains("<th>Index readiness</th>"), "{html}");
    assert!(html.contains("<th>Machine readiness</th>"), "{html}");
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
    .success()
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
    .success()
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
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

    assert_all_outputs_exist(&outputs);
    let json = std::fs::read_to_string(&outputs.json).unwrap();
    assert!(json.contains(r#""status": "WARN""#), "{json}");
    assert!(json.contains(r#""terminal_ns""#), "{json}");
}

#[test]
fn outdir_creates_nested_bundle_with_exact_names() {
    let temp_dir = TempDir::new().unwrap();
    let outdir = temp_dir.path().join("nested").join("reports");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--outdir")
        .arg(&outdir)
        .args(["--prefix", "sample-01"])
        .assert()
        .success()
        .stderr(predicate::str::contains("fastaguard error:").not());

    let expected = [
        "sample-01.fastaguard.html",
        "sample-01.fastaguard.json",
        "sample-01.fastaguard.tsv",
        "sample-01.fastaguard_mqc.json",
    ];
    let mut actual = std::fs::read_dir(&outdir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn outdir_prefix_requires_outdir() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args(["testdata/valid_assembly.fa", "--prefix", "sample-01"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--outdir"));
}

#[test]
fn outdir_conflicts_with_explicit_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let outdir = temp_dir.path().join("reports");
    let json = temp_dir.path().join("explicit.json");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--outdir")
        .arg(&outdir)
        .arg("--json")
        .arg(&json)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn outdir_rejects_prefix_with_parent_component() {
    let temp_dir = TempDir::new().unwrap();
    let outdir = temp_dir.path().join("reports");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--outdir")
        .arg(&outdir)
        .args(["--prefix", "../escape"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("--prefix"));

    assert!(!outdir.exists());
    assert!(!temp_dir.path().join("escape.fastaguard.json").exists());
}

#[test]
fn no_clobber_preserves_existing_bundle_file_without_force() {
    let temp_dir = TempDir::new().unwrap();
    let outdir = temp_dir.path().join("reports");
    std::fs::create_dir(&outdir).unwrap();
    let collision = outdir.join("sample-01.fastaguard.json");
    std::fs::write(&collision, "keep me").unwrap();

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--outdir")
        .arg(&outdir)
        .args(["--prefix", "sample-01"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("already exists"));

    assert_eq!(std::fs::read_to_string(collision).unwrap(), "keep me");
    assert!(!outdir.join("sample-01.fastaguard.html").exists());
    assert!(!outdir.join("sample-01.fastaguard.tsv").exists());
    assert!(!outdir.join("sample-01.fastaguard_mqc.json").exists());
}

#[test]
fn terminal_summary_is_stderr_only_and_excludes_sensitive_input_data() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("private-input-name.fa");
    let header = "private_header_value";
    let sequence = "AACCGGTTAACCGGTTAACCGGTT";
    std::fs::write(&input, format!(">{header}\n{sequence}\n")).unwrap();
    let input_checksum = sha256_file(&input);
    let outputs = output_paths(&temp_dir, "terminal_summary");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    let assertion = cmd
        .arg(&input)
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
        .success()
        .stdout(predicate::str::is_empty());

    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr);
    for expected in [
        "verdict=PASS",
        "gate.can_continue=true",
        "duration_ms=",
        &format!("json={}", outputs.json.display()),
        &format!("tsv={}", outputs.tsv.display()),
        &format!("multiqc={}", outputs.multiqc.display()),
        &format!("html={}", outputs.html.display()),
    ] {
        assert!(
            stderr.contains(expected),
            "missing {expected:?} in {stderr:?}"
        );
    }
    for sensitive in [
        input.display().to_string(),
        header.to_string(),
        input_checksum,
        sequence.to_string(),
    ] {
        assert!(
            !stderr.contains(&sensitive),
            "terminal summary exposed sensitive input data: {stderr:?}"
        );
    }
}

#[test]
fn outdir_force_replaces_exact_bundle_paths() {
    let temp_dir = TempDir::new().unwrap();
    let outdir = temp_dir.path().join("reports");
    std::fs::create_dir(&outdir).unwrap();
    let paths = [
        outdir.join("sample-01.fastaguard.html"),
        outdir.join("sample-01.fastaguard.json"),
        outdir.join("sample-01.fastaguard.tsv"),
        outdir.join("sample-01.fastaguard_mqc.json"),
    ];
    for path in &paths {
        std::fs::write(path, "replace me").unwrap();
    }

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("testdata/valid_assembly.fa")
        .arg("--outdir")
        .arg(&outdir)
        .args(["--prefix", "sample-01", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains("fastaguard error:").not());

    for path in paths {
        assert_ne!(std::fs::read_to_string(path).unwrap(), "replace me");
    }
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
    .stderr(predicate::str::contains("FastaGuard verdict="));

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
    .success();

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.7.0"));
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
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

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
        .success()
        .stderr(predicate::str::contains("FastaGuard verdict="));

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
        .success()
        .stderr(predicate::str::contains("FastaGuard verdict="));

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
        "--force",
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
    .stderr(predicate::str::contains("FastaGuard verdict="));

    assert_json_matches_golden(&paths.json, "tests/golden/valid_assembly.json");
}

#[test]
fn problem_assembly_writes_failure_report_with_successful_process() {
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
        .success()
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
    .success()
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["schema_version"], json!("0.7.0"));
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
        .success();

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
    .success()
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
    .success()
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
fn fail_on_rejects_unknown_finding_ids_before_writing_reports() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "unknown_fail_on");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--fail-on",
        "not_a_rule",
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
    .code(3)
    .stderr(predicate::str::contains("unknown finding id 'not_a_rule'"));

    assert!(!outputs.json.exists());
    assert!(!outputs.html.exists());
    assert!(!outputs.tsv.exists());
    assert!(!outputs.multiqc.exists());
}

#[test]
fn fail_on_reports_all_unknown_ids_once_in_lexical_order() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "multiple_unknown_fail_on");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--fail-on",
        "z_rule,a_rule,z_rule",
        "--fail-on",
        "m_rule,a_rule",
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
    .code(3)
    .stderr(predicate::eq(
        "fastaguard error: unknown finding id 'a_rule'; unknown finding id 'm_rule'; unknown finding id 'z_rule'\n",
    ));

    assert!(!outputs.json.exists());
    assert!(!outputs.html.exists());
    assert!(!outputs.tsv.exists());
    assert!(!outputs.multiqc.exists());
}

#[test]
fn fail_on_warn_report_distinguishes_pass_only_safety_from_gate_continuation() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "warn_continuation");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
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
    .arg(&outputs.multiqc)
    .assert()
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

    let report = read_json(&outputs.json);
    assert_eq!(report["verdict"]["status"], json!("WARN"));
    assert_eq!(
        report["machine_summary"]["safe_for_downstream"],
        json!(false)
    );
    assert_eq!(report["gate"]["can_continue"], json!(true));

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    assert!(tsv.contains("gate_can_continue\ttrue\n"), "{tsv}");
    assert!(tsv.contains("submission_policy_id\t.\n"), "{tsv}");

    let multiqc = read_json(&outputs.multiqc);
    assert_eq!(
        multiqc["data"]["valid_assembly"]["gate_can_continue"],
        json!(true)
    );
    assert_eq!(
        multiqc["data"]["valid_assembly"]["submission_policy_id"],
        json!(".")
    );

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("PASS-only downstream safety"), "{html}");
    assert!(html.contains("Workflow may continue"), "{html}");
}

#[test]
fn expected_size_serializes_thresholds_evidence_and_tsv_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "expected_size");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--expected-size",
        "5mb",
        "--expected-size-tolerance",
        "0.13",
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
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

    let report = read_json(&outputs.json);
    assert_eq!(
        report["provenance"]["thresholds"]["expected_size_bases"],
        json!(5_000_000)
    );
    assert_eq!(
        report["provenance"]["thresholds"]["expected_size_tolerance"],
        json!(0.13)
    );

    let evidence = &finding_by_id(&report, "expected_size_outlier")["evidence"];
    assert_eq!(evidence["observed_ungapped_length"], json!(46));
    assert_eq!(evidence["expected_size_bases"], json!(5_000_000));
    assert_eq!(evidence["expected_size_tolerance"], json!(0.13));
    assert_eq!(evidence["expected_size_lower_bound"], json!(4_350_000));
    assert_eq!(evidence["expected_size_upper_bound"], json!(5_650_000));
    assert_eq!(
        evidence["expected_size_deviation_bases"],
        json!(-4_999_954_i64)
    );

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    for metric in [
        "expected_size_bases\t5000000\n",
        "expected_size_tolerance\t0.13\n",
        "expected_size_observed_ungapped_length\t46\n",
        "expected_size_lower_bound\t4350000\n",
        "expected_size_upper_bound\t5650000\n",
        "expected_size_deviation_bases\t-4999954\n",
    ] {
        assert!(tsv.contains(metric), "missing {metric:?} in {tsv}");
    }

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(
        html.contains("Observed ungapped length:</span> 46"),
        "{html}"
    );
    assert!(html.contains("Expected size:</span> 5000000"), "{html}");
}

#[test]
fn report_parity_preserves_gate_policy_and_expected_size_evidence() {
    for (stem, input, gate_args) in [
        (
            "ncbi_fail_parity",
            "testdata/ncbi_genome/terminal_ns.fa",
            vec!["--gate", "submission", "--submission-target", "ncbi"],
        ),
        (
            "pipeline_warn_parity",
            "testdata/valid_assembly.fa",
            vec!["--gate", "pipeline", "--min-contig-length", "1"],
        ),
    ] {
        let temp_dir = TempDir::new().unwrap();
        let outputs = output_paths(&temp_dir, stem);
        let mut cmd = Command::cargo_bin("fastaguard").unwrap();
        cmd.arg(input)
            .args(gate_args)
            .args([
                "--expected-size",
                "1kb",
                "--expected-size-tolerance",
                "0.1",
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
            .success();

        let report = read_json(&outputs.json);
        let tsv = read_metric_tsv(&outputs.tsv);
        let multiqc = read_json(&outputs.multiqc);
        let sample = multiqc["data"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        let evidence = &finding_by_id(&report, "expected_size_outlier")["evidence"];
        let policy_id = report["gate"]["submission_policy"]["id"]
            .as_str()
            .unwrap_or(".");
        let target = report["gate"]["submission_target"].as_str().unwrap_or(".");
        let blocker_list = json_string_list(&report["gate"]["blocking_findings"]);
        let tsv_blocker_list = if blocker_list.is_empty() {
            "."
        } else {
            blocker_list.as_str()
        };
        let finding_ids = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|finding| finding["id"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join(",");

        for (metric, expected) in [
            ("verdict", report["verdict"]["status"].as_str().unwrap()),
            ("gate_status", report["gate"]["status"].as_str().unwrap()),
            (
                "gate_can_continue",
                if report["gate"]["can_continue"].as_bool().unwrap() {
                    "true"
                } else {
                    "false"
                },
            ),
            ("submission_target", target),
            ("submission_policy_id", policy_id),
            ("gate_blocking_findings", tsv_blocker_list),
            ("finding_ids", finding_ids.as_str()),
        ] {
            assert_eq!(
                tsv.get(metric).map(String::as_str),
                Some(expected),
                "{stem}"
            );
        }

        for (metric, json_value) in [
            (
                "expected_size_bases",
                &report["provenance"]["thresholds"]["expected_size_bases"],
            ),
            (
                "expected_size_tolerance",
                &report["provenance"]["thresholds"]["expected_size_tolerance"],
            ),
            (
                "expected_size_observed_ungapped_length",
                &evidence["observed_ungapped_length"],
            ),
            (
                "expected_size_lower_bound",
                &evidence["expected_size_lower_bound"],
            ),
            (
                "expected_size_upper_bound",
                &evidence["expected_size_upper_bound"],
            ),
            (
                "expected_size_deviation_bases",
                &evidence["expected_size_deviation_bases"],
            ),
        ] {
            assert_eq!(
                tsv.get(metric).map(String::as_str),
                Some(json_scalar(json_value).as_str()),
                "{stem}: {metric}"
            );
            assert_eq!(&sample[metric], json_value, "{stem}: {metric}");
        }

        assert_eq!(sample["verdict"], report["verdict"]["status"], "{stem}");
        assert_eq!(sample["gate_status"], report["gate"]["status"], "{stem}");
        assert_eq!(
            sample["gate_can_continue"], report["gate"]["can_continue"],
            "{stem}"
        );
        assert_eq!(sample["submission_target"], json!(target), "{stem}");
        assert_eq!(sample["submission_policy_id"], json!(policy_id), "{stem}");
        assert_eq!(
            sample["gate_blocking_findings"],
            json!(blocker_list),
            "{stem}"
        );
        assert_eq!(sample["finding_ids"], json!(finding_ids), "{stem}");

        let html = std::fs::read_to_string(&outputs.html).unwrap();
        assert!(html.contains("Overall QC signal"), "{stem}: {html}");
        assert!(html.contains("Workflow may continue"), "{stem}: {html}");
    }
}

#[test]
fn problem_assembly_json_matches_golden_contract() {
    let paths = golden_output_paths("problem_assembly");
    let provenance_command = golden_provenance_command("problem_assembly");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    with_golden_provenance(&mut cmd, provenance_command);
    cmd.args(["testdata/problem_assembly.fa", "--force", "--out"])
        .arg(&paths.html)
        .arg("--json")
        .arg(&paths.json)
        .arg("--tsv")
        .arg(&paths.tsv)
        .arg("--multiqc")
        .arg(&paths.multiqc)
        .assert()
        .success()
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
        .success()
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
        .success()
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
        .success()
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
        .arg("--force")
        .arg("--out")
        .arg(&paths.html)
        .arg("--json")
        .arg(&paths.json)
        .arg("--tsv")
        .arg(&paths.tsv)
        .arg("--multiqc")
        .arg(&paths.multiqc)
        .assert()
        .success()
        .stderr(predicate::str::contains("fastaguard error:").not());

    assert_json_matches_golden(&paths.json, "tests/golden/invalid_empty_record.json");
}

#[test]
fn structurally_invalid_fasta_writes_failure_report_with_successful_process() {
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
        .success()
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
        .success()
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
        .code(2)
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

#[test]
fn submission_gate_defaults_to_generic_target() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_default");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--gate",
        "submission",
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
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("submission"));
    assert_eq!(report["gate"]["submission_target"], json!("generic"));
    assert_eq!(report["provenance"]["submission_target"], json!("generic"));
}

#[test]
fn submission_target_ncbi_is_serialized_when_requested() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_ncbi");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--min-contig-length",
        "1",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
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
    .success()
    .stderr(predicate::str::contains("FastaGuard verdict="));

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["submission_target"], json!("ncbi"));
    assert_eq!(report["provenance"]["submission_target"], json!("ncbi"));
    assert_eq!(
        report["gate"]["submission_policy"]["id"],
        json!("ncbi_genome")
    );
    assert_eq!(
        report["provenance"]["submission_policy"],
        report["gate"]["submission_policy"]
    );
    assert!(array_contains_string(
        &report["scope"]["can_conclude"],
        "FASTA-level submission readiness"
    ));
    assert!(array_contains_string(
        &report["scope"]["cannot_conclude"],
        "repository acceptance"
    ));
}

#[test]
fn ncbi_genome_seqid_boundaries_use_ascii_byte_lengths() {
    for (fixture, expected_blocking) in [
        ("seqid_49.fa", false),
        ("seqid_50.fa", true),
        ("seqid_51.fa", true),
    ] {
        let report = run_submission_fixture(fixture, "ncbi", &[]);
        let blocking = report["gate"]["blocking_findings"].as_array().unwrap();

        assert_eq!(
            blocking.contains(&json!("ncbi_genome_seqid")),
            expected_blocking,
            "unexpected SeqID policy result for {fixture}: {report}"
        );
        assert_eq!(
            report["gate"]["can_continue"],
            json!(!expected_blocking),
            "unexpected continuation decision for {fixture}: {report}"
        );
    }
}

#[test]
fn ncbi_genome_seqid_checks_only_the_first_token_and_allowed_character_set() {
    let allowed = run_submission_fixture("seqid_allowed_chars.fa", "ncbi", &[]);
    assert!(!array_contains_string(
        &allowed["gate"]["blocking_findings"],
        "ncbi_genome_seqid"
    ));
    assert_eq!(allowed["gate"]["can_continue"], json!(true));

    let invalid = run_submission_fixture("seqid_invalid_chars.fa", "ncbi", &[]);
    assert_eq!(invalid["verdict"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &invalid["gate"]["blocking_findings"],
        "ncbi_genome_seqid"
    ));
    assert_eq!(invalid["gate"]["can_continue"], json!(false));
    let finding = finding_by_id(&invalid, "ncbi_genome_seqid");
    assert_eq!(
        finding["evidence"]["records"][0]["reason"],
        json!("NCBI genome SeqID must be 1-49 ASCII characters from [A-Za-z0-9_.:*#-]")
    );
}

#[test]
fn ncbi_genome_terminal_ns_are_submission_blockers() {
    let report = run_submission_fixture("terminal_ns.fa", "ncbi", &[]);

    assert_eq!(report["verdict"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "terminal_ns"
    ));
    assert_eq!(report["gate"]["can_continue"], json!(false));
}

#[test]
fn ncbi_genome_short_contig_boundary_is_fixed_at_200_bases() {
    for (fixture, expected_blocking) in [
        ("contig_199.fa", true),
        ("contig_200.fa", false),
        ("contig_201.fa", false),
    ] {
        let report = run_submission_fixture(fixture, "ncbi", &[]);
        assert_eq!(
            array_contains_string(
                &report["gate"]["blocking_findings"],
                "ncbi_genome_short_contigs"
            ),
            expected_blocking,
            "unexpected short-contig policy result for {fixture}: {report}"
        );
        assert_eq!(
            report["gate"]["can_continue"],
            json!(!expected_blocking),
            "unexpected continuation decision for {fixture}: {report}"
        );
    }

    let overridden = run_submission_fixture("contig_199.fa", "ncbi", &["--min-contig-length", "1"]);
    assert!(array_contains_string(
        &overridden["gate"]["blocking_findings"],
        "ncbi_genome_short_contigs"
    ));
    assert!(!array_contains_string(
        &overridden["gate"]["advisory_findings"],
        "tiny_contigs"
    ));
    assert_eq!(overridden["gate"]["can_continue"], json!(false));
}

#[test]
fn ncbi_genome_rules_do_not_leak_into_the_generic_submission_target() {
    for fixture in ["seqid_50.fa", "seqid_invalid_chars.fa", "contig_199.fa"] {
        let report = run_submission_fixture(fixture, "generic", &["--min-contig-length", "1"]);

        assert!(!array_contains_string(
            &report["gate"]["blocking_findings"],
            "ncbi_genome_seqid"
        ));
        assert!(!array_contains_string(
            &report["gate"]["blocking_findings"],
            "ncbi_genome_short_contigs"
        ));
        assert_eq!(report["gate"]["can_continue"], json!(true));
    }
}

#[test]
fn submission_gate_fails_identifier_hazards() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_ids");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
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
    .success()
    .stderr(predicate::str::contains("fastaguard error:").not());

    let report = read_json(&outputs.json);
    assert_eq!(report["gate"]["mode"], json!("submission"));
    assert_eq!(report["gate"]["status"], json!("FAIL"));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "duplicate_first_token_ids",
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "ncbi_genome_seqid",
    ));
    assert!(array_contains_string(
        &report["gate"]["blocking_findings"],
        "ncbi_genome_short_contigs",
    ));
    assert!(!array_contains_string(
        &report["gate"]["blocking_findings"],
        "unsafe_ids"
    ));
    assert!(!array_contains_string(
        &report["gate"]["blocking_findings"],
        "reserved_header_chars"
    ));
    assert!(array_contains_string(
        &report["gate"]["advisory_findings"],
        "unsafe_ids"
    ));
    assert!(array_contains_string(
        &report["gate"]["advisory_findings"],
        "reserved_header_chars"
    ));
    assert_eq!(
        report["gate"]["fail_on"],
        json!([
            "duplicate_first_token_ids",
            "duplicate_ids",
            "invalid_chars",
            "invalid_fasta_structure",
            "ncbi_genome_seqid",
            "ncbi_genome_short_contigs",
            "terminal_ns"
        ])
    );
    assert_eq!(report["gate"]["can_continue"], json!(false));
    let submission_readiness = report["readiness"]["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["id"] == json!("submission"))
        .unwrap();
    assert_eq!(submission_readiness["target"], json!("ncbi"));
    assert_eq!(submission_readiness["status"], json!("FAIL"));
}

#[test]
fn submission_gate_invalid_chars_fail_submission_readiness() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path().join("invalid_chars_only.fa");
    std::fs::write(
        &input,
        format!(">invalid_chars_only\n{}X\n", "ACGT".repeat(60)),
    )
    .unwrap();
    let outputs = output_paths(&temp_dir, "submission_invalid_chars");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg(&input)
        .args([
            "--gate",
            "submission",
            "--submission-target",
            "generic",
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
        .success();

    let report = read_json(&outputs.json);
    let submission_readiness = report["readiness"]["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["id"] == json!("submission"))
        .unwrap();
    assert_eq!(submission_readiness["status"], json!("FAIL"));
    assert!(array_contains_string(
        &submission_readiness["findings"],
        "invalid_chars"
    ));
    assert!(array_contains_string(
        &report["readiness"]["overall"]["blockers"],
        "submission.invalid_chars"
    ));
}

#[test]
fn submission_identifier_hazards_route_to_official_validators_without_claiming_results() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_routes");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
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
    .success();

    let report = read_json(&outputs.json);
    assert_routing_hint(
        &report,
        "submission_readiness_failure",
        "fix_fasta_before_official_validation",
        false,
    );
    assert!(array_contains_tool(
        &report["machine_summary"]["recommended_next_tools"],
        "official submission validator"
    ));
}

#[test]
fn submission_gate_outputs_tsv_multiqc_and_html_fields() {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, "submission_outputs");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/submission_ids.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ncbi",
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
    .success();

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    assert!(tsv.contains("submission_target\tncbi\n"), "{tsv}");
    assert!(tsv.contains("submission_status\tFAIL\n"), "{tsv}");
    assert!(tsv.contains("unsafe_identifier_count\t"), "{tsv}");

    let multiqc = read_json(&outputs.multiqc);
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_target"],
        json!("ncbi")
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_status"],
        json!("FAIL")
    );

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("Submission Readiness"), "{html}");
    assert!(
        html.contains("Official validators are still required"),
        "{html}"
    );
}

#[test]
fn compare_submission_gate_aggregates_submission_status() {
    let temp_dir = TempDir::new().unwrap();
    let clean = temp_dir.path().join("clean.fa");
    std::fs::write(&clean, format!(">clean\n{}\n", "ACGT".repeat(60))).unwrap();
    let outputs = output_paths(&temp_dir, "submission_compare");

    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.arg("compare")
        .arg(&clean)
        .arg("testdata/submission_ids.fa")
        .args([
            "--gate",
            "submission",
            "--submission-target",
            "ncbi",
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
        .success();

    let report = read_json(&outputs.json);
    assert_eq!(report["summary"]["submission_fail_count"], json!(1));
    assert_eq!(report["summary"]["submission_ready_count"], json!(1));
    let failing = report["samples"]
        .as_array()
        .unwrap()
        .iter()
        .find(|sample| sample["sample_id"] == "submission_ids")
        .unwrap();
    assert_eq!(failing["submission_target"], json!("ncbi"));
    assert_eq!(failing["submission_policy_id"], json!("ncbi_genome"));
    assert_eq!(failing["gate_can_continue"], json!(false));
    assert_eq!(failing["submission_status"], json!("FAIL"));
    assert!(report["summary"].get("submission_policy_id").is_none());
    assert!(report["summary"].get("repository_acceptance").is_none());

    let tsv = std::fs::read_to_string(&outputs.tsv).unwrap();
    let mut tsv_lines = tsv.lines();
    let headers = tsv_lines.next().unwrap().split('\t').collect::<Vec<_>>();
    let failing_row = tsv_lines
        .map(|line| line.split('\t').collect::<Vec<_>>())
        .find(|row| row[0] == "submission_ids")
        .unwrap();
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_target"),
        "ncbi"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_policy_id"),
        "ncbi_genome"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "gate_can_continue"),
        "false"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "finding_ids"),
        json_string_list(&failing["finding_ids"])
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_status"),
        "FAIL"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_ready_count"),
        "1"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_warn_count"),
        "0"
    );
    assert_eq!(
        tsv_value(&headers, &failing_row, "submission_fail_count"),
        "1"
    );

    let multiqc = read_json(&outputs.multiqc);
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_status"],
        json!("FAIL")
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_policy_id"],
        json!("ncbi_genome")
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["gate_can_continue"],
        json!(false)
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["finding_ids"],
        json!(json_string_list(&failing["finding_ids"]))
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_ready_count"],
        json!(1)
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_warn_count"],
        json!(0)
    );
    assert_eq!(
        multiqc["data"]["submission_ids"]["submission_fail_count"],
        json!(1)
    );

    let html = std::fs::read_to_string(&outputs.html).unwrap();
    assert!(html.contains("<th>Policy ID</th>"), "{html}");
    assert!(html.contains("<th>Workflow may continue</th>"), "{html}");
    assert!(html.contains("ncbi_genome"), "{html}");
}

fn tsv_value<'a>(headers: &[&str], row: &'a [&str], name: &str) -> &'a str {
    let index = headers
        .iter()
        .position(|header| *header == name)
        .unwrap_or_else(|| panic!("missing TSV column {name}"));
    row[index]
}

fn read_metric_tsv(path: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .skip(1)
        .map(|line| {
            let (metric, value) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("invalid metric TSV line: {line:?}"));
            (metric.to_string(), value.to_string())
        })
        .collect()
}

fn json_string_list(value: &Value) -> String {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",")
}

fn json_scalar(value: &Value) -> String {
    match value {
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        _ => panic!("expected scalar JSON value, got {value}"),
    }
}

#[test]
fn unknown_submission_target_is_cli_error() {
    let mut cmd = Command::cargo_bin("fastaguard").unwrap();
    cmd.args([
        "testdata/valid_assembly.fa",
        "--gate",
        "submission",
        "--submission-target",
        "ena",
    ])
    .assert()
    .code(2)
    .stderr(predicate::str::contains("invalid value 'ena'"));
}

fn run_submission_fixture(fixture: &str, target: &str, extra_args: &[&str]) -> Value {
    let temp_dir = TempDir::new().unwrap();
    let outputs = output_paths(&temp_dir, fixture.trim_end_matches(".fa"));
    let input = Path::new("testdata/ncbi_genome").join(fixture);
    let mut command = Command::cargo_bin("fastaguard").unwrap();
    command
        .arg(input)
        .args(["--gate", "submission", "--submission-target", target])
        .args(extra_args)
        .arg("--json")
        .arg(&outputs.json)
        .arg("--out")
        .arg(&outputs.html)
        .arg("--tsv")
        .arg(&outputs.tsv)
        .arg("--multiqc")
        .arg(&outputs.multiqc);

    let assertion = command.assert().code(0);
    assert_eq!(assertion.get_output().status.code(), Some(0));
    read_json(&outputs.json)
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
