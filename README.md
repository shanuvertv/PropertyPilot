# PropertyPilot

Rental contract renewal management for a leasing department — a Windows desktop app and an
Android app (Tauri 2 + Rust, one shared React UI) talking to a small Rust API server that
owns PostgreSQL and runs the daily expiry sweep, reminders and email queue. Not an
accounting system.

See [PLAN.md](PLAN.md) for the full implementation plan and phase status.

## What it does

- Buildings, units and tenants with documents; unit-wise summary with expiry colours
- Contracts (multi-unit) with rent amount, payment terms, number of tenants per unit and
  automatic expiry bands (0–30 / 31–60 / 61–90 / 91–120 / beyond)
- Renewal cases: 11-status workflow, tenant responses, follow-ups, checklist, completion that
  creates the linked renewal contract and keeps the timeline
- Renewal notices: auto-drafted letter → PDF (letterhead) → email with attachment, tracked
- Email templates with placeholders, queue with retries, SMTP or Microsoft Graph
- Automation: daily sweep marks expired contracts, opens cases, fires reminder rules once
  each; notification centre with live updates and native toasts
- Reports (5) with Excel/PDF export; full audit trail with per-record history
- Roles: Admin, Leasing Team, Operations, Management (server-side permission matrix; Management view expenses)
- Excel import of an existing tenant list — its own **Import data** page: buildings, units,
  tenants, contracts with dates, number of tenants (capacity) and rent per annum (preview,
  then commit; re-running fills in rent / tenants on contracts imported earlier, never duplicates)
- Android app: the same screens with bottom tabs and card lists; token kept in the app's
  private storage; works over the LAN or the internet (HTTPS)
- Browser version: the server serves the same UI at `https://<server>/` (set `WEB_DIR` or put
  the `web/` build next to the executable) — no install needed
- Search and filters on every list: `Ctrl+K` global search (buildings, units, tenants,
  contracts, expenses), a search box plus building / status / type / date filters on
  each page, and a quick filter on every detail-page table
- Table or card layout on every list (the toggle next to the filters, remembered per list):
  cards show the key figures at a glance — unit counts, contract end and days left, amounts,
  status badges — with the row actions underneath
- Number of tenants per unit, entered on the contract (per unit it covers — no per-person
  register); a unit shows the number from its active contract
- Expenses per unit: bills and costs by category, split equally between that many tenants
  with a "paid so far" count, attached bills; dashboard with monthly trend and totals per
  property, per unit and per category, plus what is still outstanding from the tenants

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

## Android app

The Android build is the same React UI in a Tauri 2 Android shell (`apps/desktop`), so every
screen and permission rule is shared. Below 768 px the shell switches to a top bar, four bottom
tabs (the role's most-used modules) and a "More" sheet with the full module list; lists render
as cards; dialogs scroll; touch targets grow to 44 px.

Prerequisites (once, on the build machine): [Android Studio](https://developer.android.com/studio)
with the SDK Platform 34+, *NDK (Side by side)* and *Android SDK Command-line Tools*, a JDK 17,
and the environment variables `ANDROID_HOME` (e.g. `%LOCALAPPDATA%\Android\Sdk`) and
`NDK_HOME` (`%ANDROID_HOME%\ndk\<version>`). Then:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
cd apps/desktop
npm run tauri android init            # generates src-tauri/gen/android (commit it)
```

After `init`, allow plain-HTTP servers on the office LAN by adding
`android:usesCleartextTraffic="true"` to the `<application>` element in
`src-tauri/gen/android/app/src/main/AndroidManifest.xml` (not needed if the server is only
reached over HTTPS). Then:

```bash
npm run tauri android dev                       # runs on the connected phone / emulator with hot reload
npm run tauri android build -- --apk --target aarch64   # release APK (arm64) under src-tauri/gen/android/app/build/outputs/apk
npm run tauri android build -- --aab            # Play Store bundle, all ABIs
```

`src-tauri/gen/android` is committed (generated once by `tauri android init`); `app/build.gradle.kts`
carries the release signing config and allows plain `http://` for LAN servers. Release builds are
signed with `gen/android/propertypilot-release.jks` via `gen/android/keystore.properties`
(`storeFile`, `password`, `keyAlias`) — both git-ignored. **Back the keystore and its password up:**
phones only accept updates signed with the same key. To create a new one:
`keytool -genkeypair -keystore propertypilot-release.jks -storetype PKCS12 -keyalg RSA -keysize 2048 -validity 10000 -alias propertypilot`.

On Windows the Tauri CLI links the built library into the Gradle project with a symlink, which
needs *Settings → System → For developers → Developer Mode* (or an elevated shell). Without it the
Rust step succeeds and the link fails; finish the build by hand:

```bash
mkdir -p src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a
cp ../../target/aarch64-linux-android/release/librenewal_desktop_lib.so src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/
cd src-tauri/gen/android && ./gradlew assembleArm64Release -x rustBuildArm64Release
```

`MainActivity.kt` pads the web view by the system-bar and keyboard insets: Android 15+ forces
edge-to-edge for targetSdk 35+ and WebView does not pass those insets to CSS, so without it the
bottom tabs sit under the gesture bar.

Emulator (optional): `sdkmanager "system-images;android-36;google_apis;x86_64"`, create a device with
`avdmanager create avd -n PropertyPilotTest -k "system-images;android-36;google_apis;x86_64" -d pixel_7`,
start it, `adb install -r` the APK (the Google APIs image runs arm64 APKs through translation) and use
`http://10.0.2.2:8787` as the server address — that is the host PC seen from the emulator.

Distribute the APK through the organisation's MDM or as a direct download (phones need
"install from unknown sources" for the first install); the Play Store needs the AAB. Android Studio
itself is optional — the SDK command-line tools, a JDK 17 and the NDK are enough.

On the phone, the Setup screen asks for the server address: the public HTTPS address
(see [deploy/lightsail.md](deploy/lightsail.md)) or `http://<server-ip>:8787` on the office
Wi-Fi. The session token is stored in the app's private data directory (Android sandboxes it
per app); notifications use the Android notification channel via the Tauri notification plugin.

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
| `WEB_DIR`           | `web/` beside the exe | Built web UI (`apps/desktop/dist`) to serve at `/`; omit to serve a landing page only |
| `MAIL_PROVIDER`     | `log`                | `log` (print only), `smtp` or `graph`                      |
| `MAIL_FROM_NAME`/`MAIL_FROM_ADDRESS` | —   | Sender shown on outgoing mail                              |
| `SMTP_HOST/PORT/USERNAME/PASSWORD/STARTTLS` | — | SMTP provider settings (any mailbox: Microsoft 365, Google Workspace, cPanel…) |
| `IMAP_HOST/PORT/USERNAME/PASSWORD/SENT_FOLDER` | — | Optional: also file each sent message in that mailbox's Sent folder over IMAP |
| `GRAPH_TENANT_ID/CLIENT_ID/CLIENT_SECRET/SENDER` | — | Microsoft Graph app (client credentials, `Mail.Send`) |

