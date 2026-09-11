//! Status enums and their state machines (spec §3, §5, §7, §9, §11, §12; PLAN.md §3, §6.1–6.3).
//!
//! Only *stored* statuses appear here. "Expiring Soon", "Renewal In Progress" and
//! "Urgent" are derived overlays — see [`crate::expiry`].

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

use crate::error::TransitionError;

macro_rules! status_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
        #[cfg_attr(feature = "specta", derive(specta::Type))]
        pub enum $name { $($variant),+ }
    };
}

status_enum! {
    /// Stored contract status (PLAN.md §3.2). Draft → Active → Renewed | Expired | Terminated.
    ContractStatus { Draft, Active, Renewed, Expired, Terminated }
}

status_enum! {
    /// Unit occupancy status (spec §3). Vacant/Occupied follow contracts; Reserved/Maintenance are manual.
    UnitStatus { Vacant, Occupied, Reserved, Maintenance }
}

status_enum! {
    /// Renewal case status (spec §7 step 2) — the eleven workflow states.
    RenewalStatus {
        NotStarted,
        NoticePending,
        NoticeSent,
        WaitingForTenantResponse,
        TenantInterested,
        UnderNegotiation,
        RenewalConfirmed,
        RenewalCompleted,
        TenantNotRenewing,
        Vacating,
        Closed,
    }
}

status_enum! {
    /// Renewal notice status (spec §9).
    NoticeStatus { NotRequired, Pending, Draft, Sent, Delivered, Failed }
}

status_enum! {
    /// Tenant response recorded by the assigned employee (spec §11).
    TenantResponse { NoResponse, InterestedInRenewal, RenewalConfirmed, UnderDiscussion, NotInterested, WillVacate }
}

status_enum! {
    /// Follow-up channel (spec §12).
    FollowUpType { PhoneCall, Email, Meeting, WhatsApp, InternalDiscussion }
}

status_enum! {
    FollowUpStatus { Open, Done, Cancelled }
}

status_enum! {
    /// Outbound email lifecycle (PLAN.md §6.6). The worker drains `Queued`.
    EmailStatus { Queued, Sent, Delivered, Failed }
}

status_enum! {
    /// The four buckets used by the Renewal Status report (spec §17, PLAN.md §3.4).
    ReportBucket { Pending, InProgress, Completed, NotRenewing }
}

impl ContractStatus {
    pub fn can_transition_to(self, to: ContractStatus) -> bool {
        use ContractStatus::*;
        matches!(
            (self, to),
            (Draft, Active)
                | (Draft, Terminated)
                | (Active, Renewed)
                | (Active, Expired)
                | (Active, Terminated)
        )
    }

    pub fn transition(self, to: ContractStatus) -> Result<ContractStatus, TransitionError> {
        if self.can_transition_to(to) {
            Ok(to)
        } else {
            Err(TransitionError {
                entity: "contract",
                from: self.to_string(),
                to: to.to_string(),
            })
        }
    }

    /// A contract that still occupies its units and can be renewed.
    pub fn is_live(self) -> bool {
        self == ContractStatus::Active
    }
}

impl RenewalStatus {
    /// Allowed moves in the renewal workflow (PLAN.md §6.2). The main line runs forward one
    /// step at a time; the "not renewing" branch can be entered from any open state; any open
    /// state may be closed by an employee.
    pub fn can_transition_to(self, to: RenewalStatus) -> bool {
        use RenewalStatus::*;
        if self == Closed || self == to {
            return false;
        }
        match (self, to) {
            (NotStarted, NoticePending)
            | (NoticePending, NoticeSent)
            | (NoticeSent, WaitingForTenantResponse)
            | (WaitingForTenantResponse, TenantInterested)
            | (WaitingForTenantResponse, UnderNegotiation)
            | (TenantInterested, UnderNegotiation)
            | (TenantInterested, RenewalConfirmed)
            | (UnderNegotiation, RenewalConfirmed)
            | (RenewalConfirmed, RenewalCompleted)
            | (RenewalCompleted, Closed)
            | (TenantNotRenewing, Vacating)
            | (Vacating, Closed) => true,
            // Tenant declines (or announces vacating) from anywhere before completion.
            (from, TenantNotRenewing) if !matches!(from, RenewalCompleted | Vacating) => true,
            (from, Vacating) if from != RenewalCompleted => true,
            // Any open case may be closed manually.
            (_, Closed) => true,
            _ => false,
        }
    }

    pub fn transition(self, to: RenewalStatus) -> Result<RenewalStatus, TransitionError> {
        if self.can_transition_to(to) {
            Ok(to)
        } else {
            Err(TransitionError {
                entity: "renewal case",
                from: self.to_string(),
                to: to.to_string(),
            })
        }
    }

    pub fn is_open(self) -> bool {
        !matches!(
            self,
            RenewalStatus::RenewalCompleted | RenewalStatus::Closed
        )
    }

    /// Roll-up used by reports and dashboard cards (PLAN.md §3.4).
    pub fn report_bucket(self) -> ReportBucket {
        use RenewalStatus::*;
        match self {
            NotStarted | NoticePending => ReportBucket::Pending,
            NoticeSent
            | WaitingForTenantResponse
            | TenantInterested
            | UnderNegotiation
            | RenewalConfirmed => ReportBucket::InProgress,
            RenewalCompleted | Closed => ReportBucket::Completed,
            TenantNotRenewing | Vacating => ReportBucket::NotRenewing,
        }
    }
}

