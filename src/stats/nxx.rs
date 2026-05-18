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

    let target = (sorted.len() as f64 * fraction).ceil().max(1.0) as usize;
    let index = target.saturating_sub(1).min(sorted.len() - 1);

    Nx {
        nx: sorted[index],
        lx: (index + 1) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_n50_and_l50_for_sorted_or_unsorted_lengths() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.50);
        assert_eq!(result, Nx { nx: 40, lx: 2 });
    }

    #[test]
    fn computes_n90_and_l90() {
        let lengths = vec![10, 80, 20, 40];
        let result = nx_lx(&lengths, 0.90);
        assert_eq!(result, Nx { nx: 10, lx: 4 });
    }

    #[test]
    fn empty_lengths_return_zeroes() {
        assert_eq!(nx_lx(&[], 0.50), Nx { nx: 0, lx: 0 });
    }
}
