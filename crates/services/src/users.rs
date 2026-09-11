//! User administration (Admin only — spec §18).

use renewal_core::{Capability, Role};
use renewal_db::{audit, sessions, users, PgPool};
use serde_json::json;
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::auth::{hash_password, verify_password};
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub use renewal_db::users::UserRow;

pub struct NewUserInput<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub role: Role,
    pub password: &'a str,
}

pub async fn list(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<UserRow>> {
    caller.require(Capability::ManageUsers)?;
    Ok(users::list(pool).await?)
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    input: NewUserInput<'_>,
) -> ServiceResult<UserRow> {
    caller.require(Capability::ManageUsers)?;
    let name = input.name.trim();
    let email = input.email.trim();
    if name.is_empty() || !email.contains('@') {
        return Err(ServiceError::validation(
            "name and a valid email are required",
        ));
    }
    let password_hash = hash_password(input.password)?;

    let mut tx = pool.begin().await?;
    if users::find_by_email(&mut *tx, email).await?.is_some() {
        return Err(ServiceError::Conflict(format!(
            "a user with email {email} already exists"
        )));
    }
    let user = users::insert(
        &mut *tx,
        &users::NewUser {
            name,
            email,
            role: &input.role.to_string(),
            password_hash: &password_hash,
        },
    )
    .await?;
    audit::record(
        &mut *tx,
        &audit::AuditEntry {
            actor_id: Some(caller.user_id),
            entity_type: "user",
            entity_id: Some(user.id),
            action: "CREATED",
            before: None,
            after: Some(json!({ "name": user.name, "email": user.email, "role": user.role, "active": user.active })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(user)
}

/// Deactivating a user also revokes their sessions so the change is immediate on every device.
pub async fn set_active(
    pool: &PgPool,
    caller: &Session,
    user_id: Uuid,
    active: bool,
) -> ServiceResult<UserRow> {
    caller.require(Capability::ManageUsers)?;
    if user_id == caller.user_id && !active {
        return Err(ServiceError::validation(
            "you cannot deactivate your own account",
        ));
    }
    let mut tx = pool.begin().await?;
    let Some(before) = users::find_by_id(&mut *tx, user_id).await? else {
        return Err(ServiceError::NotFound("user"));
    };
    users::set_active(&mut *tx, user_id, active).await?;
    if !active {
        sessions::revoke_all_for_user(&mut *tx, user_id).await?;
    }
    audit::record(
        &mut *tx,
        &audit::AuditEntry {
            actor_id: Some(caller.user_id),
            entity_type: "user",
            entity_id: Some(user_id),
            action: if active { "ACTIVATED" } else { "DEACTIVATED" },
            before: Some(json!({ "active": before.active })),
            after: Some(json!({ "active": active })),
        },
    )
    .await?;
    tx.commit().await?;
    users::find_by_id(pool, user_id)
        .await?
        .ok_or(ServiceError::NotFound("user"))
}

/// A user changes their own password; every other session they hold is revoked.
pub async fn change_password(
    pool: &PgPool,
    caller: &Session,
    current: &str,
    new: &str,
) -> ServiceResult<()> {
    let user = users::find_by_id(pool, caller.user_id)
        .await?
        .ok_or(ServiceError::NotFound("user"))?;
    if !verify_password(current, &user.password_hash) {
        return Err(ServiceError::validation(
            "the current password is incorrect",
        ));
    }
    let hash = hash_password(new)?;
    let mut tx = pool.begin().await?;
    users::set_password_hash(&mut *tx, caller.user_id, &hash).await?;
    sessions::revoke_all_except(&mut *tx, caller.user_id, caller.session_id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "user",
        caller.user_id,
        "PASSWORD_CHANGED",
        NONE,
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Admin sets a temporary password; the user is signed out everywhere.
pub async fn reset_password(
    pool: &PgPool,
    caller: &Session,
    user_id: Uuid,
    new: &str,
) -> ServiceResult<()> {
    caller.require(Capability::ManageUsers)?;
    let hash = hash_password(new)?;
    let mut tx = pool.begin().await?;
    users::find_by_id(&mut *tx, user_id)
        .await?
        .ok_or(ServiceError::NotFound("user"))?;
    users::set_password_hash(&mut *tx, user_id, &hash).await?;
    sessions::revoke_all_for_user(&mut *tx, user_id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "user",
        user_id,
        "PASSWORD_RESET",
        NONE,
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
