//! Platform secure store (Windows Credential Manager via `keyring`; Android keystore
//! once the `android-native-keyring-store` feature is enabled for that target).

const SERVICE: &str = "com.hsilighting.renewal";

#[derive(Debug, thiserror::Error)]
pub enum SecureError {
    #[error("invalid key: only a-z, 0-9, '.', '_' and '-' are allowed")]
    InvalidKey,
    #[error("secure store error: {0}")]
    Store(#[from] keyring::Error),
}

fn entry(key: &str) -> Result<keyring::Entry, SecureError> {
    let valid = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if !valid {
        return Err(SecureError::InvalidKey);
    }
    Ok(keyring::Entry::new(SERVICE, key)?)
}

pub fn get(key: &str) -> Result<Option<String>, SecureError> {
    match entry(key)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn set(key: &str, value: &str) -> Result<(), SecureError> {
    Ok(entry(key)?.set_password(value)?)
}

pub fn delete(key: &str) -> Result<(), SecureError> {
    match entry(key)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
