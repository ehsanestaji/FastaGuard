use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

use crate::parser::{self, FastaEvent, FastaRecord};
use crate::profile::ProfileConfig;
use crate::stats::composition::{fraction, percent, round2};
use crate::stats::nxx::nx_lx;
use crate::stats::outliers::{iqr_outlier_indices, zscore_outlier_indices};

#[derive(Debug, Clone)]
pub struct SequenceSummary {
    pub id: String,
    pub header: String,
    pub first_token_id: String,
    pub duplicate_id: bool,
    pub duplicate_first_token_id: bool,
    pub duplicate_sequence: bool,
    pub unsafe_id: bool,
    pub long_header: bool,
    pub reserved_header_chars: bool,
    pub length: u64,
    pub gc_count: u64,
    pub at_count: u64,
    pub n_count: u64,
    pub ambiguity_count: u64,
    pub invalid_count: u64,
    pub max_gap_run: u64,
    pub terminal_n_prefix: u64,
    pub terminal_n_suffix: u64,
    pub gap_run_100_count: u64,
    pub n_fraction: f64,
    pub gc_percent: f64,
    pub gc_outlier: bool,
    pub length_outlier: bool,
    pub composite_anomaly: bool,
    pub gc_zscore: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AssemblyMetrics {
    pub sequence_count: u64,
    pub total_length: u64,
    pub min_length: u64,
    pub max_length: u64,
    pub mean_length: f64,
    pub median_length: f64,
    pub n50: u64,
    pub n90: u64,
    pub l50: u64,
    pub l90: u64,
    pub gc_percent: f64,
    pub at_percent: f64,
    pub n_percent: f64,
    pub ambiguity_percent: f64,
    pub duplicate_id_count: u64,
    pub duplicate_first_token_id_count: u64,
    pub duplicate_sequence_count: u64,
    pub unsafe_id_count: u64,
    pub long_header_count: u64,
    pub reserved_header_char_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub terminal_n_sequence_count: u64,
    pub repeated_gap_pattern_sequence_count: u64,
    pub max_gap_run: u64,
    pub ungapped_total_length: u64,
    pub sequences: Vec<SequenceSummary>,
}

impl AssemblyMetrics {
    pub fn from_records(records: Vec<FastaRecord>, profile: &ProfileConfig) -> Self {
        let mut accumulator = MetricsAccumulator::new(profile);
        for record in records {
            accumulator.start_record(record.id, record.header);
            accumulator.add_sequence_bytes(&record.sequence);
            accumulator.end_record();
        }
        accumulator.finish()
    }

    pub fn from_path(path: &Path, profile: &ProfileConfig) -> Result<Self> {
        let mut accumulator = MetricsAccumulator::new(profile);
        parser::for_each_fasta_event(path, |event| {
            match event {
                FastaEvent::StartRecord { id, header, .. } => accumulator.start_record(id, header),
                FastaEvent::SequenceLine { bytes, .. } => accumulator.add_sequence_bytes(bytes),
                FastaEvent::EndRecord => accumulator.end_record(),
            }
            Ok(())
        })?;

        Ok(accumulator.finish())
    }
}

struct MetricsAccumulator<'a> {
    profile: &'a ProfileConfig,
    seen_ids: BTreeSet<String>,
    seen_first_token_ids: BTreeSet<String>,
    seen_sequence_hashes: BTreeSet<[u8; 32]>,
    duplicate_id_count: u64,
    duplicate_first_token_id_count: u64,
    duplicate_sequence_count: u64,
    lengths: Vec<u64>,
    gc_total: u128,
    at_total: u128,
    n_total: u128,
    ambiguity_total: u128,
    unsafe_id_count: u64,
    long_header_count: u64,
    reserved_header_char_count: u64,
    invalid_sequence_count: u64,
    high_n_sequence_count: u64,
    tiny_contig_count: u64,
    terminal_n_sequence_count: u64,
    repeated_gap_pattern_sequence_count: u64,
    max_gap_run: u64,
    ungapped_total: u128,
    current_sequence: Option<SequenceSummaryBuilder>,
    sequences: Vec<SequenceSummary>,
}

