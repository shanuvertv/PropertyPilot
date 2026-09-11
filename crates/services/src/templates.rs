//! Email/notice templates with `{{Placeholder}}` fields (spec §10).

use chrono::NaiveDate;
use handlebars::Handlebars;
use renewal_core::Capability;
use renewal_db::emails::{self, TemplateRow};
use renewal_db::renewals::CaseRow;
use renewal_db::{contracts, settings, PgPool};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit_log;
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

/// Every placeholder a template may use, with a short description for the editor.
pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("TenantName", "Tenant / company name"),
    (
        "ContactPerson",
        "Tenant contact person (falls back to the tenant name)",
    ),
    ("BuildingName", "Building name"),
    ("UnitNumber", "Unit number(s) on the contract"),
    ("ContractNumber", "Contract number"),
    ("ContractStartDate", "Contract start date"),
    ("ContractEndDate", "Contract end date"),
    ("RemainingDays", "Days until the contract ends"),
    (
        "ProposedRenewalPeriod",
        "Proposed renewal period (from the notice)",
    ),
    ("OtherRenewalTerms", "Other renewal terms (from the notice)"),
    ("ResponsibleEmployee", "Assigned leasing employee"),
    ("RenewalStatus", "Current renewal status"),
    ("NoticeStatus", "Current notice status"),
    ("CompanyName", "Your organisation name (Settings)"),
    ("Today", "Today's date"),
];

pub async fn list(pool: &PgPool, caller: &Session) -> ServiceResult<Vec<TemplateRow>> {
    caller.require(Capability::ViewRenewals)?;
    Ok(emails::templates(pool).await?)
}

pub async fn get(pool: &PgPool, key: &str) -> ServiceResult<TemplateRow> {
    emails::template(pool, key)
        .await?
        .ok_or(ServiceError::NotFound("email template"))
}

pub async fn update(
    pool: &PgPool,
    caller: &Session,
    key: &str,
    name: &str,
    subject: &str,
    body_text: &str,
    active: bool,
) -> ServiceResult<TemplateRow> {
    caller.require(Capability::ManageSettings)?;
    let name = name.trim();
    let subject = subject.trim();
    let body_text = body_text.trim();
    if name.is_empty() || subject.is_empty() || body_text.is_empty() {
        return Err(ServiceError::validation(
            "name, subject and body are required",
        ));
    }
    // Reject unknown placeholders up front so a typo never reaches a tenant.
    let sample = sample_context();
    render(subject, &sample)?;
    render(body_text, &sample)?;
    let mut tx = pool.begin().await?;
    let before = emails::template(&mut *tx, key)
        .await?
        .ok_or(ServiceError::NotFound("email template"))?;
    emails::update_template(
        &mut *tx,
        key,
        name,
        subject,
        body_text,
        active,
        caller.user_id,
    )
    .await?;
    let after = emails::template(&mut *tx, key)
        .await?
        .ok_or(ServiceError::NotFound("email template"))?;
    audit_log::log(
        &mut *tx,
        caller,
        "email_template",
        before.id,
        "UPDATED",
        Some(&before),
        Some(&after),
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

fn engine() -> Handlebars<'static> {
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(true);
    hbs.register_escape_fn(handlebars::no_escape);
    hbs
}

/// Renders a template body/subject against a placeholder context. Unknown placeholders are an error.
pub fn render(template: &str, ctx: &Value) -> ServiceResult<String> {
    engine().render_template(template, ctx).map_err(|e| {
        // Strict mode reports the missing variable in the error reason; surface it
        // as the placeholder the user typed, e.g. {{TenantNmae}}.
        let msg = e.to_string();
        let missing = find_unknown_placeholder(template, ctx);
        match missing {
            Some(name) => ServiceError::validation(format!("unknown placeholder {{{{{name}}}}}")),
            None => ServiceError::validation(format!("template error: {msg}")),
        }
    })
}

/// First `{{Name}}` in the template that the context does not provide.
fn find_unknown_placeholder(template: &str, ctx: &Value) -> Option<String> {
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after.find("}}")?;
        let name = after[..end]
            .trim()
            .trim_start_matches('#')
            .trim_start_matches('/');
        let known = name.is_empty()
            || name.starts_with('!')
            || ctx.get(name).is_some()
            || matches!(name, "else" | "this");
        if !known {
            return Some(name.to_owned());
        }
        rest = &after[end + 2..];
    }
    None
}

