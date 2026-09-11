//! Copies messages sent over SMTP into the mailbox's *Sent* folder over IMAP.
//!
//! SMTP submission only hands a message to the outgoing server; nothing lands in
//! the sender's Sent folder (Microsoft 365 and most hosts behave this way). When the
//! leasing mailbox is an ordinary IMAP account, appending the exact bytes we sent
//! keeps the team's mail client showing the notices it sent from PropertyPilot.
//! Best effort: a failure here is logged and never fails the send itself.

use imap::types::Flag;
use imap::{ClientBuilder, ConnectionMode, TlsKind};
use imap_proto::NameAttribute;

#[derive(Debug, Clone)]
pub struct ImapSentConfig {
    pub host: String,
    /// 993 (implicit TLS) or 143 (STARTTLS); the mode is detected from the port.
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Folder name; when `None` the server's `\Sent` special-use folder is looked up.
    pub folder: Option<String>,
}

/// Well-known Sent folder names, tried when the server does not advertise `\Sent`.
const FALLBACK_FOLDERS: &[&str] = &[
    "Sent Items",
    "Sent",
    "INBOX.Sent",
    "[Gmail]/Sent Mail",
    "Sent Messages",
];

fn find_sent_folder(session: &mut imap::Session<imap::Connection>) -> imap::Result<String> {
    let names = session.list(None, Some("*"))?;
    if let Some(n) = names.iter().find(|n| {
        n.attributes()
            .iter()
            .any(|a| matches!(a, NameAttribute::Sent))
    }) {
        return Ok(n.name().to_owned());
    }
    for candidate in FALLBACK_FOLDERS {
        if names
            .iter()
            .any(|n| n.name().eq_ignore_ascii_case(candidate))
        {
            return Ok((*candidate).to_owned());
        }
    }
    Ok("Sent".to_owned())
}

/// Blocking: run inside `spawn_blocking`.
pub fn append_to_sent(cfg: &ImapSentConfig, raw_message: &[u8]) -> imap::Result<String> {
    let client = ClientBuilder::new(&cfg.host, cfg.port)
        .mode(ConnectionMode::AutoTls)
        .tls_kind(TlsKind::Rust)
        .connect()?;
    let mut session = client
        .login(&cfg.username, &cfg.password)
        .map_err(|(e, _)| e)?;
    let folder = match &cfg.folder {
        Some(f) => f.clone(),
        None => find_sent_folder(&mut session)?,
    };
    session
        .append(&folder, raw_message)
        .flag(Flag::Seen)
        .finish()?;
    let _ = session.logout();
    Ok(folder)
}
