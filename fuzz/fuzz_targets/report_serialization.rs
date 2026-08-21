#![no_main]

use fastaguard::cli::{OutputPaths, RuleConfig, RunConfig};
use fastaguard::findings;
use fastaguard::gate::GateMode;
use fastaguard::metrics::AssemblyMetrics;
use fastaguard::models::FastaguardReport;
use fastaguard::parser::FastaRecord;
use fastaguard::profile::{ProfileConfig, ThresholdOverrides};
use fastaguard::report;
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;

fuzz_target!(|data: &[u8]| {
    let temp_dir = tempfile::tempdir().expect("create fuzz output directory");
    let input_path = temp_dir.path().join("input.fa");
    std::fs::write(&input_path, data).expect("write temporary provenance input");

    let outputs = OutputPaths {
        html: temp_dir.path().join("report.html"),
        json: temp_dir.path().join("report.json"),
        tsv: temp_dir.path().join("report.tsv"),
        multiqc: temp_dir.path().join("report_mqc.json"),
        allow_overwrite: false,
    };
    let thresholds = ThresholdOverrides {
        max_n_rate: None,
        min_contig_length: None,
        expected_size_bases: None,
        expected_size_tolerance: None,
    };
    let profile = ProfileConfig::assembly(thresholds);
    let records = synthetic_records(data);
    let metrics = AssemblyMetrics::from_records(records, &profile);
    let rules = RuleConfig {
        fail_on: BTreeSet::new(),
    };
    let analysis = findings::analyze(&metrics, &profile, &rules, None);
    let config = RunConfig {
        input: input_path,
        profile: "assembly".to_string(),
        gate_mode: GateMode::None,
        submission_target: None,
        outputs: outputs.clone(),
        rules,
        thresholds,
        threads: 1,
        command: "fastaguard fuzz-input.fa".to_string(),
        started_at: "2026-08-21T00:00:00Z".to_string(),
        provenance_timestamp_override: Some("2026-08-21T00:00:00Z".to_string()),
    };
    let report_value = FastaguardReport::from_analysis(config, &profile, metrics, analysis, 0)
        .expect("construct bounded synthetic report");

    report::write_all(&report_value, &outputs).expect("serialize all report formats");
});

fn synthetic_records(data: &[u8]) -> Vec<FastaRecord> {
    let record_count = data.first().copied().unwrap_or(0) as usize % 8 + 1;
    let payload = data.get(1..).unwrap_or_default();
    let chunk_size = (payload.len() / record_count).clamp(1, 256);

    (0..record_count)
        .map(|index| {
            let start = index.saturating_mul(chunk_size).min(payload.len());
            let end = start.saturating_add(chunk_size).min(payload.len());
            let source = &payload[start..end];
            let sequence = if source.is_empty() {
                vec![b'A']
            } else {
                source
                    .iter()
                    .take(256)
                    .map(|byte| b"ACGTN"[*byte as usize % 5])
                    .collect()
            };
            let id = format!("record_{index}");
            FastaRecord {
                header: format!("{id} synthetic"),
                id,
                sequence,
            }
        })
        .collect()
}