impl<'a> MetricsAccumulator<'a> {
    fn new(profile: &'a ProfileConfig) -> Self {
        Self {
            profile,
            seen_ids: BTreeSet::new(),
            seen_first_token_ids: BTreeSet::new(),
            seen_sequence_hashes: BTreeSet::new(),
            duplicate_id_count: 0,
            duplicate_first_token_id_count: 0,
            duplicate_sequence_count: 0,
            lengths: Vec::new(),
            gc_total: 0,
            at_total: 0,
            n_total: 0,
            ambiguity_total: 0,
            unsafe_id_count: 0,
            long_header_count: 0,
            reserved_header_char_count: 0,
            invalid_sequence_count: 0,
            high_n_sequence_count: 0,
            tiny_contig_count: 0,
            terminal_n_sequence_count: 0,
            repeated_gap_pattern_sequence_count: 0,
            max_gap_run: 0,
            ungapped_total: 0,
            current_sequence: None,
            sequences: Vec::new(),
        }
    }

    fn start_record(&mut self, id: String, header: String) {
        let duplicate_id = !self.seen_ids.insert(id.clone());
        if duplicate_id {
            self.duplicate_id_count += 1;
        }

        let first_token_id = header.split_whitespace().next().unwrap_or(&id).to_string();
        let duplicate_first_token_id = !self.seen_first_token_ids.insert(first_token_id.clone());
        if duplicate_first_token_id {
            self.duplicate_first_token_id_count += 1;
        }

        let unsafe_id = unsafe_id(&id);
        if unsafe_id {
            self.unsafe_id_count += 1;
        }
        let long_header = header.chars().count() > 255;
        if long_header {
            self.long_header_count += 1;
        }
        let reserved_header_chars = reserved_header_chars(&header);
        if reserved_header_chars {
            self.reserved_header_char_count += 1;
        }

        self.current_sequence = Some(SequenceSummaryBuilder::new(
            id,
            header,
            first_token_id,
            HeaderReadinessFlags {
                duplicate_id,
                duplicate_first_token_id,
                unsafe_id,
                long_header,
                reserved_header_chars,
            },
        ));
    }

    fn add_sequence_bytes(&mut self, bytes: &[u8]) {
        if let Some(current_sequence) = &mut self.current_sequence {
            current_sequence.add_bytes(bytes);
        }
    }

    fn end_record(&mut self) {
        let Some(current_sequence) = self.current_sequence.take() else {
            return;
        };

        let (mut summary, sequence_hash) = current_sequence.finish();
        let duplicate_sequence = !self.seen_sequence_hashes.insert(sequence_hash);
        if duplicate_sequence {
            self.duplicate_sequence_count += 1;
            summary.duplicate_sequence = true;
        }

        self.lengths.push(summary.length);
        self.gc_total += u128::from(summary.gc_count);
        self.at_total += u128::from(summary.at_count);
        self.n_total += u128::from(summary.n_count);
        self.ambiguity_total += u128::from(summary.ambiguity_count);
        self.ungapped_total += u128::from(summary.length.saturating_sub(summary.n_count));
        if summary.invalid_count > 0 {
            self.invalid_sequence_count += 1;
        }
        if summary.n_fraction >= self.profile.high_n_sequence_fraction {
            self.high_n_sequence_count += 1;
        }
        if summary.length < self.profile.min_contig_length {
            self.tiny_contig_count += 1;
        }
        if summary.terminal_n_prefix > 0 || summary.terminal_n_suffix > 0 {
            self.terminal_n_sequence_count += 1;
        }
        if summary.gap_run_100_count > 0 {
            self.repeated_gap_pattern_sequence_count += 1;
        }
        self.max_gap_run = self.max_gap_run.max(summary.max_gap_run);
        self.sequences.push(summary);
    }

