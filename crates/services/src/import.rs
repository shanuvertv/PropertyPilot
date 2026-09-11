//! Excel import of an existing tenant list (Phase 8; spec §21 "data migration").
//!
//! Accepts the leasing team's own sheet layout: one row per unit with building,
//! tenant, unit number, start/end dates and optional capacity / rent columns.
//! Rows with the same building + tenant + dates become one multi-unit contract.

use std::collections::BTreeMap;
use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use chrono::NaiveDate;
use renewal_core::{Capability, ContractStatus, UnitStatus};
use renewal_db::buildings::{self, BuildingInput};
use renewal_db::contracts::{self, ContractInput, InsertContract};
use renewal_db::tenants::{self, TenantInput};
use renewal_db::units::{self, UnitInput};
use renewal_db::PgPool;
use serde::{Deserialize, Serialize};

use crate::audit_log::{self, NONE};
use crate::error::{ServiceError, ServiceResult};
use crate::session::Session;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedRow {
    pub row: u32,
    pub building: String,
    pub tenant: String,
    pub unit: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub capacity: Option<i64>,
    pub rent_per_unit: Option<f64>,
    pub total_per_annum: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedContract {
    pub building: String,
    pub tenant: String,
    pub units: Vec<String>,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub status: String,
    pub rent_terms: Option<String>,
    pub warnings: Vec<String>,
    pub skip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub rows_read: usize,
    pub rows_skipped: usize,
    pub columns: BTreeMap<String, String>,
    pub buildings_new: Vec<String>,
    pub buildings_existing: Vec<String>,
    pub tenants_new: Vec<String>,
    pub tenants_existing: Vec<String>,
    pub units_new: usize,
    pub units_existing: usize,
    pub contracts: Vec<PlannedContract>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub buildings_created: usize,
    pub units_created: usize,
    pub tenants_created: usize,
    pub contracts_created: usize,
    pub contracts_skipped: usize,
    pub warnings: Vec<String>,
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn header_index(headers: &[String], needles: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let h = h.to_ascii_lowercase();
        needles.iter().any(|n| h.contains(n))
    })
}

