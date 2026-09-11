//! Audit trail reads (spec §19). Writes happen inside every other service.

use renewal_core::Capability;
use renewal_db::audit::{self, AuditFilter, AuditListRow};
use renewal_db::paging::{ListQuery, PageResult};
use renewal_db::PgPool;
use uuid::Uuid;

use crate::error::ServiceResult;
use crate::session::Session;

/// Admin and Management see everything; Leasing sees the entries they authored.
pub async fn list(
    pool: &PgPool,
    caller: &Session,
    mut f: AuditFilter,
    q: &ListQuery,
) -> ServiceResult<PageResult<AuditListRow>> {
    if !caller.role.allows(Capability::ViewAuditTrail) {
        caller.require(Capability::ManageContracts)?;
        f.actor_id = Some(caller.user_id);
    }
    Ok(audit::list(pool, &f, q).await?)
}

/// History tab on a record: anyone who may view the record may see its trail.
pub async fn for_entity(
    pool: &PgPool,
    caller: &Session,
    entity_type: &str,
    entity_id: Uuid,
) -> ServiceResult<Vec<AuditListRow>> {
    let cap = match entity_type {
        "building" => Capability::ViewBuildings,
        "unit" => Capability::ViewUnits,
        "tenant" => Capability::ViewTenants,
        "contract" => Capability::ViewContracts,
        "renewal_case" => Capability::ViewRenewals,
        _ => Capability::ViewAuditTrail,
    };
    caller.require(cap)?;
    let q = ListQuery::new(None, Some(1), Some(200), None, None);
    Ok(audit::list(
        pool,
        &AuditFilter {
            entity_type: Some(entity_type.to_owned()),
            entity_id: Some(entity_id),
            ..Default::default()
        },
        &q,
    )
    .await?
    .items)
}
