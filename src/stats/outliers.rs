pub fn zscore_outlier_indices(values: &[f64], threshold: f64) -> Vec<usize> {
    if values.len() < 3
        || !threshold.is_finite()
        || threshold <= 0.0
        || values.iter().any(|value| !value.is_finite())
    {
        return Vec::new();
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
        return Vec::new();
    }

    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let z = (value - mean).abs() / stddev;
            (z >= threshold).then_some(index)
        })
        .collect()
}

pub fn iqr_outlier_indices(values: &[u64], multiplier: f64) -> Vec<usize> {
    if values.len() < 4 || !multiplier.is_finite() || multiplier <= 0.0 {
        return Vec::new();
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let q1 = percentile(&sorted, 0.25);
    let q3 = percentile(&sorted, 0.75);
    let iqr = q3 - q1;
    if iqr <= 0.0 {
        return Vec::new();
    }

    let lower = q1 - multiplier * iqr;
    let upper = q3 + multiplier * iqr;

    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let value = *value as f64;
            (value < lower || value > upper).then_some(index)
        })
        .collect()
}

fn percentile(sorted: &[u64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = quantile * (sorted.len().saturating_sub(1) as f64);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower] as f64
    } else {
        let weight = rank - lower as f64;
        sorted[lower] as f64 * (1.0 - weight) + sorted[upper] as f64 * weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_extreme_value() {
        let values = vec![50.0, 51.0, 49.0, 90.0, 50.5];
        assert_eq!(zscore_outlier_indices(&values, 1.5), vec![3]);
    }

    #[test]
    fn invalid_threshold_returns_empty_indices() {
        let values = vec![50.0, 51.0, 49.0, 90.0, 50.5];
        assert_eq!(zscore_outlier_indices(&values, -1.5), Vec::<usize>::new());
    }

    #[test]
    fn non_finite_values_return_empty_indices() {
        let values = vec![50.0, f64::NAN, 49.0, 90.0, 50.5];
        assert_eq!(zscore_outlier_indices(&values, 1.5), Vec::<usize>::new());
    }

    #[test]
    fn iqr_finds_low_and_high_length_outliers() {
        let values = vec![100, 101, 102, 103, 104, 105, 10_000];
        let outliers = iqr_outlier_indices(&values, 1.5);
        assert_eq!(outliers, vec![6]);
    }

    #[test]
    fn iqr_returns_empty_for_short_or_flat_inputs() {
        assert_eq!(iqr_outlier_indices(&[100, 101], 1.5), Vec::<usize>::new());
        assert_eq!(
            iqr_outlier_indices(&[100, 100, 100, 100], 1.5),
            Vec::<usize>::new()
        );
    }
}
