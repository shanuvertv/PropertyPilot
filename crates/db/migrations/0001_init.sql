-- Phase 0 schema: users, settings, audit trail, worker heartbeat.
-- Domain tables (buildings, units, tenants, contracts, renewals, ...) arrive in Phases 1–6.
-- Enum-like columns are TEXT constrained here and mirrored by `renewal_core` enums
-- (SCREAMING_SNAKE_CASE), so Rust stays the single source of truth for allowed values.

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    email           TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('ADMIN', 'LEASING', 'OPERATIONS', 'MANAGEMENT')),
    password_hash   TEXT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at   TIMESTAMPTZ
);
CREATE UNIQUE INDEX users_email_unique ON users (lower(email));

-- Key/value settings: org profile, timezone, thresholds, sending mailbox, letterhead.
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       JSONB NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by  UUID REFERENCES users (id)
);

INSERT INTO settings (key, value) VALUES
    ('org.name',                 '"Leasing Department"'),
    ('org.timezone',             '"Asia/Dubai"'),
    ('expiry.thresholds',        '{"expiringSoonDays": 90, "urgentDays": 30}');

-- Spec §19: user, date/time, action, previous value, new value.
CREATE TABLE audit_logs (
    id           BIGSERIAL PRIMARY KEY,
    actor_id     UUID REFERENCES users (id),
    entity_type  TEXT NOT NULL,
    entity_id    UUID,
    action       TEXT NOT NULL,
    before_json  JSONB,
    after_json   JSONB,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX audit_logs_entity_idx ON audit_logs (entity_type, entity_id, created_at DESC);
CREATE INDEX audit_logs_actor_idx ON audit_logs (actor_id, created_at DESC);

-- Single-row table the worker service updates; the desktop app shows it in Settings
-- and warns when the last heartbeat is stale (PLAN.md §5, §11).
CREATE TABLE worker_status (
    id                 BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    version            TEXT NOT NULL,
    hostname           TEXT NOT NULL,
    last_heartbeat_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_sweep_at      TIMESTAMPTZ,
    last_sweep_summary JSONB
);
