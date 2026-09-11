//! Platform secure store for the server URL and session token.
//!
//! * Desktop: the OS credential store via `keyring` (Windows Credential Manager,
//!   macOS Keychain, Secret Service on Linux).
//! * Android: a file in the app's private data directory. Android sandboxes that
//!   directory per app UID (mode 0600, inaccessible to other apps without root),
//!   which is what Google now recommends for tokens since it deprecated
//!   `EncryptedSharedPreferences` in 2024. Hardware-backed encryption can be layered
//!   on later with the `android-native-keyring-store` crate once the Gradle project
//!   carries its small Kotlin shim.

#[derive(Debug, thiserror::Error)]
pub enum SecureError {
    #[error("invalid key: only a-z, 0-9, '.', '_' and '-' are allowed")]
    InvalidKey,
    #[cfg(not(target_os = "android"))]
    #[error("secure store error: {0}")]
    Store(#[from] keyring::Error),
    #[cfg(target_os = "android")]
    #[error("secure store error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(target_os = "android")]
    #[error("secure store unavailable: {0}")]
    Path(String),
}

fn check_key(key: &str) -> Result<(), SecureError> {
    let valid = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(SecureError::InvalidKey)
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::{check_key, SecureError};

    const SERVICE: &str = "com.hsilighting.renewal";

    fn entry(key: &str) -> Result<keyring::Entry, SecureError> {
        check_key(key)?;
        Ok(keyring::Entry::new(SERVICE, key)?)
    }

    pub fn get(_app: &tauri::AppHandle, key: &str) -> Result<Option<String>, SecureError> {
        match entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(_app: &tauri::AppHandle, key: &str, value: &str) -> Result<(), SecureError> {
        Ok(entry(key)?.set_password(value)?)
    }

    pub fn delete(_app: &tauri::AppHandle, key: &str) -> Result<(), SecureError> {
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(target_os = "android")]
mod imp {
    use std::fs;
    use std::path::PathBuf;

    use tauri::Manager;

    use super::{check_key, SecureError};

    fn file(app: &tauri::AppHandle, key: &str) -> Result<PathBuf, SecureError> {
        check_key(key)?;
        let dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| SecureError::Path(e.to_string()))?
            .join("secure");
        fs::create_dir_all(&dir)?;
        Ok(dir.join(format!("{key}.txt")))
    }

    pub fn get(app: &tauri::AppHandle, key: &str) -> Result<Option<String>, SecureError> {
        match fs::read_to_string(file(app, key)?) {
            Ok(v) => Ok(Some(v)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(app: &tauri::AppHandle, key: &str, value: &str) -> Result<(), SecureError> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let path = file(app, key)?;
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(value.as_bytes())?;
        Ok(())
    }

    pub fn delete(app: &tauri::AppHandle, key: &str) -> Result<(), SecureError> {
        match fs::remove_file(file(app, key)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

pub use imp::{delete, get, set};
