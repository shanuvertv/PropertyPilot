//! The daily expiry sweep planner (spec §8; PLAN.md §6.5).
//!
//! Pure: takes today's snapshot of contracts, the Admin's reminder rules and what
//! has already been dispatched, and returns the actions to perform. The server
//! applies them; tests time-travel here without a database.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Admin-configurable milestone (spec §8 "Default reminder schedule").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ReminderRule {
    pub id: Uuid,
    pub days_before: i64,
    pub label: String,
    pub notify_in_app: bool,
    pub email_assigned_employee: bool,
    pub mark_urgent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweepContract {
    pub id: Uuid,
    pub remaining_days: i64,
    pub has_open_case: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Fire this rule for the contract: notify / email / flag urgent, and record the dispatch.
    Fire { contract_id: Uuid, rule_id: Uuid },
    /// A looser milestone that was overtaken (contract entered the window late): record only.
    Skip { contract_id: Uuid, rule_id: Uuid },
    /// Contract is within the expiring-soon window with no renewal case yet.
    OpenCase { contract_id: Uuid },
    /// End date has passed: mark Expired and release units.
    Expire { contract_id: Uuid },
}

pub struct SweepInput<'a> {
    pub contracts: &'a [SweepContract],
    pub rules: &'a [ReminderRule],
    /// (contract_id, rule_id) pairs already fired or skipped.
    pub dispatched: &'a [(Uuid, Uuid)],
    /// `Some(days)` auto-opens a renewal case at that threshold (Q5); `None` leaves it manual.
    pub auto_open_case_days: Option<i64>,
}

/// Plan one sweep. Rules fire once per contract; when several milestones are
/// pending at once (a contract entered late, or the sweep missed days) only the
/// tightest fires and the rest are skipped, so nobody gets four reminders at once.
pub fn plan(input: &SweepInput<'_>) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut rules: Vec<&ReminderRule> = input.rules.iter().collect();
    rules.sort_by_key(|r| r.days_before);

    for c in input.contracts {
        if c.remaining_days < 0 {
            actions.push(Action::Expire { contract_id: c.id });
            continue;
        }
        let pending: Vec<&ReminderRule> = rules
            .iter()
            .copied()
            .filter(|r| c.remaining_days <= r.days_before)
            .filter(|r| {
                !input
                    .dispatched
                    .iter()
                    .any(|(cid, rid)| *cid == c.id && *rid == r.id)
            })
            .collect();
        if let Some((tightest, rest)) = pending.split_first() {
            actions.push(Action::Fire {
                contract_id: c.id,
                rule_id: tightest.id,
            });
            for r in rest {
                actions.push(Action::Skip {
                    contract_id: c.id,
                    rule_id: r.id,
                });
            }
        }
        if let Some(days) = input.auto_open_case_days {
            if !c.has_open_case && c.remaining_days <= days {
                actions.push(Action::OpenCase { contract_id: c.id });
            }
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(days: i64) -> ReminderRule {
        ReminderRule {
            id: Uuid::new_v4(),
            days_before: days,
            label: format!("{days} days"),
            notify_in_app: true,
            email_assigned_employee: days <= 60,
            mark_urgent: days == 30,
        }
    }

    fn default_rules() -> Vec<ReminderRule> {
        [120, 90, 60, 30, 15, 7].into_iter().map(rule).collect()
    }

    fn fires(actions: &[Action]) -> Vec<Uuid> {
        actions
            .iter()
            .filter_map(|a| {
                if let Action::Fire { rule_id, .. } = a {
                    Some(*rule_id)
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn each_rule_fires_exactly_once_when_days_pass() {
        let rules = default_rules();
        let id = Uuid::new_v4();
        let mut dispatched: Vec<(Uuid, Uuid)> = vec![];
        let mut fired = vec![];
        // Walk from 130 days out down to expiry, one sweep per day.
        for remaining in (-1..=130).rev() {
            let contracts = [SweepContract {
                id,
                remaining_days: remaining,
                has_open_case: true,
            }];
            let actions = plan(&SweepInput {
                contracts: &contracts,
                rules: &rules,
                dispatched: &dispatched,
                auto_open_case_days: None,
            });
            for a in &actions {
                match a {
                    Action::Fire { rule_id, .. } | Action::Skip { rule_id, .. } => {
                        dispatched.push((id, *rule_id))
                    }
                    _ => {}
                }
            }
            fired.extend(fires(&actions));
            if remaining < 0 {
                assert!(actions.contains(&Action::Expire { contract_id: id }));
            }
        }
        let expected: Vec<Uuid> = rules.iter().map(|r| r.id).collect();
        // Fired in order 120, 90, 60, 30, 15, 7 — each once.
        assert_eq!(fired, expected);
    }

    #[test]
    fn late_contract_fires_only_the_tightest_and_skips_the_rest() {
        let rules = default_rules();
        let id = Uuid::new_v4();
        let contracts = [SweepContract {
            id,
            remaining_days: 20,
            has_open_case: false,
        }];
        let actions = plan(&SweepInput {
            contracts: &contracts,
            rules: &rules,
            dispatched: &[],
            auto_open_case_days: Some(90),
        });
        let fired = fires(&actions);
        assert_eq!(fired, vec![rules[3].id], "only the 30-day rule fires");
        let skipped: Vec<Uuid> = actions
            .iter()
            .filter_map(|a| {
                if let Action::Skip { rule_id, .. } = a {
                    Some(*rule_id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            skipped,
            vec![rules[2].id, rules[1].id, rules[0].id],
            "60/90/120 are skipped"
        );
        assert!(actions.contains(&Action::OpenCase { contract_id: id }));
        // Next day nothing new fires; 15 fires at 15.
        let mut dispatched: Vec<(Uuid, Uuid)> = actions
            .iter()
            .filter_map(|a| match a {
                Action::Fire { rule_id, .. } | Action::Skip { rule_id, .. } => Some((id, *rule_id)),
                _ => None,
            })
            .collect();
        let contracts = [SweepContract {
            id,
            remaining_days: 19,
            has_open_case: true,
        }];
        let again = plan(&SweepInput {
            contracts: &contracts,
            rules: &rules,
            dispatched: &dispatched,
            auto_open_case_days: Some(90),
        });
        assert!(
            fires(&again).is_empty() && !again.iter().any(|a| matches!(a, Action::OpenCase { .. }))
        );
        let contracts = [SweepContract {
            id,
            remaining_days: 15,
            has_open_case: true,
        }];
        let at15 = plan(&SweepInput {
            contracts: &contracts,
            rules: &rules,
            dispatched: &dispatched,
            auto_open_case_days: Some(90),
        });
        assert_eq!(fires(&at15), vec![rules[4].id]);
        dispatched.push((id, rules[4].id));
    }

    #[test]
    fn far_out_contracts_are_untouched() {
        let rules = default_rules();
        let contracts = [SweepContract {
            id: Uuid::new_v4(),
            remaining_days: 200,
            has_open_case: false,
        }];
        let actions = plan(&SweepInput {
            contracts: &contracts,
            rules: &rules,
            dispatched: &[],
            auto_open_case_days: Some(90),
        });
        assert!(actions.is_empty());
    }
}
