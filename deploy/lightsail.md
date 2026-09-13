# Hosting PropertyPilot on AWS Lightsail

Target layout: a **Lightsail managed PostgreSQL** database plus one small **Lightsail Linux
instance** running `renewal-server` behind HTTPS. Windows desktops and Android phones connect
to the instance's public HTTPS address; nothing connects to the database directly.

```
Windows app ─┐                                   ┌─ Lightsail database (PostgreSQL 16)
Android app ─┼─ https://renewals.example.com ─▶ renewal-server (Lightsail instance) ─┘
Browser dev ─┘        (Caddy: TLS + reverse proxy)      scheduler · email queue · SSE
```

## 1. Database

1. Lightsail → Databases → *Create database* → PostgreSQL 16, same region as the instance.
   The smallest plan (1 GB RAM) is enough for this workload.
2. After creation note the **endpoint**, **port** (5432), master user name and password.
3. Keep *Public mode* **off**; the instance in the same account/region reaches the private
   endpoint. (Turn it on only temporarily if you need to run `psql`/`pg_dump` from outside.)
4. Create the application role and database (one-off, from the instance):
   ```bash
   sudo apt-get install -y postgresql-client
   psql "postgresql://dbmasteruser:MASTERPASSWORD@<endpoint>:5432/postgres?sslmode=require" \
     -c "CREATE ROLE renewal LOGIN PASSWORD 'CHANGE-ME';" \
     -c "CREATE DATABASE renewal OWNER renewal;"
   ```
5. Backups: Lightsail databases take automatic daily snapshots (7-day retention by default);
   enable point-in-time restore in the database's *Snapshots & restore* tab.

### 1b. Alternative: PostgreSQL on the same instance (no managed database)

For a small team the database can simply live on the instance — about half the monthly cost,
at the price of doing backups yourself. Skip section 1 and, after creating the instance (2):

```bash
sudo apt-get install -y postgresql
sudo -u postgres psql -c "CREATE ROLE renewal LOGIN PASSWORD 'CHANGE-ME';"                        -c "CREATE DATABASE renewal OWNER renewal;"
# nightly dump, kept 14 days (restore command is inside the script)
sudo cp ~/pp/backup-db.sh /usr/local/bin/ && sudo chmod 755 /usr/local/bin/backup-db.sh
echo '15 1 * * * root /usr/local/bin/backup-db.sh' | sudo tee /etc/cron.d/propertypilot-backup
```

Use `DATABASE_URL=postgresql://renewal:CHANGE-ME@localhost:5432/renewal` in `.env` (no `sslmode`
needed on localhost), and enable the instance's automatic snapshots in Lightsail as a second
safety net. Moving to the managed database later is a `pg_dump` + `pg_restore` and one `.env` line.

## 2. Instance

1. Lightsail → Instances → *Create instance* → Linux, **Ubuntu 24.04**, 1 GB plan (2 GB if you
   also run Caddy and expect many PDFs). Attach a **static IP**.
2. Networking tab → firewall: allow **HTTPS 443** and **HTTP 80** (for the Let's Encrypt
   challenge); keep 8787 closed — Caddy proxies to it locally.
3. DNS: point `renewals.example.com` (an A record) at the static IP.

## 3. Install the server

