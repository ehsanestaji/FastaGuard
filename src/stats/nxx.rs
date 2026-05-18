use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nx {
    pub nx: u64,
    pub lx: u64,
}

pub fn nx_lx(lengths: &[u64], fraction: f64) -> Nx {
    if lengths.is_empty() || !fraction.is_finite() || fraction <= 0.0 || fraction > 1.0 {
        return Nx { nx: 0, lx: 0 };
    }

    let mut sorted = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let total: u128 = sorted.iter().map(|length| *length as u128).sum();
    let Some((numerator, denominator_shift)) = fraction_as_binary_ratio(fraction) else {
        return Nx { nx: 0, lx: 0 };
    };
    let target = ceil_scaled_by_power_of_two(total, numerator, denominator_shift);
    let mut cumulative = 0_u128;

    for (index, length) in sorted.iter().enumerate() {
        cumulative += *length as u128;
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

fn fraction_as_binary_ratio(fraction: f64) -> Option<(u64, u32)> {
    if !fraction.is_finite() || fraction <= 0.0 || fraction > 1.0 {
        return None;
    }

    let bits = fraction.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let significand_bits = bits & ((1_u64 << 52) - 1);
    let (mut numerator, mut denominator_shift) = if exponent == 0 {
        (significand_bits, 1074_u32)
    } else {
        ((1_u64 << 52) | significand_bits, (1075 - exponent) as u32)
    };

    let removable_powers = numerator.trailing_zeros().min(denominator_shift);
    numerator >>= removable_powers;
    denominator_shift -= removable_powers;

    Some((numerator, denominator_shift))
}

fn ceil_scaled_by_power_of_two(total: u128, numerator: u64, denominator_shift: u32) -> u128 {
    let product = U192::multiply_u128_by_u64(total, numerator);
    if product.is_zero() {
        return 0;
    }

    let mut low = 0_u128;
    let mut high = total;

    while low < high {
        let mid = low + ((high - low) / 2);
        if compare_shifted_u128_to_u192(mid, denominator_shift, product) == Ordering::Less {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    low
}

fn compare_shifted_u128_to_u192(value: u128, shift: u32, other: U192) -> Ordering {
    if value == 0 {
        return U192::ZERO.cmp(&other);
    }

    let shifted_bit_len = (u128::BITS - value.leading_zeros()) + shift;
    let other_bit_len = other.bit_len();
    match shifted_bit_len.cmp(&other_bit_len) {
        Ordering::Equal => U192::from_shifted_u128(value, shift).cmp(&other),
        ordering => ordering,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct U192 {
    high: u64,
    mid: u64,
    low: u64,
}

impl U192 {
    const ZERO: Self = Self {
        high: 0,
        mid: 0,
        low: 0,
    };

    fn multiply_u128_by_u64(value: u128, multiplier: u64) -> Self {
        let low_value = value as u64;
        let high_value = (value >> 64) as u64;

        let low_product = low_value as u128 * multiplier as u128;
        let high_product = high_value as u128 * multiplier as u128;
        let mid_with_carry = (high_product & u64::MAX as u128) + (low_product >> 64);

        Self {
            high: ((high_product >> 64) + (mid_with_carry >> 64)) as u64,
            mid: mid_with_carry as u64,
            low: low_product as u64,
        }
    }

    fn from_shifted_u128(value: u128, shift: u32) -> Self {
        let mut limbs = [value as u64, (value >> 64) as u64, 0_u64];
        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let input = limbs;
        limbs = [0, 0, 0];

        for (index, limb) in input.into_iter().take(2).enumerate() {
            let target_index = index + word_shift;
            if target_index >= limbs.len() {
                continue;
            }

            limbs[target_index] |= limb << bit_shift;
            if bit_shift > 0 && target_index + 1 < limbs.len() {
                limbs[target_index + 1] |= limb >> (64 - bit_shift);
            }
        }

        Self {
            high: limbs[2],
            mid: limbs[1],
            low: limbs[0],
        }
    }

    fn bit_len(self) -> u32 {
        if self.high != 0 {
            192 - self.high.leading_zeros()
        } else if self.mid != 0 {
            128 - self.mid.leading_zeros()
        } else if self.low != 0 {
            64 - self.low.leading_zeros()
        } else {
            0
        }
    }

    fn is_zero(self) -> bool {
        self == Self::ZERO
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
    fn accumulates_large_lengths_without_overflow() {
        let lengths = vec![u64::MAX, u64::MAX, 1];
        let result = nx_lx(&lengths, 1.0);
        assert_eq!(result, Nx { nx: 1, lx: 3 });
    }

    #[test]
    fn computes_target_without_large_total_precision_loss() {
        let lengths = vec![1 << 63, 1 << 63, 1];
        let result = nx_lx(&lengths, 0.5);
        assert_eq!(
            result,
            Nx {
                nx: 9_223_372_036_854_775_808,
                lx: 2,
            }
        );
    }

    #[test]
    fn invalid_fractions_return_zeroes() {
        let lengths = vec![10, 80, 20, 40];

        assert_eq!(nx_lx(&lengths, 0.0), Nx { nx: 0, lx: 0 });
        assert_eq!(nx_lx(&lengths, -0.1), Nx { nx: 0, lx: 0 });
        assert_eq!(nx_lx(&lengths, 1.1), Nx { nx: 0, lx: 0 });
        assert_eq!(nx_lx(&lengths, f64::NAN), Nx { nx: 0, lx: 0 });
    }

    #[test]
    fn empty_lengths_return_zeroes() {
        assert_eq!(nx_lx(&[], 0.50), Nx { nx: 0, lx: 0 });
    }
}
