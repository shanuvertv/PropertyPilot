# PropertyPilot — Implementation Plan (Tauri + Rust, Windows + Android)

**Source:** `1.docx` (21-section functional spec)
**Target:** greenfield build in `D:\Umair` — Windows desktop app now, Android app later, both Tauri 2 + Rust with one shared React UI, backed by a Rust API server
**Status:** Phases 0–9 built (11 Sep 2026) — Windows app, Android app and server are feature-complete against the spec. Open items need the client/IT: Android SDK build + signing keystore, code-signing certificate, Graph app registration or the mailbox's SMTP/IMAP credentials, the Lightsail database + instance, and UAT. Repo: https://github.com/shanuvertv/PropertyPilot

---

## 1. What we are building

A Windows desktop app (and, later, an Android app from the same codebase) for the leasing team that tracks **Property → Unit → Tenant → Contract**, computes contract expiry automatically, drives a **renewal workflow** (case → notice → tenant response → follow-ups → completion), sends emails through Microsoft 365, reminds staff on a configurable schedule, and gives management a real-time dashboard plus reports.

Explicit business rule from the spec: this is **not** an accounting or payment system. No ledger, invoices, cheques, collections, reconciliation or financial reporting. The optional "rent amount" field is display-only text.

### Why the apps need a server
The spec is multi-user (Admin, Leasing, Operations, Management), §8 reminders / §15 notifications must fire whether or not anyone has the app open, and Android phones are off the office LAN and must never hold database credentials. So the deliverable is three parts sharing one Rust workspace:

1. **Clients** — `renewal-desktop` (Tauri 2, Windows installer) and later the Android app built from the *same* Tauri project and React UI. The Rust side of the client is thin: secure token storage, notifications, updater.
2. **`renewal-server`** — one Rust (Axum) process that owns the database: HTTP API, bearer-token sessions, email sending, and the in-process scheduler (daily expiry sweep, email queue, heartbeat). Runs as a Windows Service on the office server or in Azure.
3. **PostgreSQL 16** — reachable only by the server.

---

## 2. Key decisions

| # | Area | Decision | Why |
|---|---|---|---|
| D1 | Clients | **Tauri 2** shell hosting one **React + TypeScript (Vite)** UI, Tailwind + shadcn/ui; Windows first, Android from the same project (`tauri android init`) | Native apps as requested; one UI codebase for both platforms |
| D2 | Backend | **`renewal-server`: Axum HTTP API** that owns PostgreSQL 16 (`sqlx`), sessions, email and the scheduler. Clients never touch the database | Required once Android is in scope (phones are off-LAN); proper security boundary; a web client later is free |
| D3 | Shared types | `crates/api` holds every request/response type; mirrored in `apps/desktop/src/api/types.ts` (automatic TypeScript export via `specta` is a Phase 1 follow-up once its rc API is pinned) | Server and both clients stay in sync from one definition |
| D4 | Authentication | Local accounts (argon2) managed by Admin; **opaque bearer tokens** stored hashed in `sessions`, 30-day lifetime, revocable; client keeps the token in the platform secure store (Windows Credential Manager / Android keystore via `keyring`). Microsoft Entra ID SSO optional in Phase 8 | Spec asks for secure login with role-based access; works identically on desktop and mobile; deactivating a user signs them out everywhere |
| D5 | Automation | Scheduler **inside `renewal-server`** (`tokio-cron-scheduler`): heartbeat every minute, daily 00:05 org-time sweep, email queue; Postgres advisory lock if more than one server instance runs | One process to install and monitor; reminders fire unattended |
| D6 | Email | Server sends via **Microsoft Graph** `sendMail` (app-only `Mail.Send`, `reqwest` + `oauth2`), `lettre` SMTP fallback, behind one `MailProvider` trait; clients only queue messages through the API | Only the server holds mail secrets; retries and delivery status centralised |
| D7 | Templates | `handlebars` crate for email and notice templates | Matches the spec's `{{TenantName}}` placeholder syntax exactly; HTML-escaped by default |
| D8 | PDF | **Typst** via `typst-as-lib` on the server for renewal notices and PDF reports; fallback `genpdf` | Pure Rust, letterhead-quality output, no Chromium; one implementation serves both platforms |
| D9 | Excel | `rust_xlsxwriter` on the server; clients download through the API | Native, fast, styled exports |
| D10 | Documents | `StorageProvider` trait on the server: **Postgres `bytea`** (default v1) · UNC file share · Azure Blob via `object_store`; clients upload/download via the API | One backup, no storage credentials on devices; can move to Blob later |
| D11 | Live updates | Server pushes changes over **Server-Sent Events** (fed internally by Postgres `LISTEN/NOTIFY`); client invalidates TanStack Query caches; native toasts via `tauri-plugin-notification` | Dashboard and notification bell update without polling, on both platforms |
| D12 | Distribution | Windows: Tauri MSI/NSIS + `tauri-plugin-updater`, WebView2 bootstrapper bundled. Android: signed APK/AAB via MDM or Play (Q14). Server: Windows Service (`sc create`) or Azure App Service container | Standard rollout paths; code-signing certificate recommended (§11) |