    fn finish(mut self) -> AssemblyMetrics {
        self.mark_outlier_signals();
        self.lengths.sort_unstable();

        let sequence_count = self.lengths.len() as u64;
        let total_length_u128 = self
            .lengths
            .iter()
            .fold(0_u128, |total, length| total + u128::from(*length));
        let total_length = saturating_u128_to_u64(total_length_u128);
        let min_length = self.lengths.first().copied().unwrap_or(0);
        let max_length = self.lengths.last().copied().unwrap_or(0);
        let mean_length = if sequence_count == 0 {
            0.0
        } else {
            round2(total_length_u128 as f64 / sequence_count as f64)
        };
        let median_length = median(&self.lengths);
        let n50 = nx_lx(&self.lengths, 0.50);
        let n90 = nx_lx(&self.lengths, 0.90);

        AssemblyMetrics {
            sequence_count,
            total_length,
            min_length,
            max_length,
            mean_length,
            median_length,
            n50: n50.nx,
            n90: n90.nx,
            l50: n50.lx,
            l90: n90.lx,
            gc_percent: percent(saturating_u128_to_u64(self.gc_total), total_length),
            at_percent: percent(saturating_u128_to_u64(self.at_total), total_length),
            n_percent: percent(saturating_u128_to_u64(self.n_total), total_length),
            ambiguity_percent: percent(saturating_u128_to_u64(self.ambiguity_total), total_length),
            duplicate_id_count: self.duplicate_id_count,
            duplicate_first_token_id_count: self.duplicate_first_token_id_count,
            duplicate_sequence_count: self.duplicate_sequence_count,
            unsafe_id_count: self.unsafe_id_count,
            long_header_count: self.long_header_count,
            reserved_header_char_count: self.reserved_header_char_count,
            invalid_sequence_count: self.invalid_sequence_count,
            high_n_sequence_count: self.high_n_sequence_count,
            tiny_contig_count: self.tiny_contig_count,
            terminal_n_sequence_count: self.terminal_n_sequence_count,
            repeated_gap_pattern_sequence_count: self.repeated_gap_pattern_sequence_count,
            max_gap_run: self.max_gap_run,
            ungapped_total_length: saturating_u128_to_u64(self.ungapped_total),
            sequences: self.sequences,
        }
    }

    fn mark_outlier_signals(&mut self) {
        let gc_values = self
            .sequences
            .iter()
            .map(|sequence| sequence.gc_percent)
            .collect::<Vec<_>>();
        let gc_zscores = gc_zscores(&gc_values);
        for (sequence, zscore) in self.sequences.iter_mut().zip(gc_zscores) {
            sequence.gc_zscore = zscore;
        }
        for index in zscore_outlier_indices(&gc_values, self.profile.gc_outlier_zscore) {
            if let Some(sequence) = self.sequences.get_mut(index) {
                sequence.gc_outlier = true;
            }
        }

        let lengths_in_original_sequence_order = self
            .sequences
            .iter()
            .map(|sequence| sequence.length)
            .collect::<Vec<_>>();
        for index in iqr_outlier_indices(&lengths_in_original_sequence_order, 1.5) {
            if let Some(sequence) = self.sequences.get_mut(index) {
                sequence.length_outlier = true;
            }
        }

        for sequence in &mut self.sequences {
            let has_composition_signal =
                sequence.gc_outlier || sequence.n_fraction >= self.profile.high_n_sequence_fraction;
            let has_independent_signal = [
                sequence.length_outlier,
                sequence.duplicate_sequence,
                sequence.invalid_count > 0,
                sequence.max_gap_run > self.profile.max_gap_run,
            ]
            .into_iter()
            .any(|signal| signal);
            sequence.composite_anomaly = has_composition_signal && has_independent_signal;
        }
    }
}

struct HeaderReadinessFlags {
    duplicate_id: bool,
    duplicate_first_token_id: bool,
    unsafe_id: bool,
    long_header: bool,
    reserved_header_chars: bool,
}

struct SequenceSummaryBuilder {
    id: String,
    header: String,
    first_token_id: String,
    duplicate_id: bool,
    duplicate_first_token_id: bool,
    unsafe_id: bool,
    long_header: bool,
    reserved_header_chars: bool,
    hasher: Sha256,
    length: u64,
    gc_count: u64,
    at_count: u64,
    n_count: u64,
    ambiguity_count: u64,
    invalid_count: u64,
    current_gap_run: u64,
    max_gap_run: u64,
    first_base: Option<u8>,
    last_base: Option<u8>,
    leading_n_count: u64,
    trailing_n_count: u64,
    current_gap_run_length: u64,
    gap_run_100_count: u64,
}