GitHub Actions builds the Linux binary on every push to `main` (workflow *Release server binary*;
download `renewal-server-linux-x86_64` from the run's artifacts) and attaches it to a GitHub
Release when a `v*` tag is pushed. On the instance (Ubuntu), fetch the release tarball:

```bash
curl -fsSLo renewal-server.tar.gz   https://github.com/shanuvertv/PropertyPilot/releases/latest/download/renewal-server-linux-x86_64.tar.gz
mkdir -p ~/pp && tar -C ~/pp -xzf renewal-server.tar.gz && ls ~/pp
```

(Or build it yourself on any Linux x86_64 machine with `cargo build --release -p renewal-server`
and `scp` the binary plus the two files from `deploy/`.)

Then install it:

```bash
sudo useradd --system --home /opt/propertypilot --create-home propertypilot
sudo mkdir -p /opt/propertypilot/logs
sudo mv ~/pp/renewal-server /opt/propertypilot/ && sudo chmod 755 /opt/propertypilot/renewal-server
sudo rm -rf /opt/propertypilot/web && sudo mv ~/pp/web /opt/propertypilot/web   # the browser version of the app
sudo tee /opt/propertypilot/.env >/dev/null <<'ENV'
DATABASE_URL=postgresql://renewal:CHANGE-ME@<db-endpoint>:5432/renewal?sslmode=require
BIND_ADDR=127.0.0.1:8787
ORG_TIMEZONE=Asia/Dubai
SCHEDULER=on
RUST_LOG=info,sqlx=warn
LOG_DIR=/opt/propertypilot/logs
MAIL_PROVIDER=smtp
MAIL_FROM_NAME=Leasing Department
MAIL_FROM_ADDRESS=leasing@example.com
SMTP_HOST=smtp.office365.com
SMTP_PORT=587
SMTP_USERNAME=leasing@example.com
SMTP_PASSWORD=app-password
IMAP_HOST=outlook.office365.com
ENV
sudo chown -R propertypilot:propertypilot /opt/propertypilot && sudo chmod 600 /opt/propertypilot/.env
sudo cp ~/pp/renewal-server.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now renewal-server
curl -s http://127.0.0.1:8787/api/health      # {"ok":true,...}
```

Alternatively run the container image built from the repository `Dockerfile`
(`docker run -d --restart unless-stopped --env-file .env -p 127.0.0.1:8787:8787 propertypilot-server`);
Lightsail *Container services* can also host it if you prefer not to manage an instance — then
attach the database's private endpoint and let Lightsail terminate HTTPS.

## 4. HTTPS with Caddy (automatic certificates)

No domain yet? Use the free `sslip.io` name for the static IP — e.g. `13-207-218-150.sslip.io`
for `13.207.218.150` — as the host in the Caddyfile. Let's Encrypt issues a normal certificate
for it, and the apps use `https://13-207-218-150.sslip.io`. When you buy a domain, point an
A record at the IP, change the host in the Caddyfile and `systemctl reload caddy`; the apps
just get the new address on their Setup screen.

Remember the Lightsail firewall (instance → Networking): it allows only SSH and HTTP by
default — add **HTTPS (443)** or nothing outside the instance can connect.

```bash
sudo apt-get install -y caddy
sudo cp ~/pp/Caddyfile /etc/caddy/Caddyfile  # edit the host name first
sudo systemctl reload caddy
curl -s https://renewals.example.com/api/health
```

Caddy obtains and renews the Let's Encrypt certificate itself. (The server can also serve
HTTPS directly with `TLS_CERT`/`TLS_KEY` if you already have certificate files — then open
443 to the server and skip Caddy.)

## 5. Point the apps at it

`https://<host>/` also opens the app in any browser (the release tarball ships the web build
in `web/`, served by the server itself), so people without the desktop app can sign in from
a browser; the Windows and Android apps keep working the same way.

Windows app and Android app → Setup screen → `https://renewals.example.com`. Create the first
Admin from the desktop app; users are added under Settings.

## 6. Operations

- **Upgrade:** `scp` the new binary, `sudo systemctl restart renewal-server` (migrations run on
  start). Container: rebuild the image and restart.
- **Logs:** `/opt/propertypilot/logs/renewal-server.log.<date>` and `journalctl -u renewal-server`.
- **Health:** `/api/health` — wire it to Lightsail's metrics/alarms or an uptime checker.
- **Database restore:** Lightsail → database → *Snapshots & restore*; then update
  `DATABASE_URL` if the endpoint changed.
- **Time zone:** the sweep runs at 00:05 `ORG_TIMEZONE`, independent of the instance's clock zone.