**Rejected — direct-to-database desktop client.** Simplest for a LAN-only Windows tool, but it puts database credentials on every device and cannot serve phones outside the office; dropped the moment Android entered scope.

---

## 3. Spec clarifications baked into the design

The spec describes the same concepts slightly differently in different sections. These resolutions keep the system consistent; each needs a "yes" from the client.

1. **One remaining-days value, one band scale, used everywhere.**  
   `remaining_days = contract_end_date − today` (calendar days, org timezone).  
   Bands: 🔴 Expired (< 0) · 🟠 0–30 (Urgent) · 🟡 31–60 · 🟢 61–90 · 🔵 91–120 · ⚪ > 120.  
   Sections 1, 6 and 16 each show a different grouping/colouring; all three become views of this one scale. Thresholds live in Settings (`expiring_soon_days` = 90, `urgent_days` = 30 — matching the §8 reminder rules).

2. **Derived statuses are not stored.** A contract stores only `Draft / Active / Renewed / Expired / Terminated`. "Expiring Soon" = Active AND remaining ≤ 90. "Renewal In Progress" = Active AND has an open renewal case. This prevents status drift between the contract, the case and the dashboard.

3. **Tenant response drives the renewal status.** Recording a response auto-moves the case:  
   No Response → Waiting for Tenant Response · Interested in Renewal → Tenant Interested · Under Discussion → Under Negotiation · Renewal Confirmed → Renewal Confirmed · Not Interested → Tenant Not Renewing · Will Vacate → Vacating.  
   Responses are kept as a history (the tenant profile must list "Tenant Responses").

4. **11 renewal statuses roll up to the 4 report buckets:**  
   Pending = Not Started, Notice Pending · In Progress = Notice Sent, Waiting, Tenant Interested, Under Negotiation, Renewal Confirmed · Completed = Renewal Completed, Closed (after completion) · Not Renewing = Tenant Not Renewing, Vacating.

5. **§3 "Unit-Wise Summary" and §16 "Unit-Wise Summary Dashboard" are one screen** with filters, search, colour indicators and quick actions.

6. **Unit status sync rule:** contract activation → Occupied; contract Expired/Terminated with no successor → Vacant (unless Under Maintenance); Reserved / Under Maintenance are set manually by Operations.

7. **Multi-unit contracts** are supported (`contract_units` join); unit-wise screens show one row per unit.

8. **Dashboard card definitions:** "Renewals Pending" = open renewal cases + expiring-soon contracts with no case yet; "Renewals Completed" = cases completed in the selected period (default: last 90 days).

---

## 4. Architecture

