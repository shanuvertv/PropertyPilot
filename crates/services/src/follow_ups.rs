//! Follow-up tasks (spec §12).

use chrono::NaiveDate;
use renewal_core::{Capability, FollowUpStatus, FollowUpType};
use renewal_db::follow_ups::{self, FollowUpCounts, FollowUpFilter, FollowUpRow, NewFollowUp};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::{renewals, PgPool};
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &FollowUpFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<FollowUpRow>> {
    caller.require(Capability::ViewFollowUps)?;
    Ok(follow_ups::list(pool, f, q).await?)
}

pub async fn for_case(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
) -> ServiceResult<Vec<FollowUpRow>> {
    caller.require(Capability::ViewFollowUps)?;
    Ok(follow_ups::for_case(pool, case_id).await?)
}

pub async fn counts(pool: &PgPool, caller: &Session, mine: bool) -> ServiceResult<FollowUpCounts> {
    caller.require(Capability::ViewFollowUps)?;
    Ok(follow_ups::counts(pool, mine.then_some(caller.user_id)).await?)
}

pub struct FollowUpInput {
    pub due_date: NaiveDate,
    pub follow_up_type: FollowUpType,
    pub assigned_employee_id: Option<Uuid>,
    pub notes: Option<String>,
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    case_id: Uuid,
    mut input: FollowUpInput,
) -> ServiceResult<FollowUpRow> {
    caller.require(Capability::ManageFollowUps)?;
    trim_opt(&mut input.notes);
    let mut tx = pool.begin().await?;
    let case = renewals::find(&mut *tx, case_id)
        .await?
        .ok_or(ServiceError::NotFound("renewal case"))?;
    let id = follow_ups::insert(
        &mut *tx,
        &NewFollowUp {
            case_id,
            due_date: input.due_date,
            follow_up_type: &input.follow_up_type.to_string(),
            assigned_employee_id: input
                .assigned_employee_id
                .or(case.assigned_employee_id)
                .or(Some(caller.user_id)),
            notes: input.notes.as_deref(),
            created_by: caller.user_id,
        },
    )
    .await?;
    renewals::set_next_follow_up(&mut *tx, case_id).await?;
    let row = follow_ups::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("follow-up"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        case_id,
        "FOLLOW_UP_CREATED",
        NONE,
        Some(&row),
    )
    .await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: FollowUpInput,
) -> ServiceResult<FollowUpRow> {
    caller.require(Capability::ManageFollowUps)?;
    trim_opt(&mut input.notes);
    let mut tx = pool.begin().await?;
    let before = follow_ups::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("follow-up"))?;
    if before.status != FollowUpStatus::Open.to_string() {
        return Err(ServiceError::Conflict(
            "only open follow-ups can be edited".into(),
        ));
    }
    follow_ups::update(
        &mut *tx,
        id,
        input.due_date,
        &input.follow_up_type.to_string(),
        input.assigned_employee_id.or(before.assigned_employee_id),
        input.notes.as_deref(),
    )
    .await?;
    renewals::set_next_follow_up(&mut *tx, before.case_id).await?;
    let after = follow_ups::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("follow-up"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        before.case_id,
        "FOLLOW_UP_UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn set_status(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    status: FollowUpStatus,
) -> ServiceResult<FollowUpRow> {
    caller.require(Capability::ManageFollowUps)?;
    let mut tx = pool.begin().await?;
    let before = follow_ups::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("follow-up"))?;
    follow_ups::set_status(&mut *tx, id, &status.to_string(), caller.user_id).await?;
    renewals::set_next_follow_up(&mut *tx, before.case_id).await?;
    let after = follow_ups::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("follow-up"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "renewal_case",
        before.case_id,
        "FOLLOW_UP_STATUS",
        Some(&before.status),
        Some(&after.status),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}
