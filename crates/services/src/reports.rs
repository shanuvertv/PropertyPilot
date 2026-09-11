//! Simple reports (spec §17): one tabular shape feeds the screen, Excel and PDF.

use chrono::NaiveDate;
use renewal_core::{Capability, RenewalStatus, ReportBucket};
use renewal_db::contracts::{self, ContractFilter};
use renewal_db::paging::ListQuery;
use renewal_db::renewals::{self, CaseFilter};
use renewal_db::units::{self, UnitFilter};
use renewal_db::{settings, PgPool};
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};
use serde::{Deserialize, Serialize};
use typst::foundations::{Array, Dict, Str, Value};
use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::TypstEngine;
use uuid::Uuid;

use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportKind {
    ContractExpiry,
    RenewalStatus,
    UnitSummary,
    TenantRenewal,
    NoticeTracking,
}

impl ReportKind {
    pub fn parse(s: &str) -> Option<ReportKind> {
        match s {
            "contract-expiry" => Some(ReportKind::ContractExpiry),
            "renewal-status" => Some(ReportKind::RenewalStatus),
            "unit-summary" => Some(ReportKind::UnitSummary),
            "tenant-renewal" => Some(ReportKind::TenantRenewal),
            "notice-tracking" => Some(ReportKind::NoticeTracking),
            _ => None,
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            ReportKind::ContractExpiry => "Contract Expiry Report",
            ReportKind::RenewalStatus => "Renewal Status Report",
            ReportKind::UnitSummary => "Unit-Wise Summary Report",
            ReportKind::TenantRenewal => "Tenant Renewal Report",
            ReportKind::NoticeTracking => "Notice Tracking Report",
        }
    }
    pub fn slug(self) -> &'static str {
        match self {
            ReportKind::ContractExpiry => "contract-expiry",
            ReportKind::RenewalStatus => "renewal-status",
            ReportKind::UnitSummary => "unit-summary",
            ReportKind::TenantRenewal => "tenant-renewal",
            ReportKind::NoticeTracking => "notice-tracking",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReportFilter {
    pub building_id: Option<Uuid>,
    pub employee_id: Option<Uuid>,
    /// Contracts ending on/before this date (expiry report) or cases opened after it (others).
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub heading: String,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTable {
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub generated_at: String,
    pub columns: Vec<String>,
    pub sections: Vec<Section>,
    pub total_rows: usize,
}

fn d(v: NaiveDate) -> String {
    v.format("%d %b %Y").to_string()
}
fn d_opt(v: Option<NaiveDate>) -> String {
    v.map(d).unwrap_or_else(|| "—".into())
}
fn label(s: &str) -> String {
    let mut out = String::new();
    for (i, w) in s.split('_').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut chars = w.chars();
        if let Some(f) = chars.next() {
            out.push(f);
            out.push_str(&chars.as_str().to_lowercase());
        }
    }
    out
}

pub async fn build(
    pool: &PgPool,
    caller: &Session,
    kind: ReportKind,
    f: &ReportFilter,
) -> ServiceResult<ReportTable> {
    caller.require(Capability::ViewReports)?;
    let org: String = settings::get(pool, settings::ORG_NAME)
        .await?
        .unwrap_or_default();
    let today = renewal_db::today(pool).await?;
    let mut subtitle = vec![org, d(today)];
    if f.building_id.is_some() {
        subtitle.push("filtered by building".into());
    }

    let (columns, sections): (Vec<String>, Vec<Section>) = match kind {
        ReportKind::ContractExpiry => {
            let page = contracts::list(
                pool,
                &ContractFilter {
                    status: Some(vec!["ACTIVE".into(), "EXPIRED".into()]),
                    building_id: f.building_id,
                    ..Default::default()
                },
                &ListQuery::all(Some("end_date"), None),
            )
            .await?;
            let order = [
                "EXPIRED",
                "DAYS_0_TO_30",
                "DAYS_31_TO_60",
                "DAYS_61_TO_90",
                "DAYS_91_TO_120",
                "BEYOND_120",
            ];
            let headings = [
                "Expired",
                "Expiring in 0–30 days",
                "Expiring in 31–60 days",
                "Expiring in 61–90 days",
                "Expiring in 91–120 days",
                "More than 120 days",
            ];
            let mut sections = Vec::new();
            for (band, heading) in order.iter().zip(headings) {
                let rows: Vec<Vec<String>> = page
                    .items
                    .iter()
                    .filter(|c| c.band == *band)
                    .filter(|c| {
                        f.from.is_none_or(|from| c.end_date >= from)
                            && f.to.is_none_or(|to| c.end_date <= to)
                    })
                    .map(|c| {
                        vec![
                            c.contract_number.clone(),
                            c.tenant_name.clone(),
                            c.building_name.clone(),
                            c.unit_numbers.clone(),
                            d(c.start_date),
                            d(c.end_date),
                            c.remaining_days.to_string(),
                            label(&c.status),
                            c.renewal_status
                                .as_deref()
                                .map(label)
                                .unwrap_or_else(|| "Not started".into()),
                            c.case_assigned_employee_name
                                .clone()
                                .or(c.assigned_employee_name.clone())
                                .unwrap_or_else(|| "—".into()),
                        ]
                    })
                    .collect();
                if !rows.is_empty() {
                    sections.push(Section {
                        heading: heading.to_string(),
                        rows,
                    });
                }
            }
            (
                [
                    "Contract",
                    "Tenant",
                    "Building",
                    "Units",
                    "Start",
                    "End",
                    "Days left",
                    "Status",
                    "Renewal",
                    "Assigned",
                ]
                .map(String::from)
                .to_vec(),
                sections,
            )
        }
        ReportKind::RenewalStatus => {
            let page = renewals::list(
                pool,
                &CaseFilter {
                    building_id: f.building_id,
                    assigned_employee_id: f.employee_id,
                    ..Default::default()
                },
                &ListQuery::all(Some("opened_at"), Some("desc")),
            )
            .await?;
            let mut buckets: Vec<(ReportBucket, Vec<Vec<String>>)> = vec![
                (ReportBucket::Pending, vec![]),
                (ReportBucket::InProgress, vec![]),
                (ReportBucket::Completed, vec![]),
                (ReportBucket::NotRenewing, vec![]),
            ];
            for c in page
                .items
                .iter()
                .filter(|c| f.from.is_none_or(|from| c.opened_at.date_naive() >= from))
            {
                let st: RenewalStatus = c.status.parse().unwrap_or(RenewalStatus::NotStarted);
                let b = st.report_bucket();
                if let Some((_, rows)) = buckets.iter_mut().find(|(k, _)| *k == b) {
                    rows.push(vec![
                        c.tenant_name.clone(),
                        c.building_name.clone(),
                        c.unit_numbers.clone(),
                        c.contract_number.clone(),
                        d(c.end_date),
                        c.remaining_days.to_string(),
                        label(&c.status),
                        label(&c.notice_status),
                        c.latest_response
                            .as_deref()
                            .map(label)
                            .unwrap_or_else(|| "—".into()),
                        format!(
                            "{}%",
                            if c.checklist_total > 0 {
                                c.checklist_done * 100 / c.checklist_total
                            } else {
                                0
                            }
                        ),
                        c.assigned_employee_name
                            .clone()
                            .unwrap_or_else(|| "—".into()),
                        c.outcome_contract_number
                            .clone()
                            .unwrap_or_else(|| "—".into()),
                    ]);
                }
            }
            let sections = buckets
                .into_iter()
                .map(|(k, rows)| Section {
                    heading: match k {
                        ReportBucket::Pending => "Pending",
                        ReportBucket::InProgress => "In Progress",
                        ReportBucket::Completed => "Completed",
                        ReportBucket::NotRenewing => "Not Renewing",
                    }
                    .to_string(),
                    rows,
                })
                .collect();
            (
                [
                    "Tenant",
                    "Building",
                    "Units",
                    "Contract",
                    "End",
                    "Days left",
                    "Renewal status",
                    "Notice",
                    "Tenant response",
                    "Checklist",
                    "Assigned",
                    "New contract",
                ]
                .map(String::from)
                .to_vec(),
                sections,
            )
        }
        ReportKind::UnitSummary => {
            let page = units::list(
                pool,
                &UnitFilter {
                    building_id: f.building_id,
                    ..Default::default()
                },
                &ListQuery::all(Some("building_name"), None),
            )
            .await?;
            let mut sections: Vec<Section> = Vec::new();
            for u in page.items {
                let row = vec![
                    u.unit_number.clone(),
                    u.floor.clone().unwrap_or_default(),
                    u.unit_type.clone().unwrap_or_default(),
                    label(&u.status),
                    u.tenant_name.clone().unwrap_or_else(|| "—".into()),
                    u.contract_number.clone().unwrap_or_else(|| "—".into()),
                    d_opt(u.start_date),
                    d_opt(u.end_date),
                    u.remaining_days
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "—".into()),
                    u.renewal_status
                        .as_deref()
                        .map(label)
                        .unwrap_or_else(|| "—".into()),
                    u.assigned_employee_name
                        .clone()
                        .unwrap_or_else(|| "—".into()),
                ];
                match sections.iter_mut().find(|s| s.heading == u.building_name) {
                    Some(s) => s.rows.push(row),
                    None => sections.push(Section {
                        heading: u.building_name.clone(),
                        rows: vec![row],
                    }),
                }
            }
            (
                [
                    "Unit",
                    "Floor",
                    "Type",
                    "Status",
                    "Tenant",
                    "Contract",
                    "Start",
                    "End",
                    "Days left",
                    "Renewal",
                    "Assigned",
                ]
                .map(String::from)
                .to_vec(),
                sections,
            )
        }
        ReportKind::TenantRenewal => {
            let page = renewals::list(
                pool,
                &CaseFilter {
                    building_id: f.building_id,
                    ..Default::default()
                },
                &ListQuery::all(Some("tenant_name"), None),
            )
            .await?;
            let mut sections: Vec<Section> = Vec::new();
            for c in page.items {
                let row = vec![
                    c.contract_number.clone(),
                    c.building_name.clone(),
                    c.unit_numbers.clone(),
                    d(c.opened_at.date_naive()),
                    d(c.end_date),
                    label(&c.status),
                    label(&c.notice_status),
                    c.latest_response
                        .as_deref()
                        .map(label)
                        .unwrap_or_else(|| "—".into()),
                    c.outcome_contract_number
                        .clone()
                        .unwrap_or_else(|| "—".into()),
                    c.closed_at
                        .map(|t| d(t.date_naive()))
                        .unwrap_or_else(|| "—".into()),
                ];
                match sections.iter_mut().find(|s| s.heading == c.tenant_name) {
                    Some(s) => s.rows.push(row),
                    None => sections.push(Section {
                        heading: c.tenant_name.clone(),
                        rows: vec![row],
                    }),
                }
            }
            (
                [
                    "Contract",
                    "Building",
                    "Units",
                    "Case opened",
                    "Contract end",
                    "Status",
                    "Notice",
                    "Response",
                    "New contract",
                    "Closed",
                ]
                .map(String::from)
                .to_vec(),
                sections,
            )
        }
        ReportKind::NoticeTracking => {
            let page = renewals::list(
                pool,
                &CaseFilter {
                    open_only: true,
                    building_id: f.building_id,
                    assigned_employee_id: f.employee_id,
                    ..Default::default()
                },
                &ListQuery::all(Some("remaining_days"), None),
            )
            .await?;
            let groups = [
                ("Notice Pending", vec!["PENDING", "DRAFT", "NOT_REQUIRED"]),
                ("Notice Sent", vec!["SENT", "DELIVERED", "FAILED"]),
            ];
            let mut sections = Vec::new();
            for (heading, statuses) in groups {
                let rows: Vec<Vec<String>> = page
                    .items
                    .iter()
                    .filter(|c| statuses.contains(&c.notice_status.as_str()))
                    .map(|c| {
                        vec![
                            c.tenant_name.clone(),
                            c.building_name.clone(),
                            c.unit_numbers.clone(),
                            d(c.end_date),
                            c.remaining_days.to_string(),
                            label(&c.notice_status),
                            c.notice_sent_at
                                .map(|t| d(t.date_naive()))
                                .unwrap_or_else(|| "—".into()),
                            c.latest_response
                                .as_deref()
                                .map(label)
                                .unwrap_or_else(|| "No response".into()),
                            if c.open_follow_ups > 0 {
                                format!("{} open", c.open_follow_ups)
                            } else {
                                "none".into()
                            },
                            c.assigned_employee_name
                                .clone()
                                .unwrap_or_else(|| "—".into()),
                        ]
                    })
                    .collect();
                sections.push(Section {
                    heading: heading.to_string(),
                    rows,
                });
            }
            (
                [
                    "Tenant",
                    "Building",
                    "Units",
                    "Expiry",
                    "Days left",
                    "Notice status",
                    "Sent",
                    "Tenant response",
                    "Follow-ups",
                    "Employee",
                ]
                .map(String::from)
                .to_vec(),
                sections,
            )
        }
    };
    let total_rows = sections.iter().map(|s| s.rows.len()).sum();
    Ok(ReportTable {
        kind: kind.slug().to_string(),
        title: kind.title().to_string(),
        subtitle: subtitle.join(" · "),
        generated_at: chrono::Utc::now().to_rfc3339(),
        columns,
        sections,
        total_rows,
    })
}

// ---------------------------------------------------------------- exports

pub fn to_xlsx(report: &ReportTable) -> ServiceResult<Vec<u8>> {
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    sheet
        .set_name(&report.title[..report.title.len().min(31)])
        .map_err(|e| ServiceError::Internal(e.to_string()))?;
    let title_fmt = Format::new().set_bold().set_font_size(14);
    let sub_fmt = Format::new().set_font_color("#666666");
    let head_fmt = Format::new()
        .set_bold()
        .set_background_color("#DCEBE9")
        .set_border_bottom(FormatBorder::Thin);
    let section_fmt = Format::new().set_bold().set_font_color("#146C68");
    let num_fmt = Format::new().set_align(FormatAlign::Right);
    let mut row: u32 = 0;
    let ok = |r: Result<&mut rust_xlsxwriter::Worksheet, rust_xlsxwriter::XlsxError>| {
        r.map(|_| ())
            .map_err(|e| ServiceError::Internal(e.to_string()))
    };
    ok(sheet.write_with_format(row, 0, &report.title, &title_fmt))?;
    row += 1;
    ok(sheet.write_with_format(row, 0, &report.subtitle, &sub_fmt))?;
    row += 2;
    for section in &report.sections {
        ok(sheet.write_with_format(row, 0, &section.heading, &section_fmt))?;
        row += 1;
        for (c, h) in report.columns.iter().enumerate() {
            ok(sheet.write_with_format(row, c as u16, h, &head_fmt))?;
        }
        row += 1;
        for r in &section.rows {
            for (c, v) in r.iter().enumerate() {
                if let Ok(n) = v.parse::<f64>() {
                    ok(sheet.write_number_with_format(row, c as u16, n, &num_fmt))?;
                } else {
                    ok(sheet.write_string(row, c as u16, v))?;
                }
            }
            row += 1;
        }
        row += 1;
    }
    sheet.autofit();
    wb.save_to_buffer()
        .map_err(|e| ServiceError::Internal(e.to_string()))
}

const REPORT_TEMPLATE: &str = r##"
#import sys: inputs
#let d = inputs
#set page(paper: "a4", flipped: true, margin: 1.5cm, footer: context [
  #set text(size: 8pt, fill: luma(110))
  #d.subtitle #h(1fr) #counter(page).display("1 / 1", both: true)
])
#set text(font: ("Segoe UI", "Arial", "Liberation Sans", "DejaVu Sans", "Libertinus Serif"), size: 8.5pt)
#text(size: 15pt, weight: "bold", fill: rgb("#146C68"))[#d.title]
#linebreak()
#text(size: 9pt, fill: luma(90))[#d.subtitle · #d.total_rows rows]
#v(6pt)
#let ncols = d.columns.len()
#for s in d.sections [
  #v(6pt)
  #text(weight: "bold", size: 10pt, fill: rgb("#146C68"))[#s.heading #h(4pt) #text(weight: "regular", fill: luma(120))[(#s.rows.len())]]
  #v(2pt)
  #if s.rows.len() == 0 [ #text(fill: luma(140))[No records.] ] else [
    #table(
      columns: ncols,
      stroke: 0.4pt + luma(200),
      inset: 4pt,
      fill: (x, y) => if y == 0 { rgb("#DCEBE9") } else { none },
      ..d.columns.map(c => text(weight: "bold")[#c]),
      ..s.rows.flatten(),
    )
  ]
]
"##;