```
┌──────────── Windows PC ────────────┐   ┌──────────── Android phone (later) ─────────┐
│ renewal-desktop (Tauri 2)          │   │ renewal-desktop (Tauri 2, same project)      │
│  WebView2: React UI ───────────────┼─┐ │  Android WebView: same React UI ─────────────┼─┐
│  Rust: secure store, toasts,       │ │ │  Rust: keystore, notifications              │ │
│        tray, updater               │ │ │                                              │ │
└────────────────────────────────────┘ │ └──────────────────────────────────────────────┘ │
                 HTTPS · JSON · Authorization: Bearer <token> · SSE for live updates        │
┌──────────────────────────────────────▼──────────────────────────────────────────────────▼─┐
│ renewal-server (Axum, Windows Service or Azure container)                                  │
│  routes/ ─► services/ (permission check → transaction → audit) ─► db/ (sqlx) ─► PostgreSQL │
│  scheduler: heartbeat 1 min · daily sweep 00:05 org TZ · email queue · session purge       │
│  MailProvider (Graph | SMTP | log) · StorageProvider (Postgres | share | Blob)              │
│  holds all secrets: DATABASE_URL, Graph client secret                                      │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

**Cargo workspace** (as built in Phase 0)

```
D:\Umair
├─ Cargo.toml                    # workspace
├─ crates/
│  ├─ core/                      # pure domain: roles + permission matrix, status enums and state
│  │                             #   machines, expiry engine — no IO, unit-tested (17 tests)
│  ├─ api/                       # wire types shared by server and clients
│  ├─ db/                        # sqlx pool, embedded migrations/, repositories, audit writer
│  └─ services/                  # use-cases: auth + sessions, users, system; MailProvider/StorageProvider
├─ apps/
│  ├─ server/                    # renewal-server: Axum routes, bearer auth extractor, scheduler
│  └─ desktop/                   # Tauri 2
│     ├─ src-tauri/              # thin Rust shell: secure_get/set/delete, platform
│     └─ src/                    # React + TS: api/ (client, types), lib/ (app state, nav, bands),
│                                #   components/ (Shell, ui/), pages/ (Setup, Bootstrap, Login, …)
├─ .github/workflows/ci.yml      # fmt, clippy, cargo test (with Postgres), vitest, build, Windows installer
├─ docker-compose.yml            # dev Postgres
├─ README.md · PLAN.md
```

Cross-cutting: global search (`pg_trgm`), pagination on every list, org timezone from Settings, `tracing` logs, daily `pg_dump` backups, a "disconnected" banner in the client when the server is unreachable (no offline editing in v1).

---

### Hosting decision (11 Sep 2026)

The database will live in **AWS Lightsail** (managed PostgreSQL). The server therefore runs on a Lightsail Linux instance next to it (systemd unit or the container image), behind Caddy for HTTPS; both apps talk to `https://<host>`. The Windows-Service install stays as the on-premises alternative. Runbook: `deploy/lightsail.md`.

## 5. Data model

| Table | Key fields | Notes |
|---|---|---|
| `users` | name, email, role (`ADMIN` `LEASING` `OPERATIONS` `MANAGEMENT`), password_hash (argon2), active, last_login_at | Roles assigned in-app by Admin |
| `sessions` | user_id, token_hash (SHA-256), user_agent, expires_at, last_seen_at, revoked_at | Bearer tokens; revoked on logout / deactivation |
| `buildings` | name, code (unique), location, building_type, notes | Total/active/vacant unit counts are derived |
| `units` | building_id, unit_number, floor?, unit_type, status (`VACANT` `OCCUPIED` `RESERVED` `MAINTENANCE`), notes | unique (building_id, unit_number); current tenant/contract derived |
| `tenants` | name, contact_person, mobile, email, alt_contact, address, notes | |
| `contracts` | contract_number (unique), tenant_id, building_id, start_date, end_date, duration_months, rent_amount? (display only), status (`DRAFT` `ACTIVE` `RENEWED` `EXPIRED` `TERMINATED`), assigned_employee_id, previous_contract_id?, root_contract_id, renewal_sequence | Chain fields give Original → Renewal 1 → Renewal 2 timeline |
| `contract_units` | contract_id, unit_id | Multi-unit contracts |
| `renewal_cases` | contract_id, status (11 values, §7), assigned_employee_id, is_urgent, opened_by/at, closed_at, latest_response, next_follow_up_date | One open case per contract |
| `renewal_responses` | case_id, response (6 values, §11), response_date, notes, follow_up_date, recorded_by | History; latest is denormalised on the case |
| `renewal_notices` | case_id, status (`NOT_REQUIRED` `PENDING` `DRAFT` `SENT` `DELIVERED` `FAILED`), proposed_period, other_terms, subject, body_html, pdf_document_id, email_message_id, sent_at, sent_by, recipient, cc | §7 step 4 fields |
| `follow_ups` | case_id, due_date, type (`CALL` `EMAIL` `MEETING` `WHATSAPP` `INTERNAL`), assigned_employee_id, notes, status (`OPEN` `DONE` `CANCELLED`) | Today / Overdue / Upcoming views |
| `checklist_templates`, `checklist_template_items` | label, sort, required | Admin-editable (§13) |
| `renewal_checklist_items` | case_id, label, done, done_by/at | Copied from template when a case opens; progress % derived |
| `email_templates` | key, name, subject, body_html, active | `{{TenantName}}`-style placeholders (§10) |
| `email_messages`, `email_attachments` | type, tenant_id?, contract_id?, case_id?, to, cc, subject, body_html, sent_by, status (`QUEUED` `SENT` `DELIVERED` `FAILED`), attempts, provider_message_id, error, sent_at | Communication history; the worker drains `QUEUED` |
| `notifications` | user_id, type (6 values, §15), title, entity_type, entity_id, read_at | Deep-links to tenant/unit/contract/case; `NOTIFY` on insert |
| `reminder_rules` | days_before, label, notify_in_app, email_assigned_employee, mark_urgent, active | Seeded with §8 defaults; Admin-editable |
| `reminder_dispatches` | contract_id, rule_id, dispatched_at | Idempotency — a reminder fires once per contract |
| `documents`, `document_blobs` | entity_type, entity_id, file_name, mime, size, storage_key / bytes, uploaded_by | Buildings, tenants, contracts, notice PDFs |
| `audit_logs` | actor_id, entity_type, entity_id, action, before_json, after_json, created_at | §19 |
| `settings` | key, value_json | Org name, timezone, thresholds, sending mailbox, letterhead |
| `worker_status` | version, hostname, last_heartbeat_at, last_sweep_at, last_sweep_summary | Scheduler heartbeat; Settings shows "Stale" after 5 min without one |
| **view** `v_contract_expiry` | contract_id, remaining_days, band, effective_status, is_urgent | Joined by every list, dashboard and report |

