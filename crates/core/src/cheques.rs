//! Post-dated rent cheques: statuses and the plan for splitting a contract's rent into
//! N cheques spread over the contract period.

use chrono::{Datelike, Months, NaiveDate};

use crate::equal_split;

/// One planned cheque: when it falls due and for how much (minor units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedCheque {
    pub seq: u32,
    pub due_date: NaiveDate,
    pub amount_minor: i64,
}

/// `n` cheques of equal amounts (remainder on the first ones) starting on `first` and
/// then every `every_months` months; `every_months == 0` puts them all on `first`.
pub fn plan_cheques(
    total_minor: i64,
    n: usize,
    first: NaiveDate,
    every_months: u32,
) -> Vec<PlannedCheque> {
    equal_split(total_minor, n)
        .into_iter()
        .enumerate()
        .map(|(i, amount_minor)| PlannedCheque {
            seq: i as u32 + 1,
            due_date: add_months(first, every_months * i as u32),
            amount_minor,
        })
        .collect()
}

/// `date` plus `months`, clamped to the end of the target month (31 Jan + 1 → 28/29 Feb).
pub fn add_months(date: NaiveDate, months: u32) -> NaiveDate {
    date.checked_add_months(Months::new(months)).unwrap_or(date)
}

/// Evenly spaced cheques across a contract: for a 12-month contract and 4 cheques that
/// is every 3 months; anything that does not divide falls back to monthly-ish spacing.
pub fn spacing_months(start: NaiveDate, end: NaiveDate, n: usize) -> u32 {
    if n <= 1 {
        return 0;
    }
    let months = (end.year() - start.year()) * 12 + (end.month() as i32 - start.month() as i32) + 1;
    let per = months.max(1) as u32 / n as u32;
    per.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn four_quarterly_cheques_add_up() {
        let plan = plan_cheques(100_001, 4, d(2026, 1, 31), 3);
        assert_eq!(plan.len(), 4);
        assert_eq!(plan.iter().map(|c| c.amount_minor).sum::<i64>(), 100_001);
        assert_eq!(plan[0].amount_minor, 25_001);
        assert_eq!(plan[1].due_date, d(2026, 4, 30));
        assert_eq!(plan[3].due_date, d(2026, 10, 31));
        assert_eq!(plan[3].seq, 4);
    }

    #[test]
    fn spacing_follows_the_contract_length() {
        assert_eq!(spacing_months(d(2026, 9, 1), d(2027, 8, 31), 4), 3);
        assert_eq!(spacing_months(d(2026, 9, 1), d(2027, 8, 31), 12), 1);
        assert_eq!(spacing_months(d(2026, 9, 1), d(2027, 8, 31), 1), 0);
        assert_eq!(spacing_months(d(2026, 9, 1), d(2026, 10, 31), 6), 1);
    }
}