fn cell_string(d: &Data) -> String {
    match d {
        Data::String(s) => s.trim().to_owned(),
        Data::Float(f) => {
            if (f.fract()).abs() < f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt
            .as_datetime()
            .map(|d| d.date().format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(_) | Data::Empty => String::new(),
    }
}

fn cell_date(d: &Data) -> Option<NaiveDate> {
    match d {
        Data::DateTime(dt) => dt.as_datetime().map(|x| x.date()),
        Data::Float(f) => NaiveDate::from_ymd_opt(1899, 12, 30)
            .and_then(|b| b.checked_add_signed(chrono::Duration::days(*f as i64))),
        Data::Int(i) => NaiveDate::from_ymd_opt(1899, 12, 30)
            .and_then(|b| b.checked_add_signed(chrono::Duration::days(*i))),
        Data::String(s) | Data::DateTimeIso(s) => {
            let s = s.trim();
            [
                "%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%d %b %Y", "%d %B %Y", "%m/%d/%Y",
            ]
            .iter()
            .find_map(|f| NaiveDate::parse_from_str(s, f).ok())
        }
        _ => None,
    }
}

fn cell_num(d: &Data) -> Option<f64> {
    match d {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => s.trim().replace(',', "").parse().ok(),
        _ => None,
    }
}

/// Rows, the detected column mapping (field -> header text) and per-row warnings.
pub type Parsed = (Vec<ParsedRow>, BTreeMap<String, String>, Vec<String>);

/// Parses the first sheet. Returns the rows plus the detected column mapping.
pub fn parse(bytes: &[u8]) -> ServiceResult<Parsed> {
    let mut wb: Xlsx<_> = Xlsx::new(Cursor::new(bytes))
        .map_err(|e| ServiceError::validation(format!("not a readable .xlsx file: {e}")))?;
    let name = wb
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| ServiceError::validation("the workbook has no sheets"))?;
    let range = wb
        .worksheet_range(&name)
        .map_err(|e| ServiceError::validation(format!("cannot read sheet {name}: {e}")))?;
    let mut rows = range.rows();
    let headers: Vec<String> = rows
        .next()
        .map(|r| r.iter().map(cell_string).collect())
        .unwrap_or_default();
    let col_building = header_index(&headers, &["building", "property"]);
    let col_tenant = header_index(&headers, &["tenant", "customer", "company"]);
    let col_unit = header_index(&headers, &["unit", "room", "shop"]);
    let col_start = header_index(&headers, &["start"]);
    let col_end = header_index(&headers, &["end", "expiry", "expire"]);
    let col_capacity = header_index(&headers, &["capacity", "beds"]);
    let col_rent = header_index(
        &headers,
        &["rent/", "rent per", "rent /", "current rent", "rent\n"],
    );
    let col_total = header_index(&headers, &["total"]);
    let mut mapping = BTreeMap::new();
    for (k, v) in [
        ("building", col_building),
        ("tenant", col_tenant),
        ("unit", col_unit),
        ("start", col_start),
        ("end", col_end),
        ("capacity", col_capacity),
        ("rent", col_rent),
        ("total", col_total),
    ] {
        if let Some(i) = v {
            mapping.insert(k.to_string(), headers.get(i).cloned().unwrap_or_default());
        }
    }
    let (Some(cb), Some(ct), Some(cu), Some(cs), Some(ce)) =
        (col_building, col_tenant, col_unit, col_start, col_end)
    else {
        return Err(ServiceError::validation(format!(
            "could not find the required columns (building, tenant, unit, start date, end date) in the header row: {}",
            headers.join(" | ")
        )));
    };
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    for (i, r) in rows.enumerate() {
        let rownum = (i + 2) as u32;
        let get = |c: usize| r.get(c).cloned().unwrap_or(Data::Empty);
        let building = cell_string(&get(cb));
        let tenant = cell_string(&get(ct));
        let unit = cell_string(&get(cu));
        if building.is_empty() && tenant.is_empty() && unit.is_empty() {
            continue;
        }
        if building.is_empty() || tenant.is_empty() || unit.is_empty() {
            warnings.push(format!(
                "row {rownum}: missing building, tenant or unit — skipped"
            ));
            continue;
        }
        let (Some(start), Some(end)) = (cell_date(&get(cs)), cell_date(&get(ce))) else {
            warnings.push(format!("row {rownum}: unreadable start/end date — skipped"));
            continue;
        };
        if end < start {
            warnings.push(format!(
                "row {rownum}: end date before start date — skipped"
            ));
            continue;
        }
        out.push(ParsedRow {
            row: rownum,
            building,
            tenant,
            unit,
            start,
            end,
            capacity: col_capacity
                .and_then(|c| cell_num(&get(c)))
                .map(|v| v as i64),
            rent_per_unit: col_rent.and_then(|c| cell_num(&get(c))),
            total_per_annum: col_total.and_then(|c| cell_num(&get(c))),
        });
    }
    Ok((out, mapping, warnings))
}

/// "BUILDING 25 114" under building "BUILDING-25" → "114".
fn unit_number(building: &str, unit: &str) -> String {
    let b = norm(building);
    let u = norm(unit);
    if !b.is_empty() && u.starts_with(&b) && u.len() > b.len() {
        // Strip the prefix from the original text, keeping the unit's own casing.
        let mut count = 0;
        let mut idx = 0;
        for (i, ch) in unit.char_indices() {
            if ch.is_ascii_alphanumeric() {
                count += 1;
            }
            if count == b.len() {
                idx = i + ch.len_utf8();
                break;
            }
        }
        let rest = unit[idx..].trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if !rest.is_empty() {
            return rest.to_owned();
        }
    }
    unit.to_owned()
}

fn building_code(name: &str) -> String {
    let code: String = name
        .to_ascii_uppercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let code = code.trim_matches('-').replace("--", "-");
    code.chars().take(20).collect()
}

fn money(v: f64) -> String {
    let whole = v.round() as i64;
    let s = whole.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if whole < 0 {
        format!("-{out}")
    } else {
        out
    }
}

struct Group {
    building: String,
    tenant: String,
    start: NaiveDate,
    end: NaiveDate,
    rows: Vec<ParsedRow>,
}

fn group(rows: &[ParsedRow]) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for r in rows {
        match groups.iter_mut().find(|g| {
            norm(&g.building) == norm(&r.building)
                && norm(&g.tenant) == norm(&r.tenant)
                && g.start == r.start
                && g.end == r.end
        }) {
            Some(g) => g.rows.push(r.clone()),
            None => groups.push(Group {
                building: r.building.clone(),
                tenant: r.tenant.clone(),
                start: r.start,
                end: r.end,
                rows: vec![r.clone()],
            }),
        }
    }
    groups
}