Migrations are embedded (`sqlx::migrate!`) and applied by `renewal-server` at startup.

---

## 6. Core logic (all in `crates/core` + `crates/services`)

### 6.1 Contract lifecycle
`Draft → Active → Renewed | Expired | Terminated`  
Overlay (derived): *Expiring Soon* (Active, remaining ≤ 90) · *Renewal In Progress* (Active, open case) · *Urgent* (remaining ≤ 30).

### 6.2 Renewal case lifecycle (§7 step 2)
`Not Started → Notice Pending → Notice Sent → Waiting for Tenant Response → Tenant Interested → Under Negotiation → Renewal Confirmed → Renewal Completed → Closed`  
Branch: `… → Tenant Not Renewing → Vacating → Closed`. Allowed transitions are a Rust `match` in `core` with exhaustive tests; every change audited.

### 6.3 Renewal notice lifecycle (§9)
`Not Required | Pending → Draft → Sent → Delivered | Failed`. Sending sets `Sent` and stores date, sender, recipient, subject, content, attachment and delivery status. Failed sends can be retried from the UI.

### 6.4 Renewal completion (§14) — one `sqlx` transaction
Pre-conditions: case status = Renewal Confirmed; all *required* checklist items done.
1. Create new contract copying tenant, building, units, assigned employee; new dates; status Active; `previous_contract_id` = old, `root_contract_id` = old.root ?? old.id, `renewal_sequence` + 1; suggested number e.g. `C-0042-R2`.
2. Set old contract → Renewed (kept as historical record).
3. Units stay Occupied, now pointing at the new contract.
4. Case → Renewal Completed; checklist item "Renewal Completed" ticked; notification to management.
5. Audit entries for all of the above. Contract detail shows the full timeline Original → R1 → R2 → R3.

### 6.5 Daily expiry sweep (§8) — server scheduler, 00:05 org time; Admin can trigger "Run now" from Settings
```
for each ACTIVE contract:
  rd = end_date − today
  applicable = active reminder_rules where rd ≤ days_before and no dispatch for (contract, rule)
  fire the tightest applicable rule only (a contract entered with 20 days left fires the 30-day rule,
  not 120/90/60 as well); mark the earlier milestones as skipped
     → in-app notification to assigned employee (+ Admins), optional internal email, optional urgent flag
  if rd ≤ expiring_soon_days and no renewal case → auto-open case as "Not Started"   (see Q5)
  if rd < 0 and not renewed → status = Expired, units → Vacant, "Contract Expired" notification
for each OPEN follow-up: due today → "Follow-Up Due Today"; overdue → "Overdue Follow-Up" (once per day)
```
Default rules seeded: 120 (internal reminder) · 90 (show in Renewal Dashboard) · 60 (email assigned employee) · 30 (mark Urgent) · 15 (urgent reminder) · 7 (final reminder) · 0 (expire).  
The sweep is a pure function `sweep(today, contracts, rules, dispatches) -> Vec<Action>` in `core`, so it is tested by time-travel without a database.