impl SequenceSummaryBuilder {
    fn new(
        id: String,
        header: String,
        first_token_id: String,
        flags: HeaderReadinessFlags,
    ) -> Self {
        Self {
            id,
            header,
            first_token_id,
            duplicate_id: flags.duplicate_id,
            duplicate_first_token_id: flags.duplicate_first_token_id,
            unsafe_id: flags.unsafe_id,
            long_header: flags.long_header,
            reserved_header_chars: flags.reserved_header_chars,
            hasher: Sha256::new(),
            length: 0,
            gc_count: 0,
            at_count: 0,
            n_count: 0,
            ambiguity_count: 0,
            invalid_count: 0,
            current_gap_run: 0,
            max_gap_run: 0,
            first_base: None,
            last_base: None,
            leading_n_count: 0,
            trailing_n_count: 0,
            current_gap_run_length: 0,
            gap_run_100_count: 0,
        }
    }

    fn add_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            let upper = byte.to_ascii_uppercase();
            self.hasher.update([upper]);
            self.length = self.length.saturating_add(1);
            if self.first_base.is_none() {
                self.first_base = Some(upper);
            }
            self.last_base = Some(upper);
            if upper == b'N' && self.length == self.leading_n_count.saturating_add(1) {
                self.leading_n_count += 1;
            }
            if upper == b'N' {
                self.trailing_n_count += 1;
            } else {
                self.trailing_n_count = 0;
            }

            match upper {
                b'G' | b'C' => {
                    self.gc_count += 1;
                    self.finish_current_gap_run();
                    self.current_gap_run = 0;
                }
                b'A' | b'T' | b'U' => {
                    self.at_count += 1;
                    self.finish_current_gap_run();
                    self.current_gap_run = 0;
                }
                b'N' => {
                    self.n_count += 1;
                    self.ambiguity_count += 1;
                    self.current_gap_run_length += 1;
                    self.current_gap_run += 1;
                    self.max_gap_run = self.max_gap_run.max(self.current_gap_run);
                }
                b'M' | b'R' | b'W' | b'S' | b'Y' | b'K' | b'V' | b'H' | b'D' | b'B' => {
                    self.ambiguity_count += 1;
                    self.finish_current_gap_run();
                    self.current_gap_run = 0;
                }
                _ => {
                    self.invalid_count += 1;
                    self.finish_current_gap_run();
                    self.current_gap_run = 0;
                }
            }
        }
    }

    fn finish_current_gap_run(&mut self) {
        if self.current_gap_run_length == 100 {
            self.gap_run_100_count += 1;
        }
        self.current_gap_run_length = 0;
    }

    fn finish(mut self) -> (SequenceSummary, [u8; 32]) {
        self.finish_current_gap_run();
        let summary = SequenceSummary {
            id: self.id,
            header: self.header,
            first_token_id: self.first_token_id,
            duplicate_id: self.duplicate_id,
            duplicate_first_token_id: self.duplicate_first_token_id,
            duplicate_sequence: false,
            unsafe_id: self.unsafe_id,
            long_header: self.long_header,
            reserved_header_chars: self.reserved_header_chars,
            length: self.length,
            gc_count: self.gc_count,
            at_count: self.at_count,
            n_count: self.n_count,
            ambiguity_count: self.ambiguity_count,
            invalid_count: self.invalid_count,
            max_gap_run: self.max_gap_run,
            terminal_n_prefix: if self.first_base == Some(b'N') {
                self.leading_n_count
            } else {
                0
            },
            terminal_n_suffix: if self.last_base == Some(b'N') {
                self.trailing_n_count
            } else {
                0
            },
            gap_run_100_count: self.gap_run_100_count,
            n_fraction: fraction(self.n_count, self.length),
            gc_percent: percent(self.gc_count, self.length),
            gc_outlier: false,
            length_outlier: false,
            composite_anomaly: false,
            gc_zscore: None,
        };

        (summary, self.hasher.finalize().into())
    }
}

fn unsafe_id(id: &str) -> bool {
    id.trim() != id
        || id
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
}

fn reserved_header_chars(header: &str) -> bool {
    header
        .chars()
        .any(|ch| matches!(ch, '|' | ';' | '"' | '\'' | '`' | '<' | '>' | '\t'))
}

