//! Integration seams. The server picks concrete implementations from its config;
//! everything above this line only sees the traits.
//!
//! Mail: log (dev), SMTP (any mailbox, optionally mirrored to its IMAP Sent folder)
//! or Microsoft Graph. Storage: PostgreSQL blobs (default) or in-memory for tests.

pub mod dynamic;
pub mod graph;
pub mod imap_sent;
pub mod mail;
pub mod pg_storage;
pub mod smtp;
pub mod storage;

pub use dynamic::{DynamicMail, MailDescription};
pub use graph::{GraphConfig, GraphMail};
pub use imap_sent::ImapSentConfig;
pub use mail::{LogMail, MailError, MailProvider, OutboundMail, SendReceipt};
pub use pg_storage::PgStorage;
pub use smtp::{SmtpConfig, SmtpMail};
pub use storage::{MemoryStorage, StorageError, StorageProvider, StoredObject};
