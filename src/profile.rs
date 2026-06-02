#[derive(Debug, Clone, Copy)]
pub struct ThresholdOverrides {
    pub max_n_rate: Option<f64>,
    pub min_contig_length: Option<u64>,
    pub expected_size_bases: Option<u64>,
    pub expected_size_tolerance: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ProfileConfig {
    pub name: String,
    pub high_n_sequence_fraction: f64,
    pub high_global_n_fraction: f64,
    pub min_contig_length: u64,
    pub max_gap_run: u64,
    pub gc_outlier_zscore: f64,
}

impl ProfileConfig {
    pub fn assembly(overrides: ThresholdOverrides) -> Self {
        Self {
            name: "assembly".to_string(),
            high_n_sequence_fraction: 0.20,
            high_global_n_fraction: overrides.max_n_rate.unwrap_or(0.05),
            min_contig_length: overrides.min_contig_length.unwrap_or(200),
            max_gap_run: 100,
            gc_outlier_zscore: 3.0,
        }
    }
}
