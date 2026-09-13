//! Conversions from service/database rows to the wire types in `renewal_api`,
//! plus small parsers for ids and dates arriving as strings.

use chrono::{DateTime, NaiveDate, Utc};
use renewal_api::*;
use renewal_core::{Capability, Role};
use renewal_db::audit::AuditListRow;
use renewal_db::automation::{NotificationRow, RuleRow};
use renewal_db::buildings::BuildingRow;
use renewal_db::contracts::ContractRow;
use renewal_db::dashboard::{
    BuildingSummary as DbBuildingSummary, DashboardCounts as DbDashboardCounts,
    SearchHit as DbSearchHit,
};
use renewal_db::documents::DocumentRow;
use renewal_db::emails::{MessageRow, NoticeRow, TemplateRow};
use renewal_db::follow_ups::{FollowUpCounts as DbFollowUpCounts, FollowUpRow};
use renewal_db::paging::PageResult;
use renewal_db::renewals::{CaseRow, ChecklistItemRow, ResponseRow, TemplateItemRow};
use renewal_db::tenants::TenantRow;
use renewal_db::units::UnitSummaryRow;
use renewal_services::contracts::months_between;
use renewal_services::dashboard::DashboardData;
use renewal_services::system::SystemInfo;
use renewal_services::users::UserRow;
use renewal_services::{ServiceError, Session};
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::error::ApiFailure;

// ---------------------------------------------------------------- parsing helpers

pub fn uuid(s: &str, what: &str) -> Result<Uuid, ApiFailure> {
    s.trim()
        .parse()
        .map_err(|_| ApiFailure(ServiceError::Validation(format!("{what}: invalid id"))))
}

pub fn uuid_opt(s: &Option<String>, what: &str) -> Result<Option<Uuid>, ApiFailure> {
    match s.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => uuid(v, what).map(Some),
        None => Ok(None),
    }
}

pub fn date(s: &str, what: &str) -> Result<NaiveDate, ApiFailure> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| {
        ApiFailure(ServiceError::Validation(format!(
            "{what}: use the format YYYY-MM-DD"
        )))
    })
}

pub fn date_opt(s: &Option<String>, what: &str) -> Result<Option<NaiveDate>, ApiFailure> {
    match s.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => date(v, what).map(Some),
        None => Ok(None),
    }
}

fn ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339()
}
fn ts_opt(t: Option<DateTime<Utc>>) -> Option<String> {
    t.map(ts)
}
fn d(v: NaiveDate) -> String {
    v.format("%Y-%m-%d").to_string()
}
fn d_opt(v: Option<NaiveDate>) -> Option<String> {
    v.map(d)
}
fn parse_enum<T: std::str::FromStr>(s: &str) -> Option<T> {
    s.parse().ok()
}

pub fn page<T, U>(p: PageResult<T>, page: i64, page_size: i64, f: impl Fn(T) -> U) -> Page<U> {
    Page {
        items: p.items.into_iter().map(f).collect(),
        page,
        page_size,
        total: p.total,
    }
}

// ---------------------------------------------------------------- phase 0

pub fn session_info(s: &Session) -> SessionInfo {
    SessionInfo {
        user_id: s.user_id.to_string(),
        name: s.name.clone(),
        email: s.email.clone(),
        role: s.role,
        capabilities: Capability::iter().filter(|c| s.role.allows(*c)).collect(),
        expires_at: s.expires_at.to_rfc3339(),
    }
}

pub fn user_summary(u: UserRow) -> UserSummary {
    UserSummary {
        id: u.id.to_string(),
        name: u.name,
        email: u.email,
        role: u.role.parse().unwrap_or(Role::Management),
        active: u.active,
        created_at: ts(u.created_at),
        last_login_at: ts_opt(u.last_login_at),
    }
}

pub fn employee(u: UserRow) -> EmployeeOption {
    EmployeeOption {
        id: u.id.to_string(),
        name: u.name,
        role: u.role.parse().unwrap_or(Role::Management),
    }
}

