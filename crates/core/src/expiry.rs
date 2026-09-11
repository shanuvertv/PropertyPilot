//! The expiry engine (spec §5, §6, §8; PLAN.md §3.1–3.2).
//!
//! One remaining-days value and one band scale, used everywhere:
//!
//! ```text
//!  Expired | 0–30 Urgent | 31–60 | 61–90 | 91–120 | > 120
//!    < 0        rule 30     rule 60  rule 90  rule 120   not due
//! ```

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::status::ContractStatus;

/// Admin-configurable thresholds (Settings). Defaults match the §8 reminder rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Thresholds {
    /// Contracts with this many days or fewer count as "Expiring Soon" and show on the Renewal Dashboard.
    pub expiring_soon_days: i64,
    /// Contracts with this many days or fewer are flagged Urgent.
    pub urgent_days: i64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            expiring_soon_days: 90,
            urgent_days: 30,
        }
    }
}

/// `contract_end_date − today` in calendar days. Negative once the end date has passed.
pub fn remaining_days(end_date: NaiveDate, today: NaiveDate) -> i64 {
    (end_date - today).num_days()
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Band {
    #[serde(rename = "EXPIRED")]
    #[strum(serialize = "EXPIRED")]
    Expired,
    #[serde(rename = "DAYS_0_TO_30")]
    #[strum(serialize = "DAYS_0_TO_30")]
    Days0To30,
    #[serde(rename = "DAYS_31_TO_60")]
    #[strum(serialize = "DAYS_31_TO_60")]
    Days31To60,
    #[serde(rename = "DAYS_61_TO_90")]
    #[strum(serialize = "DAYS_61_TO_90")]
    Days61To90,
    #[serde(rename = "DAYS_91_TO_120")]
    #[strum(serialize = "DAYS_91_TO_120")]
    Days91To120,
    #[serde(rename = "BEYOND_120")]
    #[strum(serialize = "BEYOND_120")]
    Beyond120,
}

impl Band {
    pub fn for_days(remaining: i64) -> Band {
        match remaining {
            d if d < 0 => Band::Expired,
            0..=30 => Band::Days0To30,
            31..=60 => Band::Days31To60,
            61..=90 => Band::Days61To90,
            91..=120 => Band::Days91To120,
            _ => Band::Beyond120,
        }
    }

    pub fn for_dates(end_date: NaiveDate, today: NaiveDate) -> Band {
        Band::for_days(remaining_days(end_date, today))
    }

    /// Human label used on dashboards and reports.
    pub fn label(self) -> &'static str {
        match self {
            Band::Expired => "Expired",
            Band::Days0To30 => "Expiring in 0–30 days",
            Band::Days31To60 => "Expiring in 31–60 days",
            Band::Days61To90 => "Expiring in 61–90 days",
            Band::Days91To120 => "Expiring in 91–120 days",
            Band::Beyond120 => "More than 120 days",
        }
    }
}

/// Derived overlays on top of the stored contract status (PLAN.md §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Overlay {
    pub remaining_days: i64,
    pub band: Band,
    pub expiring_soon: bool,
    pub urgent: bool,
    pub renewal_in_progress: bool,
}

impl Overlay {
    pub fn compute(
        status: ContractStatus,
        end_date: NaiveDate,
        today: NaiveDate,
        has_open_case: bool,
        thresholds: Thresholds,
    ) -> Overlay {
        let rd = remaining_days(end_date, today);
        let live = status.is_live();
        Overlay {
            remaining_days: rd,
            band: Band::for_days(rd),
            expiring_soon: live && rd >= 0 && rd <= thresholds.expiring_soon_days,
            urgent: live && rd >= 0 && rd <= thresholds.urgent_days,
            renewal_in_progress: live && has_open_case,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn remaining_days_is_end_minus_today() {
        let today = d(2026, 9, 11);
        assert_eq!(remaining_days(d(2026, 9, 11), today), 0);
        assert_eq!(remaining_days(d(2026, 9, 12), today), 1);
        assert_eq!(remaining_days(d(2026, 9, 10), today), -1);
        assert_eq!(remaining_days(d(2027, 9, 11), today), 365);
    }

    #[test]
    fn band_boundaries_from_plan_phase_2() {
        let cases = [
            (-1, Band::Expired),
            (0, Band::Days0To30),
            (30, Band::Days0To30),
            (31, Band::Days31To60),
            (60, Band::Days31To60),
            (61, Band::Days61To90),
            (90, Band::Days61To90),
            (91, Band::Days91To120),
            (120, Band::Days91To120),
            (121, Band::Beyond120),
            (-400, Band::Expired),
            (4000, Band::Beyond120),
        ];
        for (days, band) in cases {
            assert_eq!(Band::for_days(days), band, "{days} days");
        }
    }

    #[test]
    fn overlay_uses_thresholds_and_ignores_non_live_contracts() {
        let today = d(2026, 9, 11);
        let t = Thresholds::default();
        let o = Overlay::compute(ContractStatus::Active, d(2026, 10, 1), today, true, t);
        assert_eq!(o.remaining_days, 20);
        assert!(o.expiring_soon && o.urgent && o.renewal_in_progress);

        let o = Overlay::compute(ContractStatus::Active, d(2026, 12, 10), today, false, t);
        assert_eq!(o.remaining_days, 90);
        assert!(o.expiring_soon && !o.urgent && !o.renewal_in_progress);

        let o = Overlay::compute(ContractStatus::Active, d(2026, 12, 11), today, false, t);
        assert_eq!(o.remaining_days, 91);
        assert!(!o.expiring_soon);

        // Expired contracts are past the window, not "expiring soon".
        let o = Overlay::compute(ContractStatus::Active, d(2026, 9, 1), today, false, t);
        assert_eq!(o.band, Band::Expired);
        assert!(!o.expiring_soon && !o.urgent);

        // Renewed / terminated contracts never carry overlays even if their dates are near.
        let o = Overlay::compute(ContractStatus::Renewed, d(2026, 9, 20), today, true, t);
        assert!(!o.expiring_soon && !o.urgent && !o.renewal_in_progress);
    }

    #[test]
    fn band_names_match_the_sql_view() {
        use strum::IntoEnumIterator;
        let names: Vec<String> = Band::iter().map(|b| b.to_string()).collect();
        assert_eq!(
            names,
            [
                "EXPIRED",
                "DAYS_0_TO_30",
                "DAYS_31_TO_60",
                "DAYS_61_TO_90",
                "DAYS_91_TO_120",
                "BEYOND_120"
            ]
        );
        assert_eq!("DAYS_91_TO_120".parse::<Band>().unwrap(), Band::Days91To120);
        assert_eq!(
            serde_json::to_string(&Band::Beyond120).unwrap(),
            "\"BEYOND_120\""
        );
    }

    #[test]
    fn custom_thresholds_are_respected() {
        let today = d(2026, 9, 11);
        let t = Thresholds {
            expiring_soon_days: 120,
            urgent_days: 15,
        };
        let o = Overlay::compute(ContractStatus::Active, d(2027, 1, 1), today, false, t);
        assert_eq!(o.remaining_days, 112);
        assert!(o.expiring_soon && !o.urgent);
    }
}