fn s(v: &str) -> Value {
    Value::Str(Str::from(v))
}

pub fn to_pdf(report: &ReportTable) -> ServiceResult<Vec<u8>> {
    let mut dict = Dict::new();
    dict.insert("title".into(), s(&report.title));
    dict.insert("subtitle".into(), s(&report.subtitle));
    dict.insert("total_rows".into(), Value::Int(report.total_rows as i64));
    dict.insert(
        "columns".into(),
        Value::Array(report.columns.iter().map(|c| s(c)).collect::<Array>()),
    );
    let sections: Array = report
        .sections
        .iter()
        .map(|sec| {
            let mut sd = Dict::new();
            sd.insert("heading".into(), s(&sec.heading));
            sd.insert(
                "rows".into(),
                Value::Array(
                    sec.rows
                        .iter()
                        .map(|r| Value::Array(r.iter().map(|v| s(v)).collect::<Array>()))
                        .collect::<Array>(),
                ),
            );
            Value::Dict(sd)
        })
        .collect();
    dict.insert("sections".into(), Value::Array(sections));
    let engine = TypstEngine::builder()
        .main_file(REPORT_TEMPLATE)
        .search_fonts_with(TypstKitFontOptions::default())
        .build();
    let doc: typst_layout::PagedDocument = engine
        .compile_with_input(dict)
        .output
        .map_err(|e| ServiceError::Internal(format!("typst: {e}")))?;
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
        .map_err(|e| ServiceError::Internal(format!("pdf: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ReportTable {
        ReportTable {
            kind: "unit-summary".into(),
            title: "Unit-Wise Summary Report".into(),
            subtitle: "Test Org · 11 Sep 2026".into(),
            generated_at: "now".into(),
            columns: vec!["Unit".into(), "Status".into(), "Days left".into()],
            sections: vec![
                Section {
                    heading: "Al Noor Tower".into(),
                    rows: vec![
                        vec!["101".into(), "Occupied".into(), "45".into()],
                        vec!["102".into(), "Vacant".into(), "—".into()],
                    ],
                },
                Section {
                    heading: "Empty Building".into(),
                    rows: vec![],
                },
            ],
            total_rows: 2,
        }
    }

    #[test]
    fn exports_xlsx_and_pdf() {
        let x = to_xlsx(&sample()).unwrap();
        assert!(x.starts_with(b"PK"), "xlsx is a zip");
        let p = to_pdf(&sample()).unwrap();
        assert!(p.starts_with(b"%PDF"));
    }
}