pub fn system_status(info: SystemInfo) -> SystemStatus {
    SystemStatus {
        org_name: info.org_name,
        timezone: info.timezone,
        thresholds: info.thresholds,
        scheduler: info.scheduler.map(|s| SchedulerStatus {
            version: s.version,
            hostname: s.hostname,
            last_heartbeat_at: ts(s.last_heartbeat_at),
            last_sweep_at: ts_opt(s.last_sweep_at),
            stale: s.stale,
        }),
    }
}

// ---------------------------------------------------------------- phase 1

pub fn building(b: BuildingRow) -> Building {
    Building {
        id: b.id.to_string(),
        name: b.name,
        code: b.code,
        location: b.location,
        building_type: b.building_type,
        notes: b.notes,
        total_units: b.total_units,
        occupied_units: b.occupied_units,
        vacant_units: b.vacant_units,
        reserved_units: b.reserved_units,
        maintenance_units: b.maintenance_units,
        created_at: ts(b.created_at),
        updated_at: ts(b.updated_at),
    }
}

pub fn building_summary(s: DbBuildingSummary) -> BuildingSummary {
    BuildingSummary {
        total_units: s.total_units,
        occupied_units: s.occupied_units,
        vacant_units: s.vacant_units,
        reserved_units: s.reserved_units,
        maintenance_units: s.maintenance_units,
        active_contracts: s.active_contracts,
        expiring_soon: s.expiring_soon,
        renewals_pending: s.renewals_pending,
    }
}

pub fn unit(u: UnitSummaryRow) -> UnitSummary {
    UnitSummary {
        id: u.id.to_string(),
        building_id: u.building_id.to_string(),
        building_name: u.building_name,
        building_code: u.building_code,
        unit_number: u.unit_number,
        floor: u.floor,
        unit_type: u.unit_type,
        status: parse_enum(&u.status).unwrap_or(renewal_core::UnitStatus::Vacant),
        occupant_count: i64::from(u.occupant_count),
        notes: u.notes,
        contract_id: u.contract_id.map(|x| x.to_string()),
        contract_number: u.contract_number,
        rent_amount: u.rent_amount_minor.map(money),
        tenant_id: u.tenant_id.map(|x| x.to_string()),
        tenant_name: u.tenant_name,
        start_date: d_opt(u.start_date),
        end_date: d_opt(u.end_date),
        remaining_days: u.remaining_days,
        band: u.band.as_deref().and_then(parse_enum),
        expiring_soon: u.expiring_soon.unwrap_or(false),
        urgent: u.urgent.unwrap_or(false),
        case_id: u.case_id.map(|x| x.to_string()),
        renewal_status: u.renewal_status.as_deref().and_then(parse_enum),
        assigned_employee_id: u.assigned_employee_id.map(|x| x.to_string()),
        assigned_employee_name: u.assigned_employee_name,
        created_at: ts(u.created_at),
        updated_at: ts(u.updated_at),
    }
}

pub fn tenant(t: TenantRow) -> Tenant {
    Tenant {
        id: t.id.to_string(),
        name: t.name,
        contact_person: t.contact_person,
        mobile: t.mobile,
        email: t.email,
        alt_contact: t.alt_contact,
        address: t.address,
        notes: t.notes,
        active_contracts: t.active_contracts,
        current_units: t.current_units,
        created_at: ts(t.created_at),
        updated_at: ts(t.updated_at),
    }
}

pub fn document(doc: DocumentRow) -> DocumentInfo {
    DocumentInfo {
        id: doc.id.to_string(),
        entity_type: DocumentEntity::parse(&doc.entity_type).unwrap_or(DocumentEntity::Building),
        entity_id: doc.entity_id.to_string(),
        file_name: doc.file_name,
        content_type: doc.content_type,
        size_bytes: doc.size_bytes,
        uploaded_by_name: doc.uploaded_by_name,
        created_at: ts(doc.created_at),
    }
}

// ---------------------------------------------------------------- phase 2–4