### 6.6 Email sending
Client: compose (template + placeholders, preview, edit, To, CC) → `POST /api/emails` → server inserts `email_messages` as `QUEUED`.
Scheduler: picks up the row, sends via Graph (`POST /users/{sharedMailbox}/sendMail`), sets `SENT` + provider id, or `FAILED` after 3 attempts with the error stored; the desktop UI shows the status change live.  
Placeholders: `{{TenantName}} {{ContactPerson}} {{BuildingName}} {{UnitNumber}} {{ContractNumber}} {{ContractEndDate}} {{RemainingDays}} {{ProposedRenewalPeriod}} {{ResponsibleEmployee}} {{CompanyName}}`. Handlebars strict mode flags unknown placeholders before send.

### 6.7 Permission matrix (§18)

| Capability | Admin | Leasing | Operations | Management |
|---|:-:|:-:|:-:|:-:|
| Dashboard, renewal status monitoring | ✓ | ✓ | units view only | ✓ |
| Buildings — manage | ✓ | ✓ | view | view |
| Units — manage / update status | ✓ / ✓ | ✓ / ✓ | – / ✓ | view |
| Tenants — manage | ✓ | ✓ | view | view |
| Contracts — manage | ✓ | ✓ | – | view |
| Renewals — start / update, tenant responses | ✓ | ✓ | – | view |
| Notices & emails — send | ✓ | ✓ | – | – |
| Follow-ups — create / update | ✓ | ✓ | – | view |
| Reports — view / export | ✓ | ✓ | – | ✓ |
| Settings (users, templates, reminder rules, checklist, thresholds, DB init) | ✓ | – | – | – |
| Audit trail | ✓ | own records | – | view |

Every API handler resolves the bearer token to a `Session` and every service function calls `role.require(capability)` before touching the database (implemented in `renewal_core::roles`, 6 tests). `GET /api/auth/me` returns the role's capability list, so the UI hides what a role cannot use without duplicating the matrix.

---

## 7. Screens (sidebar per §20)

| Nav item | Screens | Spec sections |
|---|---|---|
| Dashboard | 10 summary cards · expiry summary (bands) · 5 tables (Urgent Renewals, Upcoming Expiries, Pending Tenant Responses, Notices Pending, Recently Completed) with Tenant/Building/Unit/End Date/Remaining/Status/Action columns | 1 |
| Properties / Buildings | List · detail (summary cards, units tab, documents, notes) · create/edit | 2 |
| Units | **Unit-Wise Summary** (filters: building, status, tenant, expiry, renewal status; search; colour indicators; quick actions: View Tenant, View Contract, Start Renewal, Send Notice, Add Follow-Up) · unit detail · create/edit | 3, 16 |
| Tenants | List · profile tabs: Current (units, contracts, dates, remaining days) / History (contracts, renewals, notices, responses) / Communications / Documents | 4 |
| Contracts | List · detail (units, document, timeline Original → R1 → R2, audit history) · create/edit · terminate · **Renew** action | 5, 14 |
| Renewals | Renewal Dashboard by band (Expired, 0–30, 31–60, 61–90, 91–120) · case detail (status, response, follow-ups, checklist %, notice, communications) | 6, 7, 11, 13 |
| Renewal Notices | Notice Tracking table (Tenant/Building/Unit/Expiry/Required/Sent/Sent Date/Response/Status; filters) · **Notice builder**: auto-populate → edit → save draft → preview → generate PDF (Typst) → send (To/CC, preview email) | 7, 9 |
| Follow-Ups | Today / Overdue / Upcoming; create from case or unit row | 12 |
| Email Communication | Global log (date, recipient, subject, type, sent by, status) · compose from template · view sent message | 10 |
| Notifications | Bell with unread count (live via `LISTEN`) · centre with 6 types, click-through to tenant/unit/contract/case · native Windows toast when the app is running/in tray | 15 |
| Reports | Contract Expiry · Renewal Status · Unit-Wise Summary · Tenant Renewal History · Notice Tracking — each with filters and Excel/PDF export via the native save dialog | 17 |
| Settings (Admin) | Users & roles ✅ · scheduler status ✅ · email templates · reminder schedule · checklist template · thresholds & colours · org profile / letterhead / sending mailbox · audit trail viewer | 8, 10, 13, 18, 19 |

Before any of these, the client shows **Setup** (server address, saved in the secure store) → **Bootstrap** (first Admin, only while the server has no users) → **Login** — all built in Phase 0.

Desktop niceties (cheap with Tauri): open a contract/tenant in a second window; minimise to tray with unread badge; `Ctrl+K` global search palette; "Open PDF" via the default viewer (`tauri-plugin-opener`). The UI is laid out to also work at phone widths so the Android phase is packaging, not a redesign.

---

## 8. Delivery phases

