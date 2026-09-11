//! In-app notification centre (spec §15).

use renewal_db::automation::{self, NewNotification, NotificationRow};
use renewal_db::PgPool;
use uuid::Uuid;

use crate::error::ServiceResult;
use crate::session::Session;

/// Who receives a contract/case alert: the assigned employee, or every Admin when unassigned.
pub fn audience(assigned: Option<Uuid>, admins: &[Uuid]) -> Vec<Uuid> {
    match assigned {
        Some(u) => vec![u],
        None => admins.to_vec(),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn notify_many(
    pool: &PgPool,
    users: &[Uuid],
    kind: &str,
    title: &str,
    body: Option<&str>,
    entity_type: &str,
    entity_id: Uuid,
    dedupe_key: Option<&str>,
) -> ServiceResult<usize> {
    let mut created = 0;
    for user_id in users {
        if automation::notify(
            pool,
            &NewNotification {
                user_id: *user_id,
                kind,
                title,
                body,
                entity_type,
                entity_id,
                dedupe_key,
            },
        )
        .await?
        {
            created += 1;
        }
    }
    Ok(created)
}

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    unread_only: bool,
) -> ServiceResult<Vec<NotificationRow>> {
    Ok(automation::list(pool, caller.user_id, unread_only, 200).await?)
}

pub async fn unread_count(pool: &PgPool, caller: &Session) -> ServiceResult<i64> {
    Ok(automation::unread_count(pool, caller.user_id).await?)
}

pub async fn mark_read(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    automation::mark_read(pool, caller.user_id, id).await?;
    Ok(())
}

pub async fn mark_all_read(pool: &PgPool, caller: &Session) -> ServiceResult<u64> {
    Ok(automation::mark_all_read(pool, caller.user_id).await?)
}

/// Direct alert when a case is assigned to someone by a colleague.
pub async fn case_assigned(
    pool: &PgPool,
    assignee: Uuid,
    case_id: Uuid,
    title: &str,
) -> ServiceResult<()> {
    automation::notify(
        pool,
        &NewNotification {
            user_id: assignee,
            kind: "CASE_ASSIGNED",
            title,
            body: None,
            entity_type: "renewal_case",
            entity_id: case_id,
            dedupe_key: None,
        },
    )
    .await?;
    Ok(())
}