pub fn contract(c: ContractRow) -> Contract {
    Contract {
        id: c.id.to_string(),
        contract_number: c.contract_number,
        tenant_id: c.tenant_id.to_string(),
        tenant_name: c.tenant_name,
        tenant_contact: c.tenant_contact,
        tenant_email: c.tenant_email,
        building_id: c.building_id.to_string(),
        building_name: c.building_name,
        building_code: c.building_code,
        unit_tenants: c
            .unit_ids
            .iter()
            .zip(c.unit_occupant_counts.iter())
            .map(|(u, n)| ContractUnitTenants {
                unit_id: u.to_string(),
                occupant_count: i64::from(*n),
            })
            .collect(),
        occupant_count: c.occupant_count,
        unit_ids: c.unit_ids.iter().map(|u| u.to_string()).collect(),
        unit_numbers: c.unit_numbers,
        start_date: d(c.start_date),
        end_date: d(c.end_date),
        duration_months: months_between(c.start_date, c.end_date),
        rent_terms: c.rent_terms,
        rent_amount: c.rent_amount_minor.map(money),
        status: parse_enum(&c.status).unwrap_or(renewal_core::ContractStatus::Draft),
        assigned_employee_id: c.assigned_employee_id.map(|x| x.to_string()),
        assigned_employee_name: c.assigned_employee_name,
        previous_contract_id: c.previous_contract_id.map(|x| x.to_string()),
        root_contract_id: c.root_contract_id.map(|x| x.to_string()),
        renewal_sequence: c.renewal_sequence,
        notes: c.notes,
        remaining_days: c.remaining_days,
        band: parse_enum(&c.band).unwrap_or(renewal_core::Band::Beyond120),
        expiring_soon: c.expiring_soon,
        urgent: c.urgent,
        renewal_in_progress: c.renewal_in_progress,
        case_id: c.case_id.map(|x| x.to_string()),
        renewal_status: c.renewal_status.as_deref().and_then(parse_enum),
        notice_status: c.notice_status.as_deref().and_then(parse_enum),
        case_assigned_employee_id: c.case_assigned_employee_id.map(|x| x.to_string()),
        case_assigned_employee_name: c.case_assigned_employee_name,
        activated_at: ts_opt(c.activated_at),
        ended_at: ts_opt(c.ended_at),
        created_at: ts(c.created_at),
        updated_at: ts(c.updated_at),
    }
}

pub fn case(r: CaseRow) -> RenewalCase {
    let progress = if r.checklist_total > 0 {
        ((r.checklist_done * 100) / r.checklist_total) as i32
    } else {
        0
    };
    RenewalCase {
        id: r.id.to_string(),
        contract_id: r.contract_id.to_string(),
        contract_number: r.contract_number,
        contract_status: parse_enum(&r.contract_status)
            .unwrap_or(renewal_core::ContractStatus::Active),
        status: parse_enum(&r.status).unwrap_or(renewal_core::RenewalStatus::NotStarted),
        notice_status: parse_enum(&r.notice_status).unwrap_or(renewal_core::NoticeStatus::Pending),
        assigned_employee_id: r.assigned_employee_id.map(|x| x.to_string()),
        assigned_employee_name: r.assigned_employee_name,
        is_urgent: r.is_urgent,
        latest_response: r.latest_response.as_deref().and_then(parse_enum),
        latest_response_at: ts_opt(r.latest_response_at),
        next_follow_up_date: d_opt(r.next_follow_up_date),
        outcome_contract_id: r.outcome_contract_id.map(|x| x.to_string()),
        outcome_contract_number: r.outcome_contract_number,
        notes: r.notes,
        tenant_id: r.tenant_id.to_string(),
        tenant_name: r.tenant_name,
        tenant_contact: r.tenant_contact,
        tenant_email: r.tenant_email,
        tenant_mobile: r.tenant_mobile,
        building_id: r.building_id.to_string(),
        building_name: r.building_name,
        unit_numbers: r.unit_numbers,
        start_date: d(r.start_date),
        end_date: d(r.end_date),
        remaining_days: r.remaining_days,
        band: parse_enum(&r.band).unwrap_or(renewal_core::Band::Beyond120),
        checklist_total: r.checklist_total,
        checklist_done: r.checklist_done,
        progress_percent: progress,
        open_follow_ups: r.open_follow_ups,
        notice_sent_at: ts_opt(r.notice_sent_at),
        notice_recipient: r.notice_recipient,
        opened_at: ts(r.opened_at),
        closed_at: ts_opt(r.closed_at),
        updated_at: ts(r.updated_at),
    }
}