Each phase ends with something demoable. Effort assumes one senior developer comfortable in both Rust and React; treat as ±30% until Phase 1 is done. Two developers (React / Rust) compress the Windows scope to roughly 9 weeks.

| Phase | Scope | Done when | Est. |
|---|---|---|---|
| **0 — Foundation** ✅ *(11 Sep 2026)* | Cargo workspace (`core`, `api`, `db`, `services`, `server`, `desktop`); Tauri 2 + React + Vite + Tailwind + shadcn; `sqlx` + embedded migrations (`users`, `settings`, `audit_logs`, `worker_status`, `sessions`); Axum server with health / bootstrap / login / logout / me / users / system-status routes; bearer-token sessions; permission matrix + status state machines + expiry bands in `core` with tests; audit writer; `MailProvider` / `StorageProvider` traits; scheduler heartbeat; client Setup → Bootstrap → Login → Shell with the 12-item sidebar, role-gated routes, Settings (users, scheduler status); CI workflow. | Login works; Management is blocked from Settings; an audit row is written on a test entity | 1.5 wk |
| **1 — Master data** ✅ *(built 11 Sep 2026)* | Buildings CRUD + documents + building summary; Units CRUD + status + Unit-Wise Summary skeleton (filters, search, sort, pagination); Tenants CRUD + documents + profile skeleton | Create building → units → tenant end to end; upload/download documents; audit entries | 1.5 wk |
| **2 — Contracts & expiry engine** ✅ *(built 11 Sep 2026)* | Contract CRUD (multi-unit, document, assigned employee), status machine, `v_contract_expiry` view, bands + thresholds in Settings, unit status sync, chain fields, tenant Current/History tabs, unit-wise summary fully populated with colours | Boundary tests for bands pass (−1, 0, 30, 31, 60, 61, 90, 91, 120); terminate → unit Vacant | 1.5 wk |
| **3 — Dashboards & search** ✅ *(built 11 Sep 2026)* | Main dashboard (cards, expiry summary, 5 tables), building summary dashboard, global search (`pg_trgm`) + `Ctrl+K` palette | Reconciliation test: every card total equals the matching filtered list count | 1 wk |
| **4 — Renewal management** ✅ *(built 11 Sep 2026)* | Renewal Dashboard by band; renewal cases with 11-status machine; tenant responses (history + auto-status + colour highlights); follow-ups (types, Today/Overdue/Upcoming, Follow-Ups page); checklist (admin template, per-case progress %); **renewal completion transaction** + contract timeline | Open case → confirm → complete produces linked new contract, old marked Renewed, timeline shows Original → Renewal 1 | 2 wk |
| **5 — Email, notices, tracking** ✅ *(built 11 Sep 2026)* | Email templates admin (handlebars placeholders, preview); email queue + worker sender (Graph + SMTP fallback, retries); compose flow (preview, edit, To, CC); notice builder (auto-populate, edit, draft, preview, Typst PDF, send) → notice status + all §7-step-4 fields; Notice Tracking screen; communication history on tenant and contract; Email Communication page | Notice sent from the UI lands in a real mailbox with the PDF attached; failed send shows Failed and can be retried | 2 wk |
| **6 — Automation & notifications** ✅ *(built 11 Sep 2026)* | Worker cron sweep (pure `core::sweep` + DB adapter); reminder rules table + Admin UI; idempotent dispatch with catch-up; urgent flag; expired marking; notification centre (bell + page + deep links, 6 types) fed by `LISTEN/NOTIFY`; native toasts + tray badge; internal reminder emails; follow-up due/overdue notifications; worker heartbeat shown in Settings | Time-travel tests: advancing the clock fires each reminder exactly once; Admin edits schedule and next run honours it; toast appears on a new urgent notification | 1 wk |
| **7 — Reports & audit UI** ✅ *(built 11 Sep 2026)* | 5 reports with filters; Excel (`rust_xlsxwriter`) + PDF (Typst) export via native save dialog; audit trail viewer + per-record History tab | Each export matches on-screen data | 1 wk |
| **8 — Packaging, hardening & launch** 🟡 *(code done 11 Sep 2026; external items open)* | **Done:** MSI/NSIS installer with WebView2 bootstrapper; server as a Windows Service (`renewal-server --service`, `installers/install-server.ps1`) with optional built-in TLS and file logging; login rate limiting; change-password + admin reset (sessions revoked); Excel tenant-list import (preview → commit, idempotent); production CSP; permission-matrix checks in every service; runbook in README. **Open (needs the client/IT):** code-signing certificate, updater endpoint + signing keys, Graph app registration, production DB + backups + monitoring, UAT with the leasing team, optional Entra ID SSO. | UAT sign-off; installed app on a user PC logs in and sends one real notice; the server runs the sweep unattended overnight | 2 wk |
| **10 — Tenants per unit & expenses** ✅ *(built 13 Sep 2026)* | Number of tenants entered on the contract per unit (no per-person register, by the client's choice; a unit reads the number from its active contract); rent amount on the contract (renewal completion can set the new rent); Import data page — the Excel import also maps capacity → number of tenants and total rent/annum → rent, and back-fills them on re-import; expenses per unit with categories, equal split between that many people with a "paid so far" count, documents; Expenses dashboard (monthly trend, by property, by unit, by category, outstanding) and per-unit charts; new capabilities per role; Admin user deletion (soft); search box + filters on every list (server-side `q` on paged lists, local filter on detail tables), richer global search (units by building, contracts by tenant, expenses); Operations lands on Units instead of a forbidden dashboard; table ⇄ card layout toggle on every list (cards = heading, status badges, stat tiles for the key figures, details, actions; remembered per list) | Bill split adds up exactly; dashboard totals reconcile with the list; link crawl per role finds no dead links | 1 wk |
| **9 — Android** 🟡 *(code done 11 Sep 2026; device build open)* | **Done:** Tauri Android shell config (`bundle.android`, mobile entry point, `cargo check --target aarch64-linux-android` clean); token in the app-private data directory (Google deprecated EncryptedSharedPreferences in 2024 — the sandbox is the recommended store; a Keystore-backed `android-native-keyring-store` can be added once the Gradle project exists); phone layout: top bar + four role-aware bottom tabs + "More" sheet, lists as cards with sort, scrolling dialogs, 44 px touch targets, phone-aware Setup screen; notifications via SSE + the notification plugin's Android channel; acceptance flow (Leasing login → unit → tenant response → follow-up) verified at 375×812. **Open (needs the Android SDK/NDK on a build machine):** `tauri android init`, cleartext manifest flag, keystore + signed APK/AAB, device testing on the org's phones, MDM/Play distribution (Q14). | Leasing user completes login → view unit → record tenant response → add follow-up on a phone | 2 wk |

**Total ≈ 14–15 weeks** for Windows (one developer), **+2 weeks** for Android. Critical path: Phase 0 → 2 → 4 → 5 → 6 → 8 → 9. Remaining work is the external/launch list above.

---

## 9. Testing & quality

- **Rust unit tests (`cargo test`, in `core` and `services`):** remaining-days/band calculation, contract and case state machines, response→status mapping, permission matrix, password hashing and token hashing, sweep planner (time-travel), handlebars placeholder checks, Excel row parsing/grouping, login rate limiter — 36 passing.
- **Integration tests (`#[sqlx::test]`, ephemeral test DB):** renewal-completion transaction, sweep idempotency and catch-up, unit status sync, report totals vs list counts, migrations from empty.
- **UI tests (Vitest + Testing Library):** band scale (same boundary table as Rust, 12 cases), URL normalisation — 13 passing; tables/filters/forms with a mocked API client are the next addition.
- **API scenarios (Python, against a running server):** `scenario*.py` in the session scratchpad exercise phases 1–8 end to end (master data → contracts → renewal completion → notices → sweep → reports → import/passwords/rate limit). To be ported to `#[sqlx::test]` integration tests.
- **E2E smoke (`tauri-driver` + WebdriverIO on Windows):** login → create contract → start renewal → send notice (mail provider stubbed) → complete renewal.
- **Seed data:** `cargo run -p db --bin seed` generates buildings, units, tenants and contracts with expiry dates spread across every band so dashboards look real from day one.
- **Security:** authorisation in the service layer on every call, validated inputs, argon2 passwords, hashed bearer tokens with revocation, token in the platform secure store (never in plain files), HTTPS between clients and server (Phase 8), Tauri 2 capabilities restricted to the plugins used, audit trail on all core entities, the server is the only holder of database and mail secrets.

---

## 10. Open questions for the client (answer before Phase 0)

| # | Question | Default if unanswered |
|---|---|---|
| Q1 | Local accounts managed by Admin, or Microsoft 365 sign-in (adds an OAuth flow to the desktop app)? | Local accounts in v1; SSO optional in Phase 8 |
| Q2 | Send from one shared leasing mailbox, or from each employee's own mailbox? | Shared mailbox (app-only Graph permission) |
| Q3 | What should "Delivered" mean? Graph only confirms acceptance; true delivery needs bounce monitoring | Sent = accepted by Graph; Delivered deferred |
| Q4 | Confirm the single band/colour scale in §3.1 and thresholds 90 / 30 | As proposed |
| Q5 | Auto-open a renewal case at 90 days, or always opened manually by the employee? | Auto-open as "Not Started" |
| Q6 | Renewal notice letter: letterhead, signature block, language(s) (English only? bilingual?) | English, org letterhead |
| Q7 | Existing data to import (Excel)? Contract numbering convention? | One-off Excel import in Phase 8; auto numbers `C-0001` |
| Q8 | Where does `renewal-server` run — office Windows Server (LAN + VPN for phones) or Azure (reachable from anywhere over HTTPS)? | Office server for Windows-only use; Azure once Android is live |
| Q9 | Period for the "Renewals Completed" dashboard card | Last 90 days, switchable |
| Q10 | Should Management receive a weekly email digest of expiries? | Not in v1 |
| Q11 | Should a contract ending on a Reserved / Under-Maintenance unit change its status? | Keep manual status |
| Q12 | Expected volume (buildings / units / contracts) and number of users / PCs | Hundreds of units, < 20 PCs |
| Q13 | Is a code-signing certificate available (avoids SmartScreen warnings on install/update)? | Buy one (~$200–400/yr) or accept the warning internally |
| Q14 | Android distribution: company MDM (Intune), Google Play (private/managed), or sideloaded APK? | Intune / managed Play |
| Q15 | Do phones need push notifications when the app is closed (adds Firebase Cloud Messaging), or are in-app notifications enough? | In-app only in v1 |

---

## 11. Risks

| Risk | Mitigation |
|---|---|
| Graph API app registration and admin consent can take days with IT | Request in Phase 0; SMTP fallback keeps Phase 5 unblocked |
| Server not running → no API, no reminders | Windows Service with automatic restart; heartbeat row + "Stale" badge in Settings; Admin "Run sweep now"; health endpoint for monitoring |
| Server exposed to the internet for phones | HTTPS only, bearer tokens with revocation, rate-limited login, no database port exposed; Azure front door / VPN per Q8 |
| Android WebView / background limits | Target recent WebView; keep the UI free of desktop-only APIs; notifications in-app unless FCM is added (Q15) |
| Unsigned installer triggers SmartScreen | Code-signing certificate (Q13) or internal rollout via Intune/GPO |
| Typst template work for letterhead-quality notices | Start the `.typ` template in Phase 5 week 1 with the client's letterhead; fall back to simpler `genpdf` layout if needed |
| Spec inconsistencies (bands, statuses) resurface during UAT | Sign off §3 before coding; thresholds are settings, not code |
| Duplicate or missed reminders | `reminder_dispatches` idempotency + "≤ days_before" catch-up + tightest-rule-only firing, all in a pure tested function |
| Bad end dates on imported contracts drive every alert | Import validation report; contracts flagged for review before activation |
| Scope creep toward payments/accounting | Business rule in the spec; rent field is display-only text |

---

## 12. Rust dependency shortlist

| Purpose | Crate(s) |
|---|---|
| Client shell | `tauri` 2, `tauri-plugin-opener`, `keyring` (Windows Credential Manager; Android keystore feature later), then `-notification`, `-updater`, `-window-state` |
| HTTP server | `axum` 0.8, `tower-http` (cors, trace), `tokio` |
| Database | `sqlx` 0.8 (postgres, runtime-tokio, tls-rustls-ring, chrono, uuid, json, migrate) |
| Domain | `serde`, `serde_json`, `chrono`, `uuid`, `thiserror`, `strum` (enum ↔ string) |
| Auth | `argon2`, `sha2` + `hex` (token hashes) |
| Templates / email | `handlebars`, `reqwest`, `oauth2` (client credentials), `lettre` (SMTP fallback) |
| PDF / Excel | `typst-as-lib` (+ `genpdf` fallback), `rust_xlsxwriter` |
| Documents | `object_store` (Azure / local FS; the Postgres bytea path is plain `sqlx`) |
| Scheduling / ops | `tokio-cron-scheduler`, `hostname`, `tracing`, `tracing-subscriber`, `dotenvy` |
| Tests | `#[sqlx::test]`, `insta` (snapshot PDFs/HTML), `tauri-driver` + WebdriverIO |