## Deploy the server on AWS Lightsail (planned hosting)

The intended production layout is a Lightsail managed PostgreSQL database plus a small
Lightsail Linux instance running `renewal-server` behind Caddy (automatic HTTPS). Both the
Windows and Android apps then use `https://<your-host>` on the Setup screen. Step by step:
[deploy/lightsail.md](deploy/lightsail.md); supporting files: [deploy/renewal-server.service](deploy/renewal-server.service),
[deploy/Caddyfile](deploy/Caddyfile), and the container image in [Dockerfile](Dockerfile).

## Deploy the server as a Windows Service (on-premises alternative)

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

### Sending from your own mailbox

**In the app:** Settings → *Email sending* (Admin). Choose *SMTP*, enter the mailbox's details —
the same account you read over IMAP or in Outlook (Microsoft 365: `smtp.office365.com`, 587,
STARTTLS, SMTP AUTH enabled on the mailbox; Google Workspace: `smtp.gmail.com`, 587, an app
password; cPanel/Zoho hosts: `mail.yourdomain.com`, 465 SSL or 587 STARTTLS) — tick *file sent
email in the Sent folder* if you want Outlook and phones to show what PropertyPilot sent, save,
and press *Send test*. The settings live in the database (password included, never returned to
clients), so every desktop and phone uses the same mailbox and nothing on the server changes.

The `MAIL_*`/`SMTP_*`/`IMAP_*` variables in `.env` are only the fallback used until something
is saved in the app (the *log* provider prints messages instead of sending — handy for a first
run). Microsoft Graph remains available through `.env` for tenants that disable SMTP AUTH.

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
| Users | `users`, `users/{id}` (delete), `users/{id}/active`, `users/{id}/password`, `employees` |
| Master data | `buildings`, `buildings/options`, `units`, `units/{id}/status`, `tenants`, `tenants/options`, `documents`, `documents/{id}/download` |
| Contracts | `contracts`, `contracts/suggest-number`, `contracts/{id}/activate|terminate|assign|renewal`, `contracts/expire-overdue` |
| Renewals | `renewals`, `renewals/checklist-template`, `renewals/{id}/status|assign|notes|responses|checklist/{item}|complete|follow-ups|notice|notice/pdf|notice/send`, `follow-ups`, `follow-ups/counts` |
| Email | `email-templates`, `email-templates/placeholders`, `email-templates/{key}/preview`, `emails`, `emails/{id}/retry`, `system/mail` |
| Automation | `notifications`, `notifications/count`, `notifications/read-all`, `events` (SSE), `settings/org`, `settings/reminder-rules`, `settings/mail`, `settings/mail/test`, `system/sweep`, `system/status` |
| Reports & audit | `reports/{kind}` (`?format=xlsx|pdf`), `audit`, `audit/{entityType}/{id}` |
| Import | `import/preview`, `import/commit` (multipart `file` = .xlsx) |
| Expenses | `expenses`, `expenses/summary`, `expenses/{id}`, `expenses/{id}/split`, `expenses/{id}/settled` (the number of tenants travels with the contract's `unitTenants`) |
| Dashboard | `dashboard`, `search` |

Errors are always `{ "error": { "code": "...", "message": "..." } }`.
