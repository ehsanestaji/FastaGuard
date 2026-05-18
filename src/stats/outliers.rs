pub fn zscore_outlier_indices(values: &[f64], threshold: f64) -> Vec<usize> {
    if values.len() < 3 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_extreme_value() {
        let values = vec![50.0, 51.0, 49.0, 90.0, 50.5];
        assert_eq!(zscore_outlier_indices(&values, 1.5), vec![3]);
    }
}
