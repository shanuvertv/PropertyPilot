//! Use-cases. Each public function takes the caller's [`Session`] (or none for
//! login/bootstrap), checks the permission matrix from `renewal_core`, performs
//! the change inside a transaction, and writes the audit entry alongside it.

pub mod audit;
pub mod audit_log;
pub mod auth;
pub mod buildings;
pub mod contracts;
pub mod dashboard;
pub mod documents;
pub mod emails;
pub mod error;
pub mod expenses;
pub mod follow_ups;
pub mod import;
pub mod mail_settings;
pub mod notices;
pub mod notifications;
pub mod occupants;
pub mod pdf;
pub mod providers;
pub mod renewals;
pub mod reports;
pub mod session;
pub mod settings;
pub mod sweep;
pub mod system;
pub mod templates;
pub mod tenants;
pub mod units;
pub mod users;

pub use error::{ServiceError, ServiceResult};
pub use session::Session;