fn fmt_date(d: NaiveDate) -> String {
    d.format("%d %B %Y").to_string()
}

pub fn sample_context() -> Value {
    let mut v = json!({});
    for (k, _) in PLACEHOLDERS {
        v[*k] = Value::String(format!("<{k}>"));
    }
    v
}

/// Builds the placeholder context for a renewal case (spec §7 step 3 "automatically populate").
pub async fn context_for_case(
    pool: &PgPool,
    case: &CaseRow,
    proposed_period: Option<&str>,
    other_terms: Option<&str>,
) -> ServiceResult<Value> {
    let contract = contracts::find(pool, case.contract_id)
        .await?
        .ok_or(ServiceError::NotFound("contract"))?;
    let company: String = settings::get(pool, settings::ORG_NAME)
        .await?
        .unwrap_or_default();
    let today = renewal_db::today(pool).await?;
    let employee = case
        .assigned_employee_name
        .clone()
        .or(contract.assigned_employee_name.clone())
        .unwrap_or_else(|| "Leasing Team".into());
    Ok(json!({
        "TenantName": case.tenant_name,
        "ContactPerson": case.tenant_contact.clone().unwrap_or_else(|| case.tenant_name.clone()),
        "BuildingName": case.building_name,
        "UnitNumber": case.unit_numbers,
        "ContractNumber": case.contract_number,
        "ContractStartDate": fmt_date(case.start_date),
        "ContractEndDate": fmt_date(case.end_date),
        "RemainingDays": case.remaining_days.to_string(),
        "ProposedRenewalPeriod": proposed_period.unwrap_or("a further 12 months on the same terms"),
        "OtherRenewalTerms": other_terms.unwrap_or(""),
        "ResponsibleEmployee": employee,
        "RenewalStatus": case.status,
        "NoticeStatus": case.notice_status,
        "CompanyName": company,
        "Today": fmt_date(today),
    }))
}

/// Plain text (paragraphs separated by blank lines) → minimal, escaped HTML email body.
pub fn text_to_html(text: &str) -> String {
    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    let paragraphs: Vec<String> = text
        .replace("\r\n", "\n")
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{}</p>", esc(p).replace('\n', "<br>")))
        .collect();
    format!(
        "<!doctype html><html><body style=\"font-family:Segoe UI,Arial,sans-serif;font-size:14px;line-height:1.5;color:#1b2426\">{}</body></html>",
        paragraphs.join("\n")
    )
}

pub struct Rendered {
    pub subject: String,
    pub body_text: String,
}

pub async fn render_for_case(
    pool: &PgPool,
    key: &str,
    case: &CaseRow,
    proposed: Option<&str>,
    terms: Option<&str>,
) -> ServiceResult<Rendered> {
    let tpl = get(pool, key).await?;
    let ctx = context_for_case(pool, case, proposed, terms).await?;
    Ok(Rendered {
        subject: render(&tpl.subject, &ctx)?,
        body_text: render(&tpl.body_text, &ctx)?,
    })
}

pub fn placeholder_help() -> Vec<(String, String)> {
    PLACEHOLDERS
        .iter()
        .map(|(k, d)| (k.to_string(), d.to_string()))
        .collect()
}

/// Lightweight id parser shared by callers that receive template keys with ids.
pub fn parse_uuid(s: &str) -> ServiceResult<Uuid> {
    s.parse()
        .map_err(|_| ServiceError::validation("invalid id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_placeholders_and_rejects_unknown_ones() {
        let ctx = json!({ "TenantName": "Falcon <LLC>", "RemainingDays": "45" });
        assert_eq!(
            render("Hi {{TenantName}}, {{RemainingDays}} days", &ctx).unwrap(),
            "Hi Falcon <LLC>, 45 days"
        );
        let err = render("Hi {{Nope}}", &ctx).unwrap_err();
        assert!(err.to_string().contains("Nope"), "{err}");
    }

    #[test]
    fn html_body_escapes_and_paragraphs() {
        let html = text_to_html("Dear <A>,\nline two\n\nSecond paragraph");
        assert!(html.contains("<p>Dear &lt;A&gt;,<br>line two</p>"));
        assert!(html.contains("<p>Second paragraph</p>"));
    }

    #[test]
    fn sample_context_covers_every_placeholder() {
        let ctx = sample_context();
        for (k, _) in PLACEHOLDERS {
            assert!(render(&format!("{{{{{k}}}}}"), &ctx).is_ok(), "{k}");
        }
    }
}
