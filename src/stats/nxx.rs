#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nx {
    pub nx: u64,
    pub lx: u64,
}

pub fn nx_lx(lengths: &[u64], fraction: f64) -> Nx {
    if lengths.is_empty() {
        return Nx { nx: 0, lx: 0 };
    }

    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let total: u64 = sorted.iter().sum();
    let target = (total as f64 * fraction).ceil() as u64;
    let mut cumulative = 0_u64;

    for (index, length) in sorted.iter().enumerate() {
        cumulative += *length;
        if cumulative >= target {
            return Nx {
                nx: *length,
                lx: (index + 1) as u64,
            };
        }
    }

    Nx {
        nx: *sorted.last().unwrap_or(&0),
        lx: sorted.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_n50_and_l50_by_cumulative_length() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.50);
        assert_eq!(result, Nx { nx: 80, lx: 1 });
    }

    #[test]
    fn computes_n90_and_l90() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.90);
        assert_eq!(result, Nx { nx: 20, lx: 3 });
    }

    #[test]
    fn computes_n50_when_multiple_sequences_reach_target() {
        let lengths = vec![50, 40, 30, 20, 10];
        let result = nx_lx(&lengths, 0.50);
        assert_eq!(result, Nx { nx: 40, lx: 2 });
    }

    #[test]
    fn empty_lengths_return_zeroes() {
        assert_eq!(nx_lx(&[], 0.50), Nx { nx: 0, lx: 0 });
    }
}
