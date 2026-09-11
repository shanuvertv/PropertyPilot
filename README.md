# PropertyPilot

Rental contract renewal management for a leasing department — a Windows desktop app
(Tauri 2 + Rust + React) talking to a small Rust API server that owns PostgreSQL and runs
the daily expiry sweep, reminders and email queue. An Android build on the same codebase
is the next phase. Not an accounting system.

See [PLAN.md](PLAN.md) for the full implementation plan and phase status.

## What it does

- Buildings, units and tenants with documents; unit-wise summary with expiry colours
- Contracts (multi-unit) with automatic expiry bands (0–30 / 31–60 / 61–90 / 91–120 / beyond)
- Renewal cases: 11-status workflow, tenant responses, follow-ups, checklist, completion that
  creates the linked renewal contract and keeps the timeline
- Renewal notices: auto-drafted letter → PDF (letterhead) → email with attachment, tracked
- Email templates with placeholders, queue with retries, SMTP or Microsoft Graph
- Automation: daily sweep marks expired contracts, opens cases, fires reminder rules once
  each; notification centre with live updates and native toasts
- Reports (5) with Excel/PDF export; full audit trail with per-record history
- Roles: Admin, Leasing Team, Operations, Management (server-side permission matrix)
- Excel import of an existing tenant list (preview, then commit; safe to re-run)

## Layout

```
crates/core       pure domain: roles, statuses, state machines, expiry engine (no IO)
crates/api        request/response types shared by server and clients
crates/db         PostgreSQL access, embedded migrations, audit writer
crates/services   use-cases: auth, sessions, users, providers (mail, storage)
apps/server       renewal-server: Axum HTTP API + in-process scheduler (owns the DB)
apps/desktop      Tauri 2 shell + React UI (src/), talks to the server over HTTP
```

## Prerequisites

- Rust stable (MSVC toolchain on Windows) and Visual Studio C++ Build Tools
- Node.js 24 + npm
- PostgreSQL 16 — either Docker (`docker compose up -d`) or a local/remote instance
- Windows 10/11 with the Edge WebView2 runtime (bundled by the installer)

## Run locally

1. Database — pick one:
   - Docker: `docker compose up -d` (user/password/db all `renewal`, port 5432)
   - Existing PostgreSQL: create a database and note its connection string
2. Server:
   ```bash
   cp .env.example .env        # edit DATABASE_URL if needed
   cargo run -p renewal-server # applies migrations, listens on http://127.0.0.1:8787
   ```
3. Desktop app (dev, hot reload):
   ```bash
   cd apps/desktop
   npm install
   npm run tauri dev
   ```
   On first launch enter the server address (`http://localhost:8787`), then create the
   first administrator. Admins add other users under Settings.

UI-only work without Tauri: `npm run dev` in `apps/desktop` and open http://localhost:1420
(the secure store falls back to localStorage in a plain browser).

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace            # core/services unit tests need no database
cd apps/desktop && npm run typecheck && npm test && npm run build
```

## Build the Windows installer

```bash
cd apps/desktop && npm run tauri build
```
Outputs MSI and NSIS installers under `target/release/bundle/`.

## Server configuration (environment)

Read from the environment, a `.env` in the working directory, or a `.env` next to
`renewal-server.exe` (service installs). Full example: [.env.example](.env.example).

| Variable            | Default              | Purpose                                                    |
|---------------------|----------------------|------------------------------------------------------------|
| `DATABASE_URL`      | —                    | PostgreSQL connection string (required)                    |
| `BIND_ADDR`         | `127.0.0.1:8787`     | Listen address; use `0.0.0.0:8787` for LAN clients         |
| `ORG_TIMEZONE`      | `Asia/Dubai`         | "Today" for expiry bands and the 00:05 daily sweep         |
| `SCHEDULER`         | `on`                 | Run the heartbeat / daily sweep / email sender here        |
| `RUST_LOG`          | `info,sqlx=warn`     | Log filter                                                 |
| `LOG_DIR`           | —                    | When set, write daily rolling log files there (services)   |
| `TLS_CERT`/`TLS_KEY`| —                    | PEM pair; when both are set the server serves HTTPS itself |
| `MAIL_PROVIDER`     | `log`                | `log` (print only), `smtp` or `graph`                      |
| `MAIL_FROM_NAME`/`MAIL_FROM_ADDRESS` | —   | Sender shown on outgoing mail                              |
| `SMTP_HOST/PORT/USERNAME/PASSWORD/STARTTLS` | — | SMTP provider settings                              |
| `GRAPH_TENANT_ID/CLIENT_ID/CLIENT_SECRET/SENDER` | — | Microsoft Graph app (client credentials, `Mail.Send`) |

## Deploy the server as a Windows Service

On the server machine (PostgreSQL reachable, elevated PowerShell):

```powershell
cargo build --release -p renewal-server
.\installers\install-server.ps1 -DatabaseUrl "postgres://renewal:SECRET@localhost:5432/renewal" -BindAddr "0.0.0.0:8787"
```

This copies the binary to `C:\ProgramData\PropertyPilot\server`, writes a `.env` there
(kept on upgrades — edit it to configure email and TLS), registers the
`PropertyPilotServer` service (auto-start, restart on failure), opens the firewall port and
starts it. Logs go to `...\server\logs\renewal-server.log.<date>`. Re-run the script to
upgrade; `installers\uninstall-server.ps1` removes the service (the database is never touched).

Point the desktop app at `http://<server>:8787` (or `https://` when TLS is configured) on
its Setup screen. Alternatives: Docker (`docker-compose.yml`) or any host that can run
the static binary — the server has no other dependencies.