impl TenantResponse {
    /// Recording a response auto-moves the case (PLAN.md §3.3).
    pub fn implied_renewal_status(self) -> RenewalStatus {
        match self {
            TenantResponse::NoResponse => RenewalStatus::WaitingForTenantResponse,
            TenantResponse::InterestedInRenewal => RenewalStatus::TenantInterested,
            TenantResponse::UnderDiscussion => RenewalStatus::UnderNegotiation,
            TenantResponse::RenewalConfirmed => RenewalStatus::RenewalConfirmed,
            TenantResponse::NotInterested => RenewalStatus::TenantNotRenewing,
            TenantResponse::WillVacate => RenewalStatus::Vacating,
        }
    }
}

impl NoticeStatus {
    pub fn can_transition_to(self, to: NoticeStatus) -> bool {
        use NoticeStatus::*;
        matches!(
            (self, to),
            (NotRequired, Pending)
                | (Pending, Draft)
                | (Draft, Sent)
                | (Sent, Delivered)
                | (Sent, Failed)
                | (Failed, Sent)
                | (Pending, NotRequired)
                | (Draft, NotRequired)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn contract_lifecycle_is_draft_active_then_terminal() {
        use ContractStatus::*;
        assert!(Draft.can_transition_to(Active));
        assert!(Active.can_transition_to(Renewed));
        assert!(Active.can_transition_to(Expired));
        assert!(Active.can_transition_to(Terminated));
        for terminal in [Renewed, Expired, Terminated] {
            for to in ContractStatus::iter() {
                assert!(
                    !terminal.can_transition_to(to),
                    "{terminal} must be terminal"
                );
            }
        }
        assert!(!Draft.can_transition_to(Renewed));
        assert!(!Draft.can_transition_to(Expired));
        let err = Expired.transition(Active).unwrap_err();
        assert_eq!(
            err.to_string(),
            "cannot move contract from EXPIRED to ACTIVE"
        );
    }

    #[test]
    fn renewal_main_line_runs_forward() {
        use RenewalStatus::*;
        let line = [
            NotStarted,
            NoticePending,
            NoticeSent,
            WaitingForTenantResponse,
            TenantInterested,
            UnderNegotiation,
            RenewalConfirmed,
            RenewalCompleted,
            Closed,
        ];
        for pair in line.windows(2) {
            assert!(
                pair[0].can_transition_to(pair[1]),
                "{} -> {}",
                pair[0],
                pair[1]
            );
            assert!(
                !pair[1].can_transition_to(pair[0]),
                "{} must not go back to {}",
                pair[1],
                pair[0]
            );
        }
    }

    #[test]
    fn renewal_branch_and_close_rules() {
        use RenewalStatus::*;
        assert!(WaitingForTenantResponse.can_transition_to(TenantNotRenewing));
        assert!(NotStarted.can_transition_to(TenantNotRenewing));
        assert!(TenantNotRenewing.can_transition_to(Vacating));
        assert!(Vacating.can_transition_to(Closed));
        assert!(!RenewalCompleted.can_transition_to(TenantNotRenewing));
        assert!(!Vacating.can_transition_to(TenantNotRenewing));
        assert!(UnderNegotiation.can_transition_to(Closed));
        for to in RenewalStatus::iter() {
            assert!(!Closed.can_transition_to(to), "closed is terminal");
        }
        assert!(!NotStarted.can_transition_to(NotStarted));
    }

    #[test]
    fn responses_map_to_statuses_that_are_reachable_from_waiting() {
        for response in TenantResponse::iter() {
            let implied = response.implied_renewal_status();
            assert!(
                implied == RenewalStatus::WaitingForTenantResponse
                    || RenewalStatus::WaitingForTenantResponse.can_transition_to(implied)
                    || RenewalStatus::TenantInterested.can_transition_to(implied)
                    || RenewalStatus::UnderNegotiation.can_transition_to(implied),
                "{response} implies {implied}, which the workflow cannot reach"
            );
        }
    }

    #[test]
    fn report_buckets_match_plan() {
        use RenewalStatus::*;
        assert_eq!(NotStarted.report_bucket(), ReportBucket::Pending);
        assert_eq!(NoticePending.report_bucket(), ReportBucket::Pending);
        assert_eq!(NoticeSent.report_bucket(), ReportBucket::InProgress);
        assert_eq!(RenewalConfirmed.report_bucket(), ReportBucket::InProgress);
        assert_eq!(RenewalCompleted.report_bucket(), ReportBucket::Completed);
        assert_eq!(Closed.report_bucket(), ReportBucket::Completed);
        assert_eq!(TenantNotRenewing.report_bucket(), ReportBucket::NotRenewing);
        assert_eq!(Vacating.report_bucket(), ReportBucket::NotRenewing);
    }

    #[test]
    fn notice_can_be_retried_after_failure() {
        use NoticeStatus::*;
        assert!(Failed.can_transition_to(Sent));
        assert!(!Delivered.can_transition_to(Sent));
        assert!(!Sent.can_transition_to(Draft));
    }

    #[test]
    fn enums_serialize_as_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&RenewalStatus::WaitingForTenantResponse).unwrap(),
            "\"WAITING_FOR_TENANT_RESPONSE\""
        );
        assert_eq!(
            RenewalStatus::WaitingForTenantResponse.to_string(),
            "WAITING_FOR_TENANT_RESPONSE"
        );
        assert_eq!(
            "WHATS_APP".parse::<FollowUpType>().unwrap(),
            FollowUpType::WhatsApp
        );
    }
}
