//! Splitting a bill between the occupants of a unit (minor currency units, e.g. fils).
//!
//! Money is handled as integers so the shares always add up to the bill exactly;
//! any remainder from the division goes to the first shares, one minor unit each.

/// Splits `total` into `n` shares that sum to `total`. Empty when `n == 0`.
pub fn equal_split(total: i64, n: usize) -> Vec<i64> {
    if n == 0 {
        return Vec::new();
    }
    let n_i = n as i64;
    let base = total.div_euclid(n_i);
    let remainder = total.rem_euclid(n_i);
    (0..n_i)
        .map(|i| if i < remainder { base + 1 } else { base })
        .collect()
}

/// True when custom shares cover the bill exactly.
pub fn shares_cover(total: i64, shares: &[i64]) -> bool {
    shares.iter().all(|s| *s >= 0) && shares.iter().sum::<i64>() == total
}

/// Minor units -> "1,234.56" style string (major units with two decimals).
pub fn format_minor(amount: i64) -> String {
    let sign = if amount < 0 { "-" } else { "" };
    let abs = amount.abs();
    let major = abs / 100;
    let minor = abs % 100;
    let mut digits = major.to_string();
    let mut grouped = String::new();
    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        grouped = format!(",{tail}{grouped}");
    }
    format!("{sign}{digits}{grouped}.{minor:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_split_adds_up_and_spreads_the_remainder() {
        assert_eq!(equal_split(10_000, 3), vec![3_334, 3_333, 3_333]);
        assert_eq!(equal_split(10_000, 4), vec![2_500; 4]);
        assert_eq!(equal_split(7, 8), vec![1, 1, 1, 1, 1, 1, 1, 0]);
        assert!(equal_split(100, 0).is_empty());
        for n in 1..=9 {
            assert_eq!(equal_split(123_457, n).iter().sum::<i64>(), 123_457);
        }
    }

    #[test]
    fn custom_shares_must_cover_the_bill() {
        assert!(shares_cover(1_000, &[400, 600]));
        assert!(!shares_cover(1_000, &[400, 500]));
        assert!(!shares_cover(1_000, &[1_200, -200]));
    }

    #[test]
    fn formats_minor_units() {
        assert_eq!(format_minor(0), "0.00");
        assert_eq!(format_minor(5), "0.05");
        assert_eq!(format_minor(123_456_789), "1,234,567.89");
        assert_eq!(format_minor(-250), "-2.50");
    }
}