fn rent_terms(g: &Group) -> Option<String> {
    let total: f64 = g.rows.iter().filter_map(|r| r.total_per_annum).sum();
    let beds: i64 = g.rows.iter().filter_map(|r| r.capacity).sum();
    let rents: Vec<f64> = g.rows.iter().filter_map(|r| r.rent_per_unit).collect();
    if total <= 0.0 && rents.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if total > 0.0 {
        parts.push(format!("AED {}/annum", money(total)));
    }
    if beds > 0 {
        parts.push(format!("{beds} beds"));
    }
    if !rents.is_empty() {
        let min = rents.iter().cloned().fold(f64::MAX, f64::min);
        let max = rents.iter().cloned().fold(f64::MIN, f64::max);
        parts.push(if (max - min).abs() < 0.01 {
            format!("AED {}/bed", money(min))
        } else {
            format!("AED {}–{}/bed", money(min), money(max))
        });
    }
    Some(parts.join(" · "))
}

async fn contract_exists<'e>(
    ex: impl renewal_db::sqlx::PgExecutor<'e>,
    tenant_id: uuid::Uuid,
    building_id: uuid::Uuid,
    start: NaiveDate,
    end: NaiveDate,
) -> ServiceResult<bool> {
    Ok(renewal_db::sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM contracts WHERE tenant_id = $1 AND building_id = $2 AND start_date = $3 AND end_date = $4)",
    )
    .bind(tenant_id)
    .bind(building_id)
    .bind(start)
    .bind(end)
    .fetch_one(ex)
    .await?)
}

fn require_import(caller: &Session) -> ServiceResult<()> {
    caller.require(Capability::ManageContracts)?;
    caller.require(Capability::ManageBuildings)?;
    caller.require(Capability::ManageTenants)?;
    Ok(())
}

pub async fn preview(
    pool: &PgPool,
    caller: &Session,
    bytes: &[u8],
) -> ServiceResult<ImportPreview> {
    require_import(caller)?;
    let (rows, columns, mut warnings) = parse(bytes)?;
    let today = renewal_db::today(pool).await?;
    let existing_buildings = buildings::all_active(pool).await?;
    let existing_tenants = tenants::all_active(pool).await?;
    let mut b_new = Vec::new();
    let mut b_existing = Vec::new();
    for name in rows
        .iter()
        .map(|r| r.building.clone())
        .collect::<std::collections::BTreeSet<_>>()
    {
        if existing_buildings
            .iter()
            .any(|b| norm(&b.name) == norm(&name) || norm(&b.code) == norm(&building_code(&name)))
        {
            b_existing.push(name);
        } else {
            b_new.push(name);
        }
    }
    let mut t_new = Vec::new();
    let mut t_existing = Vec::new();
    for name in rows
        .iter()
        .map(|r| r.tenant.clone())
        .collect::<std::collections::BTreeSet<_>>()
    {
        if existing_tenants
            .iter()
            .any(|t| norm(&t.name) == norm(&name))
        {
            t_existing.push(name);
        } else {
            t_new.push(name);
        }
    }
    let mut units_new = 0;
    let mut units_existing = 0;
    let mut seen_units = std::collections::BTreeSet::new();
    for r in &rows {
        let key = (norm(&r.building), norm(&unit_number(&r.building, &r.unit)));
        if !seen_units.insert(key.clone()) {
            warnings.push(format!(
                "row {}: unit {} listed more than once",
                r.row, r.unit
            ));
            continue;
        }
        let bid = existing_buildings
            .iter()
            .find(|b| norm(&b.name) == norm(&r.building))
            .map(|b| b.id);
        let exists = match bid {
            Some(bid) => units::for_building(pool, bid)
                .await?
                .iter()
                .any(|u| norm(&u.unit_number) == key.1),
            None => false,
        };
        if exists {
            units_existing += 1;
        } else {
            units_new += 1;
        }
    }
    let mut contracts = Vec::new();
    for g in group(&rows) {
        let status = if g.end < today {
            ContractStatus::Expired
        } else {
            ContractStatus::Active
        };
        // Same duplicate rule as `commit`: an identical (tenant, building, dates) contract is skipped.
        let tenant_id = existing_tenants
            .iter()
            .find(|t| norm(&t.name) == norm(&g.tenant))
            .map(|t| t.id);
        let building_id = existing_buildings
            .iter()
            .find(|b| {
                norm(&b.name) == norm(&g.building)
                    || norm(&b.code) == norm(&building_code(&g.building))
            })
            .map(|b| b.id);
        let skip = match (tenant_id, building_id) {
            (Some(t), Some(b)) => contract_exists(pool, t, b, g.start, g.end).await?,
            _ => false,
        };
        contracts.push(PlannedContract {
            building: g.building.clone(),
            tenant: g.tenant.clone(),
            units: g
                .rows
                .iter()
                .map(|r| unit_number(&g.building, &r.unit))
                .collect(),
            start: g.start,
            end: g.end,
            status: status.to_string(),
            rent_terms: rent_terms(&g),
            warnings: if skip {
                vec!["identical contract already exists".into()]
            } else {
                vec![]
            },
            skip,
        });
    }
    Ok(ImportPreview {
        rows_read: rows.len(),
        rows_skipped: warnings.len(),
        columns,
        buildings_new: b_new,
        buildings_existing: b_existing,
        tenants_new: t_new,
        tenants_existing: t_existing,
        units_new,
        units_existing,
        contracts,
        warnings,
    })
}

