pub fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        round2((part as f64 / total as f64) * 100.0)
    }
}

pub fn fraction(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        part as f64 / total as f64
    }
}

pub fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_rounds_to_two_decimals() {
        assert_eq!(percent(1, 3), 33.33);
    }

    #[test]
    fn zero_total_is_zero() {
        assert_eq!(percent(4, 0), 0.0);
        assert_eq!(fraction(4, 0), 0.0);
    }
}
