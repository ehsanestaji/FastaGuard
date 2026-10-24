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
    pub expected_size_bases: Option<u64>,
    pub expected_size_tolerance: Option<f64>,
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
            expected_size_bases: overrides.expected_size_bases,
            expected_size_tolerance: overrides.expected_size_tolerance,
        }
    }

    pub fn protein(overrides: ThresholdOverrides) -> Self {
        Self {
            name: "protein".to_string(),
            high_n_sequence_fraction: 0.20,
            high_global_n_fraction: overrides.max_n_rate.unwrap_or(0.05),
            min_contig_length: overrides.min_contig_length.unwrap_or(10),
            max_gap_run: 100,
            gc_outlier_zscore: 3.0,
            expected_size_bases: overrides.expected_size_bases,
            expected_size_tolerance: overrides.expected_size_tolerance,
        }
    }

    pub fn from_name(name: &str, overrides: ThresholdOverrides) -> Self {
        match name {
            "protein" => Self::protein(overrides),
            _ => Self::assembly(overrides),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_preserves_expected_size_thresholds() {
        let profile = ProfileConfig::assembly(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: None,
            expected_size_bases: Some(5_000_000),
            expected_size_tolerance: Some(0.25),
        });

        assert_eq!(profile.expected_size_bases, Some(5_000_000));
        assert_eq!(profile.expected_size_tolerance, Some(0.25));
    }

    #[test]
    fn protein_uses_default_short_sequence_threshold_and_respects_overrides() {
        let default_profile = ProfileConfig::protein(ThresholdOverrides {
            max_n_rate: None,
            min_contig_length: None,
            expected_size_bases: None,
            expected_size_tolerance: None,
        });
        assert_eq!(default_profile.name, "protein");
        assert_eq!(default_profile.min_contig_length, 10);
        assert_eq!(default_profile.high_global_n_fraction, 0.05);

        let custom_profile = ProfileConfig::from_name(
            "protein",
            ThresholdOverrides {
                max_n_rate: Some(0.10),
                min_contig_length: Some(30),
                expected_size_bases: None,
                expected_size_tolerance: None,
            },
        );
        assert_eq!(custom_profile.name, "protein");
        assert_eq!(custom_profile.min_contig_length, 30);
        assert_eq!(custom_profile.high_global_n_fraction, 0.10);
    }
}