## Operations runbook

- **Backups:** `pg_dump renewal` nightly; documents and generated PDFs live in the
  database (`document_blobs`, `email_attachments`) so one dump is a full backup.
- **Health:** `GET /api/health` returns `{ok, database, version}`; Settings → Automation
  shows the scheduler heartbeat (stale after 5 minutes) and the last sweep.
- **Sweep on demand:** Settings → Automation → *Run sweep now* (Admin).
- **Email failures:** Email Communication page lists failed messages with the provider
  error; *Retry* re-queues them.
- **Locked-out user:** an Admin resets the password in Settings → Users; the user is
  signed out everywhere and sets a new one via the key icon next to their name.
- **Sign-in throttling:** 10 failed attempts per client/mailbox in 15 minutes returns
  `429` — no lockout, it clears on its own.

## Release signing and updates (not done here)

- **Code signing:** buy an EV or OV certificate, then set `bundle.windows.certificateThumbprint`
  and `timestampUrl` in `apps/desktop/src-tauri/tauri.conf.json`; `npm run tauri build` signs
  the MSI/NSIS output. Unsigned builds work but SmartScreen warns on first launch.
- **Auto-update:** add `tauri-plugin-updater`, generate a key pair with
  `npm run tauri signer generate`, host `latest.json` + the signed installer on an internal
  URL, and set `plugins.updater.endpoints` + `pubkey` in `tauri.conf.json`. Until then,
  distribute the installer from `target/release/bundle/` directly.

## API

All routes live under `/api`, JSON bodies in camelCase, bearer tokens from `/api/auth/login`.

| Area | Routes |
|------|--------|
| Auth | `health`, `auth/bootstrap`, `auth/login`, `auth/logout`, `auth/me`, `auth/password` |
| Users | `users`, `users/{id}/active`, `users/{id}/password`, `employees` |
| Master data | `buildings`, `buildings/options`, `units`, `units/{id}/status`, `tenants`, `tenants/options`, `documents`, `documents/{id}/download` |
| Contracts | `contracts`, `contracts/suggest-number`, `contracts/{id}/activate|terminate|assign|renewal`, `contracts/expire-overdue` |
| Renewals | `renewals`, `renewals/checklist-template`, `renewals/{id}/status|assign|notes|responses|checklist/{item}|complete|follow-ups|notice|notice/pdf|notice/send`, `follow-ups`, `follow-ups/counts` |
| Email | `email-templates`, `email-templates/placeholders`, `email-templates/{key}/preview`, `emails`, `emails/{id}/retry`, `system/mail` |
| Automation | `notifications`, `notifications/count`, `notifications/read-all`, `events` (SSE), `settings/org`, `settings/reminder-rules`, `system/sweep`, `system/status` |
| Reports & audit | `reports/{kind}` (`?format=xlsx|pdf`), `audit`, `audit/{entityType}/{id}` |
| Import | `import/preview`, `import/commit` (multipart `file` = .xlsx) |
| Dashboard | `dashboard`, `search` |

Errors are always `{ "error": { "code": "...", "message": "..." } }`.