pub fn response(r: ResponseRow) -> TenantResponseRecord {
    TenantResponseRecord {
        id: r.id.to_string(),
        response: parse_enum(&r.response).unwrap_or(renewal_core::TenantResponse::NoResponse),
        response_date: d(r.response_date),
        notes: r.notes,
        follow_up_date: d_opt(r.follow_up_date),
        recorded_by_name: r.recorded_by_name,
        created_at: ts(r.created_at),
    }
}

pub fn checklist_item(i: ChecklistItemRow) -> ChecklistItem {
    ChecklistItem {
        id: i.id.to_string(),
        key: i.key,
        label: i.label,
        required: i.required,
        done: i.done,
        done_by_name: i.done_by_name,
        done_at: ts_opt(i.done_at),
    }
}

pub fn template_item(i: TemplateItemRow) -> ChecklistTemplateItem {
    ChecklistTemplateItem {
        id: Some(i.id.to_string()),
        key: i.key,
        label: i.label,
        required: i.required,
        active: i.active,
    }
}

pub fn follow_up(f: FollowUpRow) -> FollowUp {
    FollowUp {
        id: f.id.to_string(),
        case_id: f.case_id.to_string(),
        contract_id: f.contract_id.to_string(),
        contract_number: f.contract_number,
        tenant_id: f.tenant_id.to_string(),
        tenant_name: f.tenant_name,
        building_name: f.building_name,
        unit_numbers: f.unit_numbers,
        due_date: d(f.due_date),
        days_until_due: f.days_until_due,
        follow_up_type: parse_enum(&f.follow_up_type)
            .unwrap_or(renewal_core::FollowUpType::PhoneCall),
        assigned_employee_id: f.assigned_employee_id.map(|x| x.to_string()),
        assigned_employee_name: f.assigned_employee_name,
        notes: f.notes,
        status: parse_enum(&f.status).unwrap_or(renewal_core::FollowUpStatus::Open),
        completed_at: ts_opt(f.completed_at),
        created_at: ts(f.created_at),
    }
}

pub fn follow_up_counts(c: DbFollowUpCounts) -> FollowUpCounts {
    FollowUpCounts {
        today: c.today,
        overdue: c.overdue,
        upcoming: c.upcoming,
    }
}

pub fn dashboard(dd: DashboardData) -> Dashboard {
    let c: DbDashboardCounts = dd.counts;
    Dashboard {
        counts: DashboardCounts {
            total_buildings: c.total_buildings,
            total_units: c.total_units,
            occupied_units: c.occupied_units,
            vacant_units: c.vacant_units,
            active_tenants: c.active_tenants,
            active_contracts: c.active_contracts,
            expiring_soon: c.expiring_soon,
            renewals_pending: c.renewals_pending,
            renewals_completed: c.renewals_completed,
            expired_contracts: c.expired_contracts,
        },
        bands: dd
            .bands
            .into_iter()
            .filter_map(|b| {
                parse_enum(&b.band).map(|band| BandCount {
                    band,
                    count: b.count,
                })
            })
            .collect(),
        urgent_renewals: dd.urgent_renewals.into_iter().map(contract).collect(),
        upcoming_expiries: dd.upcoming_expiries.into_iter().map(contract).collect(),
        pending_tenant_responses: dd
            .pending_tenant_responses
            .into_iter()
            .map(contract)
            .collect(),
        notices_pending: dd.notices_pending.into_iter().map(contract).collect(),
        recently_completed: dd.recently_completed.into_iter().map(case).collect(),
        follow_ups: follow_up_counts(dd.follow_ups),
        completed_window_days: dd.completed_window_days,
    }
}

pub fn search_hit(h: DbSearchHit) -> Option<SearchHit> {
    let kind = match h.kind.as_str() {
        "building" => SearchKind::Building,
        "unit" => SearchKind::Unit,
        "tenant" => SearchKind::Tenant,
        "contract" => SearchKind::Contract,
        "expense" => SearchKind::Expense,
        _ => return None,
    };
    Some(SearchHit {
        kind,
        id: h.id.to_string(),
        title: h.title,
        subtitle: h.subtitle,
    })
}

