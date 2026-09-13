//! Tenant master (spec §4).

use renewal_core::Capability;
use renewal_db::contracts::{self, ContractRow};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::renewals::{self, CaseRow};
use renewal_db::tenants::{self, TenantFilter, TenantInput, TenantRow};
use renewal_db::PgPool;
use uuid::Uuid;

use crate::audit_log::{self, NONE};
use crate::buildings::trim_opt;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

pub async fn list(
    pool: &PgPool,
    caller: &Session,
    f: &TenantFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<TenantRow>> {
    caller.require(Capability::ViewTenants)?;
    Ok(tenants::list(pool, f, q).await?)
}

pub async fn options(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<TenantRow>> {
    caller.require(Capability::ViewTenants)?;
    Ok(tenants::all_active(pool).await?)
}

pub async fn get(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<TenantRow> {
    caller.require(Capability::ViewTenants)?;
    tenants::find(pool, id)
        .await?
        .filter(|t| t.archived_at.is_none())
        .ok_or(ServiceError::NotFound("tenant"))
}

/// Spec §4 profile: current + historical contracts and every renewal case.
pub struct TenantHistory {
    pub contracts: Vec<ContractRow>,
    pub cases: Vec<CaseRow>,
}

pub async fn history(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<TenantHistory> {
    caller.require(Capability::ViewTenants)?;
    let contracts = if caller.role.allows(Capability::ViewContracts) {
        contracts::for_tenant(pool, id).await?
    } else {
        vec![]
    };
    let cases = if caller.role.allows(Capability::ViewRenewals) {
        renewals::for_tenant(pool, id).await?
    } else {
        vec![]
    };
    Ok(TenantHistory { contracts, cases })
}

fn validate(input: &mut TenantInput) -> ServiceResult<()> {
    input.name = input.name.trim().to_owned();
    for f in [
        &mut input.contact_person,
        &mut input.mobile,
        &mut input.email,
        &mut input.alt_contact,
        &mut input.address,
        &mut input.notes,
    ] {
        trim_opt(f);
    }
    if input.name.is_empty() {
        return Err(ServiceError::validation(
            "tenant / company name is required",
        ));
    }
    if let Some(e) = &input.email {
        if !e.contains('@') {
            return Err(ServiceError::validation("email address looks invalid"));
        }
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    caller: &Session,
    mut input: TenantInput,
) -> ServiceResult<TenantRow> {
    caller.require(Capability::ManageTenants)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let id = tenants::insert(&mut *tx, &input, caller.user_id).await?;
    let row = tenants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("tenant"))?;
    audit_log::log(&mut *tx, caller, "tenant", id, "CREATED", NONE, Some(&row)).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    id: Uuid,
    mut input: TenantInput,
) -> ServiceResult<TenantRow> {
    caller.require(Capability::ManageTenants)?;
    validate(&mut input)?;
    let mut tx = pool.begin().await?;
    let before = tenants::find(&mut *tx, id)
        .await?
        .filter(|t| t.archived_at.is_none())
        .ok_or(ServiceError::NotFound("tenant"))?;
    tenants::update(&mut *tx, id, &input).await?;
    let after = tenants::find(&mut *tx, id)
        .await?
        .ok_or(ServiceError::NotFound("tenant"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "tenant",
        id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

pub async fn archive(pool: &PgPool, caller: &Session, id: Uuid) -> ServiceResult<()> {
    caller.require(Capability::ManageTenants)?;
    let mut tx = pool.begin().await?;
    let before = tenants::find(&mut *tx, id)
        .await?
        .filter(|t| t.archived_at.is_none())
        .ok_or(ServiceError::NotFound("tenant"))?;
    if before.active_contracts > 0 {
        return Err(ServiceError::Conflict(format!(
            "this tenant has {} active contract(s); end them first",
            before.active_contracts
        )));
    }
    tenants::archive(&mut *tx, id).await?;
    audit_log::log(
        &mut *tx,
        caller,
        "tenant",
        id,
        "ARCHIVED",
        Some(&before),
        NONE,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