/// Creates everything in one transaction. Re-running the same file is safe:
/// existing buildings/tenants/units are reused and identical contracts are skipped.
pub async fn commit(pool: &PgPool, caller: &Session, bytes: &[u8]) -> ServiceResult<ImportResult> {
    require_import(caller)?;
    let (rows, _columns, mut warnings) = parse(bytes)?;
    let today = renewal_db::today(pool).await?;
    let mut result = ImportResult::default();
    let mut tx = pool.begin().await?;

    // Buildings
    let mut building_ids: BTreeMap<String, uuid::Uuid> = BTreeMap::new();
    let existing = buildings::all_active(&mut *tx).await?;
    for name in rows
        .iter()
        .map(|r| r.building.clone())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let id = match existing
            .iter()
            .find(|b| norm(&b.name) == norm(&name) || norm(&b.code) == norm(&building_code(&name)))
        {
            Some(b) => b.id,
            None => {
                let id = buildings::insert(
                    &mut *tx,
                    &BuildingInput {
                        name: name.clone(),
                        code: building_code(&name),
                        location: None,
                        building_type: None,
                        notes: Some("Imported from Excel".into()),
                    },
                    caller.user_id,
                )
                .await?;
                audit_log::log(
                    &mut *tx,
                    caller,
                    "building",
                    id,
                    "IMPORTED",
                    NONE,
                    Some(&name),
                )
                .await?;
                result.buildings_created += 1;
                id
            }
        };
        building_ids.insert(norm(&name), id);
    }

    // Tenants
    let mut tenant_ids: BTreeMap<String, uuid::Uuid> = BTreeMap::new();
    let existing_t = tenants::all_active(&mut *tx).await?;
    for name in rows
        .iter()
        .map(|r| r.tenant.clone())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let id = match existing_t.iter().find(|t| norm(&t.name) == norm(&name)) {
            Some(t) => t.id,
            None => {
                let id = tenants::insert(
                    &mut *tx,
                    &TenantInput {
                        name: name.clone(),
                        contact_person: None,
                        mobile: None,
                        email: None,
                        alt_contact: None,
                        address: None,
                        notes: Some("Imported from Excel".into()),
                    },
                    caller.user_id,
                )
                .await?;
                audit_log::log(
                    &mut *tx,
                    caller,
                    "tenant",
                    id,
                    "IMPORTED",
                    NONE,
                    Some(&name),
                )
                .await?;
                result.tenants_created += 1;
                id
            }
        };
        tenant_ids.insert(norm(&name), id);
    }

    // Units
    let mut unit_ids: BTreeMap<(String, String), uuid::Uuid> = BTreeMap::new();
    for r in &rows {
        let bkey = norm(&r.building);
        let number = unit_number(&r.building, &r.unit);
        let key = (bkey.clone(), norm(&number));
        if unit_ids.contains_key(&key) {
            continue;
        }
        let building_id = building_ids[&bkey];
        let existing_units = units::for_building(&mut *tx, building_id).await?;
        let id = match existing_units
            .iter()
            .find(|u| norm(&u.unit_number) == key.1)
        {
            Some(u) => u.id,
            None => {
                let id = units::insert(
                    &mut *tx,
                    &UnitInput {
                        building_id,
                        unit_number: number.clone(),
                        floor: None,
                        unit_type: r.capacity.map(|c| format!("{c} beds")),
                        status: UnitStatus::Vacant.to_string(),
                        notes: None,
                    },
                    caller.user_id,
                )
                .await?;
                result.units_created += 1;
                id
            }
        };
        unit_ids.insert(key, id);
    }

    // Contracts
    let mut next_number: i64 = renewal_db::sqlx::query_scalar("SELECT count(*) FROM contracts")
        .fetch_one(&mut *tx)
        .await?;
    for g in group(&rows) {
        let building_id = building_ids[&norm(&g.building)];
        let tenant_id = tenant_ids[&norm(&g.tenant)];
        let mut ids: Vec<uuid::Uuid> = g
            .rows
            .iter()
            .filter_map(|r| {
                unit_ids
                    .get(&(norm(&g.building), norm(&unit_number(&g.building, &r.unit))))
                    .copied()
            })
            .collect();
        ids.sort();
        ids.dedup();
        // Identical contract already there? (same tenant, building, dates)
        if contract_exists(&mut *tx, tenant_id, building_id, g.start, g.end).await? {
            result.contracts_skipped += 1;
            warnings.push(format!(
                "{} @ {} {}–{}: already exists, skipped",
                g.tenant, g.building, g.start, g.end
            ));
            continue;
        }
        let active = g.end >= today;
        if active {
            let busy = units::occupied_by_other_contract(&mut *tx, &ids, None).await?;
            if !busy.is_empty() {
                warnings.push(format!(
                    "{} @ {}: {} unit(s) already under an active contract were left out",
                    g.tenant,
                    g.building,
                    busy.len()
                ));
                ids.retain(|u| !busy.contains(u));
            }
        }
        if ids.is_empty() {
            result.contracts_skipped += 1;
            continue;
        }
        next_number += 1;
        let number = format!("C-{next_number:04}");
        let notes = g
            .rows
            .iter()
            .map(|r| {
                let mut s = unit_number(&g.building, &r.unit);
                if let Some(c) = r.capacity {
                    s.push_str(&format!(": {c} beds"));
                }
                if let Some(rent) = r.rent_per_unit {
                    s.push_str(&format!(" @ AED {}", money(rent)));
                }
                if let Some(t) = r.total_per_annum {
                    s.push_str(&format!(" = AED {}/annum", money(t)));
                }
                s
            })
            .collect::<Vec<_>>()
            .join("\n");
        let input = ContractInput {
            contract_number: number,
            tenant_id,
            building_id,
            unit_ids: ids.clone(),
            start_date: g.start,
            end_date: g.end,
            rent_terms: rent_terms(&g),
            assigned_employee_id: None,
            notes: Some(format!("Imported from Excel\n{notes}")),
        };
        let status = if active {
            ContractStatus::Active
        } else {
            ContractStatus::Expired
        };
        let id = contracts::insert(
            &mut tx,
            &InsertContract {
                input: &input,
                status: &status.to_string(),
                previous_contract_id: None,
                root_contract_id: None,
                renewal_sequence: 0,
                created_by: caller.user_id,
            },
        )
        .await?;
        if active {
            units::set_status_many(&mut *tx, &ids, &UnitStatus::Occupied.to_string(), None).await?;
        }
        audit_log::log(&mut *tx, caller, "contract", id, "IMPORTED", NONE, Some(&serde_json::json!({ "tenant": g.tenant, "building": g.building, "units": ids.len(), "status": status.to_string() }))).await?;
        result.contracts_created += 1;
    }
    tx.commit().await?;
    result.warnings = warnings;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_building_prefix_from_unit_numbers() {
        assert_eq!(unit_number("BUILDING-25", "BUILDING 25 114"), "114");
        assert_eq!(unit_number("BUILDING-25", "BUILDING 25 G01"), "G01");
        assert_eq!(unit_number("Marina Plaza", "G-01"), "G-01");
        assert_eq!(
            unit_number("Al Noor Tower", "Al Noor Tower"),
            "Al Noor Tower"
        );
    }

    #[test]
    fn building_codes_and_money() {
        assert_eq!(building_code("BUILDING-25"), "BUILDING-25");
        assert_eq!(
            building_code("Al Noor Tower (Phase 2)"),
            "AL-NOOR-TOWER-PHASE-"
        );
        assert_eq!(money(76800.0), "76,800");
        assert_eq!(money(478.0), "478");
        assert_eq!(money(1234567.0), "1,234,567");
    }
}
