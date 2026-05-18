use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

use crate::parser::{self, FastaRecord};
use crate::profile::ProfileConfig;
use crate::stats::composition::{fraction, percent, round2};
use crate::stats::nxx::nx_lx;

#[derive(Debug, Clone)]
pub struct SequenceSummary {
    pub id: String,
    pub length: u64,
    pub gc_count: u64,
    pub at_count: u64,
    pub n_count: u64,
    pub ambiguity_count: u64,
    pub invalid_count: u64,
    pub max_gap_run: u64,
    pub n_fraction: f64,
    pub gc_percent: f64,
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
    pub duplicate_sequence_count: u64,
    pub invalid_sequence_count: u64,
    pub high_n_sequence_count: u64,
    pub tiny_contig_count: u64,
    pub max_gap_run: u64,
    pub sequences: Vec<SequenceSummary>,
}

impl AssemblyMetrics {
    pub fn from_records(records: Vec<FastaRecord>, profile: &ProfileConfig) -> Self {
        let mut accumulator = MetricsAccumulator::new(profile);
        for record in records {
            accumulator.add_record(record);
        }
        accumulator.finish()
    }

    pub fn from_path(path: &Path, profile: &ProfileConfig) -> Result<Self> {
        let mut accumulator = MetricsAccumulator::new(profile);
        parser::for_each_fasta_record(path, |record| {
            accumulator.add_record(record);
            Ok(())
        })?;

        Ok(accumulator.finish())
    }
}

struct MetricsAccumulator<'a> {
    profile: &'a ProfileConfig,
    seen_ids: BTreeSet<String>,
    seen_sequence_hashes: BTreeSet<[u8; 32]>,
    duplicate_id_count: u64,
    duplicate_sequence_count: u64,
    lengths: Vec<u64>,
    gc_total: u64,
    at_total: u64,
    n_total: u64,
    ambiguity_total: u64,
    invalid_sequence_count: u64,
    high_n_sequence_count: u64,
    tiny_contig_count: u64,
    max_gap_run: u64,
    sequences: Vec<SequenceSummary>,
}

impl<'a> MetricsAccumulator<'a> {
    fn new(profile: &'a ProfileConfig) -> Self {
        Self {
            profile,
            seen_ids: BTreeSet::new(),
            seen_sequence_hashes: BTreeSet::new(),
            duplicate_id_count: 0,
            duplicate_sequence_count: 0,
            lengths: Vec::new(),
            gc_total: 0,
            at_total: 0,
            n_total: 0,
            ambiguity_total: 0,
            invalid_sequence_count: 0,
            high_n_sequence_count: 0,
            tiny_contig_count: 0,
            max_gap_run: 0,
            sequences: Vec::new(),
        }
    }

    fn add_record(&mut self, record: FastaRecord) {
        if !self.seen_ids.insert(record.id.clone()) {
            self.duplicate_id_count += 1;
        }

        if !self
            .seen_sequence_hashes
            .insert(sequence_hash(&record.sequence))
        {
            self.duplicate_sequence_count += 1;
        }

        let summary = summarize_sequence(record);
        self.lengths.push(summary.length);
        self.gc_total += summary.gc_count;
        self.at_total += summary.at_count;
        self.n_total += summary.n_count;
        self.ambiguity_total += summary.ambiguity_count;
        if summary.invalid_count > 0 {
            self.invalid_sequence_count += 1;
        }
        if summary.n_fraction >= self.profile.high_n_sequence_fraction {
            self.high_n_sequence_count += 1;
        }
        if summary.length < self.profile.min_contig_length {
            self.tiny_contig_count += 1;
        }
        self.max_gap_run = self.max_gap_run.max(summary.max_gap_run);
        self.sequences.push(summary);
    }

    fn finish(mut self) -> AssemblyMetrics {
        self.lengths.sort_unstable();

        let sequence_count = self.lengths.len() as u64;
        let total_length = self.lengths.iter().sum();
        let min_length = self.lengths.first().copied().unwrap_or(0);
        let max_length = self.lengths.last().copied().unwrap_or(0);
        let mean_length = if sequence_count == 0 {
            0.0
        } else {
            round2(total_length as f64 / sequence_count as f64)
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
            gc_percent: percent(self.gc_total, total_length),
            at_percent: percent(self.at_total, total_length),
            n_percent: percent(self.n_total, total_length),
            ambiguity_percent: percent(self.ambiguity_total, total_length),
            duplicate_id_count: self.duplicate_id_count,
            duplicate_sequence_count: self.duplicate_sequence_count,
            invalid_sequence_count: self.invalid_sequence_count,
            high_n_sequence_count: self.high_n_sequence_count,
            tiny_contig_count: self.tiny_contig_count,
            max_gap_run: self.max_gap_run,
            sequences: self.sequences,
        }
    }
}

fn summarize_sequence(record: FastaRecord) -> SequenceSummary {
    let mut gc_count = 0;
    let mut at_count = 0;
    let mut n_count = 0;
    let mut ambiguity_count = 0;
    let mut invalid_count = 0;
    let mut current_gap_run = 0;
    let mut max_gap_run = 0;

    for byte in &record.sequence {
        match byte.to_ascii_uppercase() {
            b'G' | b'C' => {
                gc_count += 1;
                current_gap_run = 0;
            }
            b'A' | b'T' | b'U' => {
                at_count += 1;
                current_gap_run = 0;
            }
            b'N' => {
                n_count += 1;
                ambiguity_count += 1;
                current_gap_run += 1;
                max_gap_run = max_gap_run.max(current_gap_run);
            }
            b'M' | b'R' | b'W' | b'S' | b'Y' | b'K' | b'V' | b'H' | b'D' | b'B' => {
                ambiguity_count += 1;
                current_gap_run = 0;
            }
            _ => {
                invalid_count += 1;
                current_gap_run = 0;
            }
        }
    }

    let length = record.sequence.len() as u64;

    SequenceSummary {
        id: record.id,
        length,
        gc_count,
        at_count,
        n_count,
        ambiguity_count,
        invalid_count,
        max_gap_run,
        n_fraction: fraction(n_count, length),
        gc_percent: percent(gc_count, length),
    }
}

fn sequence_hash(sequence: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for byte in sequence {
        hasher.update([byte.to_ascii_uppercase()]);
    }
    hasher.finalize().into()
}

fn median(lengths: &[u64]) -> f64 {
    if lengths.is_empty() {
        return 0.0;
    }

    let midpoint = lengths.len() / 2;
    if lengths.len() % 2 == 1 {
        lengths[midpoint] as f64
    } else {
        round2((lengths[midpoint - 1] as f64 + lengths[midpoint] as f64) / 2.0)
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
    }

    #[test]
    fn streams_metrics_from_path_without_retaining_sequence_bodies() {
        let metrics =
            AssemblyMetrics::from_path(Path::new("testdata/problem_assembly.fa"), &profile())
                .unwrap();

        assert_eq!(metrics.duplicate_id_count, 1);
        assert_eq!(metrics.invalid_sequence_count, 1);
    }
}
