//! Pure domain logic for the Rental Contract Renewal Management System.
//!
//! Nothing in this crate performs IO. Every rule that the desktop app and the
//! worker share — who may do what, which status may follow which, how many days
//! a contract has left — lives here so it can be unit-tested by itself.
//!
//! See `PLAN.md` §3 (spec clarifications) and §6 (core logic).

pub mod error;
pub mod expiry;
pub mod roles;
pub mod split;
pub mod status;
pub mod sweep;

pub use error::{PermissionDenied, TransitionError};
pub use expiry::{remaining_days, Band, Overlay, Thresholds};
pub use roles::{Capability, Role};
pub use split::{equal_split, format_minor, settled_amount};
pub use status::{
    ContractStatus, EmailStatus, ExpenseCategory, FollowUpStatus, FollowUpType, NoticeStatus,
    RenewalStatus, ReportBucket, SplitMethod, TenantResponse, UnitStatus,
};
pub use sweep::{
    plan as plan_sweep, Action as SweepAction, ReminderRule, SweepContract, SweepInput,
};