// ---------------------------------------------------------------- phase 5

pub fn email_template(t: TemplateRow) -> EmailTemplate {
    EmailTemplate {
        id: t.id.to_string(),
        key: t.key,
        name: t.name,
        subject: t.subject,
        body_text: t.body_text,
        active: t.active,
        updated_at: ts(t.updated_at),
    }
}

pub fn email_message(m: MessageRow) -> EmailMessage {
    EmailMessage {
        id: m.id.to_string(),
        email_type: m.email_type,
        tenant_id: m.tenant_id.map(|x| x.to_string()),
        tenant_name: m.tenant_name,
        contract_id: m.contract_id.map(|x| x.to_string()),
        contract_number: m.contract_number,
        case_id: m.case_id.map(|x| x.to_string()),
        to: m.to_addresses,
        cc: m.cc_addresses,
        subject: m.subject,
        body_text: m.body_text,
        status: parse_enum(&m.status).unwrap_or(renewal_core::EmailStatus::Queued),
        attempts: m.attempts,
        last_error: m.last_error,
        provider: m.provider,
        sent_by_name: m.sent_by_name,
        attachment_count: m.attachment_count,
        queued_at: ts(m.queued_at),
        sent_at: ts_opt(m.sent_at),
    }
}

pub fn notice(n: NoticeRow) -> Notice {
    Notice {
        id: n.id.to_string(),
        case_id: n.case_id.to_string(),
        status: parse_enum(&n.status).unwrap_or(renewal_core::NoticeStatus::Draft),
        proposed_period: n.proposed_period,
        other_terms: n.other_terms,
        subject: n.subject,
        body_text: n.body_text,
        recipient: n.recipient,
        cc: n.cc_addresses,
        pdf_document_id: n.pdf_document_id.map(|x| x.to_string()),
        email_message_id: n.email_message_id.map(|x| x.to_string()),
        email_status: n.email_status.as_deref().and_then(parse_enum),
        sent_by_name: n.sent_by_name,
        sent_at: ts_opt(n.sent_at),
        created_at: ts(n.created_at),
        updated_at: ts(n.updated_at),
    }
}

// ---------------------------------------------------------------- phase 6

pub fn notification(n: NotificationRow) -> Notification {
    Notification {
        id: n.id.to_string(),
        kind: n.kind,
        title: n.title,
        body: n.body,
        entity_type: n.entity_type,
        entity_id: n.entity_id.to_string(),
        created_at: ts(n.created_at),
        read_at: ts_opt(n.read_at),
    }
}

pub fn reminder_rule(r: RuleRow) -> ReminderRuleDto {
    ReminderRuleDto {
        id: Some(r.id.to_string()),
        days_before: r.days_before,
        label: r.label,
        notify_in_app: r.notify_in_app,
        email_assigned_employee: r.email_assigned_employee,
        mark_urgent: r.mark_urgent,
        active: r.active,
    }
}

pub fn sweep_summary(s: &renewal_services::sweep::SweepSummary) -> SweepSummaryDto {
    SweepSummaryDto {
        contracts_checked: s.contracts_checked as i64,
        reminders_fired: s.reminders_fired as i64,
        reminders_skipped: s.reminders_skipped as i64,
        cases_opened: s.cases_opened as i64,
        contracts_expired: s.contracts_expired as i64,
        notifications_created: s.notifications_created as i64,
        emails_queued: s.emails_queued as i64,
        follow_up_alerts: s.follow_up_alerts as i64,
        notice_alerts: s.notice_alerts as i64,
        response_alerts: s.response_alerts as i64,
    }
}

// ---------------------------------------------------------------- phase 7

pub fn audit_entry(a: AuditListRow) -> AuditEntry {
    AuditEntry {
        id: a.id,
        actor_id: a.actor_id.map(|x| x.to_string()),
        actor_name: a.actor_name,
        entity_type: a.entity_type,
        entity_id: a.entity_id.map(|x| x.to_string()),
        action: a.action,
        before: a.before_json,
        after: a.after_json,
        created_at: ts(a.created_at),
    }
}

// ---------------------------------------------------------------- occupants & expenses

