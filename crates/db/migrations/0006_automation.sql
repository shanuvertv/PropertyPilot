-- Phase 6: reminder schedule, dispatch ledger, in-app notifications (spec §8, §15).

CREATE TABLE reminder_rules (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    days_before              INT NOT NULL CHECK (days_before >= 0),
    label                    TEXT NOT NULL,
    notify_in_app            BOOLEAN NOT NULL DEFAULT TRUE,
    email_assigned_employee  BOOLEAN NOT NULL DEFAULT FALSE,
    mark_urgent              BOOLEAN NOT NULL DEFAULT FALSE,
    active                   BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order               INT NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX reminder_rules_days_unique ON reminder_rules (days_before) WHERE active;

-- Spec §8 defaults. "90 days: show in Renewal Dashboard" is the expiring-soon threshold
-- (settings), so that milestone here is the in-app reminder that accompanies it.
INSERT INTO reminder_rules (days_before, label, notify_in_app, email_assigned_employee, mark_urgent, sort_order) VALUES
    (120, 'Internal renewal reminder',            TRUE,  FALSE, FALSE, 10),
    (90,  'Contract enters the Renewal Dashboard', TRUE,  FALSE, FALSE, 20),
    (60,  'Reminder to assigned employee',        TRUE,  TRUE,  FALSE, 30),
    (30,  'Mark as Urgent Renewal',               TRUE,  TRUE,  TRUE,  40),
    (15,  'Urgent reminder',                      TRUE,  TRUE,  TRUE,  50),
    (7,   'Final internal reminder',              TRUE,  TRUE,  TRUE,  60);

-- One row per (contract, rule): a reminder fires once, ever (PLAN.md §11 "duplicate or missed reminders").
CREATE TABLE reminder_dispatches (
    contract_id    UUID NOT NULL REFERENCES contracts (id) ON DELETE CASCADE,
    rule_id        UUID NOT NULL REFERENCES reminder_rules (id) ON DELETE CASCADE,
    skipped        BOOLEAN NOT NULL DEFAULT FALSE,
    dispatched_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (contract_id, rule_id)
);

CREATE TABLE notifications (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN (
                     'CONTRACT_EXPIRING_SOON', 'RENEWAL_NOTICE_PENDING', 'TENANT_RESPONSE_PENDING',
                     'FOLLOW_UP_DUE_TODAY', 'OVERDUE_FOLLOW_UP', 'CONTRACT_EXPIRED', 'RENEWAL_REMINDER',
                     'RENEWAL_COMPLETED', 'CASE_ASSIGNED')),
    title        TEXT NOT NULL,
    body         TEXT,
    entity_type  TEXT NOT NULL,   -- tenant | unit | contract | renewal_case
    entity_id    UUID NOT NULL,
    dedupe_key   TEXT,            -- e.g. 'follow-up-due:<id>:2026-09-11' — one per user per day
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    read_at      TIMESTAMPTZ
);
CREATE INDEX notifications_user_idx ON notifications (user_id, read_at, created_at DESC);
CREATE UNIQUE INDEX notifications_dedupe_unique ON notifications (user_id, dedupe_key) WHERE dedupe_key IS NOT NULL;

INSERT INTO settings (key, value) VALUES
    ('renewals.autoOpenCase', 'true'),
    ('dashboard.completedWindowDays', '90');
