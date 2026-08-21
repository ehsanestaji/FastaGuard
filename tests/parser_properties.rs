use fastaguard::cli::RuleConfig;
use fastaguard::findings;
use fastaguard::metrics::AssemblyMetrics;
use fastaguard::parser::{for_each_fasta_event_from_reader, FastaEvent, FastaRecord};
use fastaguard::profile::{ProfileConfig, ThresholdOverrides};
use proptest::prelude::*;
use std::collections::{BTreeSet, HashSet};
use std::io::Cursor;

#[derive(Debug, PartialEq, Eq)]
struct Observed {
    sequence_count: u64,
    total_length: u64,
    n50: u64,
    n90: u64,
    duplicate_id_count: u64,
    duplicate_first_token_id_count: u64,
    duplicate_sequence_count: u64,
    finding_ids: Vec<String>,
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn wrapped_records_have_exact_metrics_and_stable_ordered_findings(
        sequences in prop::collection::vec(
            prop::collection::vec(prop::sample::select(b"ACGTN".to_vec()), 1..129),
            1..9,
        ),
    ) {
        let expected = expected_metrics(&sequences);
        let mut reference_finding_ids = None;

        for &line_width in &[1, 2, 3, 7, 16, 64, 257] {
            for newline in ["\n", "\r\n"] {
                let input = serialize_records(&sequences, line_width, newline);
                let observed = parse_and_analyze(&input).unwrap();

                prop_assert_eq!(observed.sequence_count, sequences.len() as u64);
                prop_assert_eq!(observed.total_length, expected.total_length);
                prop_assert_eq!(observed.n50, expected.n50);
                prop_assert_eq!(observed.n90, expected.n90);
                prop_assert_eq!(observed.duplicate_id_count, 0);
                prop_assert_eq!(observed.duplicate_first_token_id_count, 0);
                prop_assert_eq!(
                    observed.duplicate_sequence_count,
                    expected.duplicate_sequence_count,
                );

                if let Some(reference) = &reference_finding_ids {
                    prop_assert_eq!(&observed.finding_ids, reference);
                } else {
                    reference_finding_ids = Some(observed.finding_ids);
                }
            }
        }
    }
}

struct ExpectedMetrics {
    total_length: u64,
    n50: u64,
    n90: u64,
    duplicate_sequence_count: u64,
}

fn expected_metrics(sequences: &[Vec<u8>]) -> ExpectedMetrics {
    let lengths: Vec<u64> = sequences
        .iter()
        .map(|sequence| sequence.len() as u64)
        .collect();
    let total_length = lengths.iter().sum();
    let mut seen = HashSet::new();
    let duplicate_sequence_count = sequences
        .iter()
        .filter(|sequence| !seen.insert(sequence.as_slice()))
        .count() as u64;

    ExpectedMetrics {
        total_length,
        n50: expected_nx(&lengths, 50),
        n90: expected_nx(&lengths, 90),
        duplicate_sequence_count,
    }
}

fn expected_nx(lengths: &[u64], percentage: u64) -> u64 {
    let mut descending = lengths.to_vec();
    descending.sort_unstable_by(|left, right| right.cmp(left));
    let target = ((descending.iter().sum::<u64>() as f64) * (percentage as f64 / 100.0)).ceil();
    let mut cumulative = 0;
    for length in descending {
        cumulative += length;
        if cumulative as f64 >= target {
            return length;
        }
    }
    0
}

fn serialize_records(sequences: &[Vec<u8>], line_width: usize, newline: &str) -> Vec<u8> {
    let mut fasta = Vec::new();
    for (index, sequence) in sequences.iter().enumerate() {
        fasta.extend_from_slice(format!(">record_{index} description{newline}").as_bytes());
        for chunk in sequence.chunks(line_width) {
            fasta.extend_from_slice(chunk);
            fasta.extend_from_slice(newline.as_bytes());
        }
    }
    fasta
}

fn parse_and_analyze(input: &[u8]) -> anyhow::Result<Observed> {
    let mut records = Vec::new();
    let mut current_id = String::new();
    let mut current_header = String::new();
    let mut current_sequence = Vec::new();

    for_each_fasta_event_from_reader(Cursor::new(input), |event| {
        match event {
            FastaEvent::StartRecord { id, header, .. } => {
                current_id = id;
                current_header = header;
            }
            FastaEvent::SequenceLine { bytes, .. } => current_sequence.extend_from_slice(bytes),
            FastaEvent::EndRecord => records.push(FastaRecord {
                id: std::mem::take(&mut current_id),
                header: std::mem::take(&mut current_header),
                sequence: std::mem::take(&mut current_sequence),
            }),
        }
        Ok(())
    })?;

    let profile = ProfileConfig::assembly(ThresholdOverrides {
        max_n_rate: None,
        min_contig_length: None,
        expected_size_bases: None,
        expected_size_tolerance: None,
    });
    let metrics = AssemblyMetrics::from_records(records, &profile);
    let analysis = findings::analyze(
        &metrics,
        &profile,
        &RuleConfig {
            fail_on: BTreeSet::new(),
        },
        None,
    );

    Ok(Observed {
        sequence_count: metrics.sequence_count,
        total_length: metrics.total_length,
        n50: metrics.n50,
        n90: metrics.n90,
        duplicate_id_count: metrics.duplicate_id_count,
        duplicate_first_token_id_count: metrics.duplicate_first_token_id_count,
        duplicate_sequence_count: metrics.duplicate_sequence_count,
        finding_ids: analysis
            .findings
            .into_iter()
            .map(|finding| finding.id)
            .collect(),
    })
}