fn gc_zscores(values: &[f64]) -> Vec<Option<f64>> {
    if values.len() < 3 || values.iter().any(|value| !value.is_finite()) {
        return vec![None; values.len()];
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    let stddev = variance.sqrt();

    if stddev == 0.0 {
        return vec![None; values.len()];
    }

    values
        .iter()
        .map(|value| Some(round2((value - mean).abs() / stddev)))
        .collect()
}

fn saturating_u128_to_u64(value: u128) -> u64 {
    value.try_into().unwrap_or(u64::MAX)
}

fn median(lengths: &[u64]) -> f64 {
    if lengths.is_empty() {
        return 0.0;
    }

    let midpoint = lengths.len() / 2;
    if lengths.len() % 2 == 1 {
        lengths[midpoint] as f64
    } else {
        round2((lengths[midpoint - 1] as f64 / 2.0) + (lengths[midpoint] as f64 / 2.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ProfileConfig, ThresholdOverrides};

    fn profile() -> ProfileConfig {
        ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: Some(10),
            expected_size_bases: None,
            expected_size_tolerance: None,
        })
    }

    #[test]
    fn summarizes_valid_records() {
        let records = vec![
            FastaRecord {
                id: "a".into(),
                header: "a".into(),
                sequence: b"ACGTNN".to_vec(),
            },
            FastaRecord {
                id: "b".into(),
                header: "b".into(),
                sequence: b"GGCC".to_vec(),
            },
        ];

        let metrics = AssemblyMetrics::from_records(records, &profile());

        assert_eq!(metrics.sequence_count, 2);
        assert_eq!(metrics.total_length, 10);
        assert_eq!(metrics.n50, 6);
        assert_eq!(metrics.gc_percent, 60.0);
        assert_eq!(metrics.n_percent, 20.0);
    }

    #[test]
    fn detects_duplicate_ids_invalid_chars_tiny_contigs_and_gap_runs() {
        let records = vec![
            FastaRecord {
                id: "dup".into(),
                header: "dup".into(),
                sequence: b"ACGT".to_vec(),
            },
            FastaRecord {
                id: "dup".into(),
                header: "dup second".into(),
                sequence: b"ACGT".to_vec(),
            },
            FastaRecord {
                id: "bad".into(),
                header: "bad".into(),
                sequence: b"ACGTXYZ".to_vec(),
            },
            FastaRecord {
                id: "gap".into(),
                header: "gap".into(),
                sequence: b"AAANNNNNCCCC".to_vec(),
            },
        ];

        let metrics = AssemblyMetrics::from_records(records, &profile());

        assert_eq!(metrics.duplicate_id_count, 1);
        assert_eq!(metrics.duplicate_sequence_count, 1);
        assert_eq!(metrics.invalid_sequence_count, 1);
        assert_eq!(metrics.tiny_contig_count, 3);
        assert_eq!(metrics.max_gap_run, 5);
        assert!(metrics.sequences[1].duplicate_id);
        assert!(metrics.sequences[1].duplicate_sequence);
        assert_eq!(metrics.sequences[2].invalid_count, 2);
    }

    #[test]
    fn records_header_and_index_readiness_signals() {
        let metrics = AssemblyMetrics::from_records(
            vec![
                FastaRecord {
                    id: "contig1".into(),
                    header: "contig1 length=1000".into(),
                    sequence: b"ACGT".to_vec(),
                },
                FastaRecord {
                    id: "contig1".into(),
                    header: "contig1 length=2000".into(),
                    sequence: b"TGCA".to_vec(),
                },
                FastaRecord {
                    id: "unsafe/path".into(),
                    header: "unsafe/path with|pipe".into(),
                    sequence: b"ACGT".to_vec(),
                },
            ],
            &profile(),
        );

        assert_eq!(metrics.duplicate_first_token_id_count, 1);
        assert_eq!(metrics.unsafe_id_count, 1);
        assert_eq!(metrics.reserved_header_char_count, 1);
        assert!(metrics.sequences[1].duplicate_first_token_id);
        assert!(metrics.sequences[2].unsafe_id);
        assert!(metrics.sequences[2].reserved_header_chars);
    }

    #[test]
    fn detects_terminal_ns_and_gap_pattern_counts() {
        let metrics = AssemblyMetrics::from_records(
            vec![
                FastaRecord {
                    id: "terminal".into(),
                    header: "terminal".into(),
                    sequence: b"NACGTN".to_vec(),
                },
                FastaRecord {
                    id: "gap100".into(),
                    header: "gap100".into(),
                    sequence: format!("AAA{}TTT", "N".repeat(100)).into_bytes(),
                },
            ],
            &profile(),
        );

        assert_eq!(metrics.terminal_n_sequence_count, 1);
        assert_eq!(metrics.repeated_gap_pattern_sequence_count, 1);
        assert_eq!(metrics.sequences[0].terminal_n_prefix, 1);
        assert_eq!(metrics.sequences[0].terminal_n_suffix, 1);
        assert_eq!(metrics.sequences[1].gap_run_100_count, 1);
    }

    #[test]
    fn streams_metrics_from_path_with_event_parser() {
        let metrics =
            AssemblyMetrics::from_path(Path::new("testdata/problem_assembly.fa"), &profile())
                .unwrap();

        assert_eq!(metrics.duplicate_id_count, 1);
        assert_eq!(metrics.invalid_sequence_count, 1);
    }

    #[test]
    fn streams_exact_100_gap_runs_across_lines_and_resets_records() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("split_gap.fa");
        std::fs::write(
            &path,
            format!(
                ">split_gap\nAAA{}\n{}TTT\n>reset_gap\n{}AAA{}\n",
                "N".repeat(60),
                "N".repeat(40),
                "N".repeat(50),
                "N".repeat(50),
            ),
        )
        .unwrap();

        let metrics = AssemblyMetrics::from_path(&path, &profile()).unwrap();

        assert_eq!(metrics.repeated_gap_pattern_sequence_count, 1);
        assert_eq!(metrics.sequences[0].gap_run_100_count, 1);
        assert_eq!(metrics.sequences[1].gap_run_100_count, 0);
    }

    #[test]
    fn median_handles_large_even_lengths_without_overflow() {
        assert_eq!(median(&[u64::MAX, u64::MAX]), u64::MAX as f64);
    }

    #[test]
    fn composite_anomaly_requires_composition_signal_plus_independent_signal() {
        let mut records = normal_records();
        records.push(FastaRecord {
            id: "long_balanced_1".into(),
            header: "long_balanced_1".into(),
            sequence: balanced_sequence(10_000),
        });
        records.push(FastaRecord {
            id: "long_balanced_2".into(),
            header: "long_balanced_2".into(),
            sequence: balanced_sequence(10_000),
        });

        let metrics = AssemblyMetrics::from_records(records, &profile());
        let duplicate_length_outlier = metrics
            .sequences
            .iter()
            .find(|sequence| sequence.id == "long_balanced_2")
            .unwrap();

        assert!(duplicate_length_outlier.length_outlier);
        assert!(duplicate_length_outlier.duplicate_sequence);
        assert!(!duplicate_length_outlier.composite_anomaly);
    }

    #[test]
    fn composite_anomaly_allows_composition_signal_plus_independent_signal() {
        let mut records = normal_records();
        records.push(FastaRecord {
            id: "long_high_gc".into(),
            header: "long_high_gc".into(),
            sequence: vec![b'G'; 10_000],
        });

        let metrics = AssemblyMetrics::from_records(records, &profile());
        let long_high_gc = metrics
            .sequences
            .iter()
            .find(|sequence| sequence.id == "long_high_gc")
            .unwrap();

        assert!(long_high_gc.gc_outlier);
        assert!(long_high_gc.length_outlier);
        assert!(long_high_gc.composite_anomaly);
    }

    fn normal_records() -> Vec<FastaRecord> {
        [
            900, 940, 980, 1_000, 1_020, 1_040, 1_060, 1_080, 1_100, 1_120, 1_140,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, length)| FastaRecord {
            id: format!("normal_{}", index + 1),
            header: format!("normal_{}", index + 1),
            sequence: balanced_sequence(length),
        })
        .collect()
    }

    fn balanced_sequence(length: usize) -> Vec<u8> {
        b"ACGT"
            .repeat(length.div_ceil(4))
            .into_iter()
            .take(length)
            .collect()
    }
}
