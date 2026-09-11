//! Integration seams. The server picks concrete implementations from its config;
//! everything above this line only sees the traits.
//!
//! Phase 0 ships the traits plus in-memory/log implementations for development and
//! tests. Microsoft Graph / SMTP mail and Azure Blob / file-share storage arrive in
//! Phase 5 (PLAN.md D6, D10).

pub mod graph;
pub mod mail;
pub mod pg_storage;
pub mod smtp;
pub mod storage;

pub use graph::{GraphConfig, GraphMail};
pub use mail::{LogMail, MailError, MailProvider, OutboundMail, SendReceipt};
pub use pg_storage::PgStorage;
pub use smtp::{SmtpConfig, SmtpMail};
pub use storage::{MemoryStorage, StorageError, StorageProvider, StoredObject};
