//! Login, bootstrap of the first Admin, and bearer-token sessions.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chrono::{Duration, Utc};
use renewal_core::Role;
use renewal_db::{audit, sessions, users, PgPool};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

/// Sessions live this long; `last_seen_at` is bumped on use for auditing only.
pub const SESSION_TTL_DAYS: i64 = 30;
pub const MIN_PASSWORD_LEN: usize = 8;

pub fn hash_password(password: &str) -> ServiceResult<String> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(ServiceError::validation(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ServiceError::Internal(format!("password hashing failed: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Opaque token: 32 random bytes (two v4 UUIDs, OS-backed randomness), hex encoded.
fn new_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    hex::encode(bytes)
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub struct Issued {
    pub token: String,
    pub session: Session,
}

/// Verify credentials and issue a session token.
pub async fn login(
    pool: &PgPool,
    email: &str,
    password: &str,
    user_agent: Option<&str>,
) -> ServiceResult<Issued> {
    let email = email.trim();
    let user = users::find_by_email(pool, email).await?;
    // Always run the verifier so timing does not reveal whether the email exists.
    let dummy = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    let ok = match &user {
        Some(u) if u.active => verify_password(password, &u.password_hash),
        _ => {
            let _ = verify_password(password, dummy);
            false
        }
    };
    let Some(user) = user.filter(|_| ok) else {
        return Err(ServiceError::InvalidCredentials);
    };
    let role: Role = user
        .role
        .parse()
        .map_err(|_| ServiceError::Internal(format!("bad role {}", user.role)))?;

    let token = new_token();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    let mut tx = pool.begin().await?;
    let row = sessions::insert(
        &mut *tx,
        user.id,
        &token_hash(&token),
        user_agent,
        expires_at,
    )
    .await?;
    users::touch_last_login(&mut *tx, user.id).await?;
    audit::record(
        &mut *tx,
        &audit::AuditEntry {
            actor_id: Some(user.id),
            entity_type: "user",
            entity_id: Some(user.id),
            action: "LOGIN",
            before: None,
            after: Some(json!({ "sessionId": row.id, "userAgent": user_agent })),
        },
    )
    .await?;
    tx.commit().await?;

    Ok(Issued {
        token,
        session: Session {
            session_id: row.id,
            user_id: user.id,
            name: user.name,
            email: user.email,
            role,
            expires_at: row.expires_at,
        },
    })
}

/// Resolve a bearer token to a live session. `None` means "sign in again".
pub async fn authenticate(pool: &PgPool, token: &str) -> ServiceResult<Option<Session>> {
    let Some(row) = sessions::touch_by_token_hash(pool, &token_hash(token)).await? else {
        return Ok(None);
    };
    let Some(user) = users::find_by_id(pool, row.user_id).await? else {
        return Ok(None);
    };
    if !user.active {
        return Ok(None);
    }
    let role: Role = user
        .role
        .parse()
        .map_err(|_| ServiceError::Internal(format!("bad role {}", user.role)))?;
    Ok(Some(Session {
        session_id: row.id,
        user_id: user.id,
        name: user.name,
        email: user.email,
        role,
        expires_at: row.expires_at,
    }))
}

pub async fn logout(pool: &PgPool, session: &Session) -> ServiceResult<()> {
    let mut tx = pool.begin().await?;
    sessions::revoke(&mut *tx, session.session_id).await?;
    audit::record(
        &mut *tx,
        &audit::AuditEntry {
            actor_id: Some(session.user_id),
            entity_type: "user",
            entity_id: Some(session.user_id),
            action: "LOGOUT",
            before: None,
            after: Some(json!({ "sessionId": session.session_id })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Create the first Admin. Refused once any user exists (the Admin then adds users in Settings).
pub async fn bootstrap_admin(
    pool: &PgPool,
    name: &str,
    email: &str,
    password: &str,
) -> ServiceResult<Issued> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() || !email.contains('@') {
        return Err(ServiceError::validation(
            "name and a valid email are required",
        ));
    }
    let password_hash = hash_password(password)?;

    let mut tx = pool.begin().await?;
    if users::count(&mut *tx).await? > 0 {
        return Err(ServiceError::Conflict(
            "the system already has users; ask an Admin for an account".into(),
        ));
    }
    let user = users::insert(
        &mut *tx,
        &users::NewUser {
            name,
            email,
            role: &Role::Admin.to_string(),
            password_hash: &password_hash,
        },
    )
    .await?;
    audit::record(
        &mut *tx,
        &audit::AuditEntry {
            actor_id: Some(user.id),
            entity_type: "user",
            entity_id: Some(user.id),
            action: "BOOTSTRAP_ADMIN",
            before: None,
            after: Some(json!({ "name": user.name, "email": user.email, "role": user.role })),
        },
    )
    .await?;
    tx.commit().await?;

    login(pool, email, password, Some("bootstrap")).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!verify_password("anything", "not-a-hash"));
    }

    #[test]
    fn short_passwords_are_rejected() {
        assert!(matches!(
            hash_password("short"),
            Err(ServiceError::Validation(_))
        ));
    }

    #[test]
    fn tokens_are_unique_and_hashes_are_stable() {
        let a = new_token();
        let b = new_token();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert_eq!(token_hash(&a), token_hash(&a));
        assert_ne!(token_hash(&a), token_hash(&b));
        assert_eq!(token_hash(&a).len(), 64);
    }
}