fn money(minor: i64) -> f64 {
    minor as f64 / 100.0
}

/// Major units from the wire (`1234.56`) to minor units, rounded to the fils.
pub fn minor(amount: f64, what: &str) -> Result<i64, ApiFailure> {
    if !amount.is_finite() || !(0.0..=1.0e12).contains(&amount) {
        return Err(ApiFailure(ServiceError::validation(format!(
            "{what} is not a valid amount"
        ))));
    }
    Ok((amount * 100.0).round() as i64)
}

pub fn expense(e: renewal_db::expenses::ExpenseRow) -> Expense {
    Expense {
        id: e.id.to_string(),
        unit_id: e.unit_id.to_string(),
        unit_number: e.unit_number,
        building_id: e.building_id.to_string(),
        building_name: e.building_name,
        category: e.category,
        description: e.description,
        amount: money(e.amount_minor),
        expense_date: d(e.expense_date),
        period_start: e.period_start.map(d),
        period_end: e.period_end.map(d),
        vendor: e.vendor,
        reference: e.reference,
        split_method: e.split_method,
        notes: e.notes,
        split_count: i64::from(e.split_count),
        settled_count: i64::from(e.settled_count),
        created_by_name: e.created_by_name,
        created_at: ts(e.created_at),
        updated_at: ts(e.updated_at),
    }
}

pub fn expense_input(i: &ExpenseInput) -> Result<renewal_db::expenses::ExpenseInput, ApiFailure> {
    Ok(renewal_db::expenses::ExpenseInput {
        unit_id: uuid(&i.unit_id, "unit")?,
        category: i.category.clone(),
        description: i.description.clone(),
        amount_minor: minor(i.amount, "amount")?,
        expense_date: date(&i.expense_date, "expense date")?,
        period_start: date_opt(&i.period_start, "period start")?,
        period_end: date_opt(&i.period_end, "period end")?,
        vendor: i.vendor.clone(),
        reference: i.reference.clone(),
        split_method: i.split_method.clone(),
        split_count: count(i.split_count.unwrap_or(0), "number of people")?,
        notes: i.notes.clone(),
    })
}

/// A small non-negative count from the wire (people in a unit, shares paid).
pub fn count(n: i64, what: &str) -> Result<i32, ApiFailure> {
    i32::try_from(n)
        .ok()
        .filter(|n| *n >= 0)
        .ok_or_else(|| ApiFailure(ServiceError::validation(format!("invalid {what}"))))
}

pub fn expense_detail(dtl: renewal_services::expenses::ExpenseDetail) -> ExpenseDetail {
    ExpenseDetail {
        expense: expense(dtl.expense),
        shares: dtl
            .shares
            .into_iter()
            .map(|s| ExpenseShare {
                index: i64::from(s.index),
                amount: money(s.amount_minor),
                settled: s.settled,
            })
            .collect(),
    }
}

pub fn expense_summary(s: renewal_services::expenses::Summary) -> ExpenseSummary {
    ExpenseSummary {
        from: d(s.scope.from),
        to: d(s.scope.to),
        total: money(s.total),
        expense_count: s.expense_count,
        this_month: money(s.this_month),
        last_month: money(s.last_month),
        outstanding: money(s.outstanding),
        outstanding_shares: s.outstanding_shares,
        monthly: s
            .monthly
            .into_iter()
            .map(|m| MonthPoint {
                month: d(m.month),
                amount: money(m.amount_minor),
                expense_count: m.expense_count,
            })
            .collect(),
        by_building: s.by_building.into_iter().map(group_point).collect(),
        by_unit: s.by_unit.into_iter().map(group_point).collect(),
        by_category: s
            .by_category
            .into_iter()
            .map(|c| CategoryPoint {
                category: c.category,
                amount: money(c.amount_minor),
                expense_count: c.expense_count,
            })
            .collect(),
    }
}

fn group_point(g: renewal_db::expenses::GroupTotal) -> GroupPoint {
    GroupPoint {
        id: g.id.to_string(),
        label: g.label,
        sublabel: g.sublabel,
        amount: money(g.amount_minor),
        expense_count: g.expense_count,
    }
}
