-- Phases 2–4: contracts, renewal cases, tenant responses, follow-ups, checklist.
-- Enum-like columns mirror `renewal_core` enums (SCREAMING_SNAKE_CASE).

-- ---------------------------------------------------------------- contracts (spec §5, §14)
CREATE TABLE contracts (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_number       TEXT NOT NULL,
    tenant_id             UUID NOT NULL REFERENCES tenants (id),
    building_id           UUID NOT NULL REFERENCES buildings (id),
    start_date            DATE NOT NULL,
    end_date              DATE NOT NULL,
    rent_terms            TEXT,                       -- optional, display only (not an accounting system)
    status                TEXT NOT NULL DEFAULT 'DRAFT'
                          CHECK (status IN ('DRAFT', 'ACTIVE', 'RENEWED', 'EXPIRED', 'TERMINATED')),
    assigned_employee_id  UUID REFERENCES users (id),
    previous_contract_id  UUID REFERENCES contracts (id),
    root_contract_id      UUID REFERENCES contracts (id),
    renewal_sequence      INT NOT NULL DEFAULT 0,      -- 0 = original, 1 = Renewal 1, ...
    notes                 TEXT,
    activated_at          TIMESTAMPTZ,
    ended_at              TIMESTAMPTZ,                -- when it became RENEWED / EXPIRED / TERMINATED
    created_by            UUID REFERENCES users (id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (end_date >= start_date)
);
CREATE UNIQUE INDEX contracts_number_unique ON contracts (lower(contract_number));
CREATE INDEX contracts_tenant_idx ON contracts (tenant_id);
CREATE INDEX contracts_building_idx ON contracts (building_id);
CREATE INDEX contracts_status_end_idx ON contracts (status, end_date);
CREATE INDEX contracts_root_idx ON contracts (root_contract_id);
CREATE INDEX contracts_number_trgm ON contracts USING gin (contract_number gin_trgm_ops);

CREATE TABLE contract_units (
    contract_id  UUID NOT NULL REFERENCES contracts (id) ON DELETE CASCADE,
    unit_id      UUID NOT NULL REFERENCES units (id),
    PRIMARY KEY (contract_id, unit_id)
);
CREATE INDEX contract_units_unit_idx ON contract_units (unit_id);

-- ---------------------------------------------------------------- renewal cases (spec §6, §7, §11)
CREATE TABLE renewal_cases (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id           UUID NOT NULL REFERENCES contracts (id),
    status                TEXT NOT NULL DEFAULT 'NOT_STARTED' CHECK (status IN (
                              'NOT_STARTED', 'NOTICE_PENDING', 'NOTICE_SENT', 'WAITING_FOR_TENANT_RESPONSE',
                              'TENANT_INTERESTED', 'UNDER_NEGOTIATION', 'RENEWAL_CONFIRMED', 'RENEWAL_COMPLETED',
                              'TENANT_NOT_RENEWING', 'VACATING', 'CLOSED')),
    notice_status         TEXT NOT NULL DEFAULT 'PENDING'
                          CHECK (notice_status IN ('NOT_REQUIRED', 'PENDING', 'DRAFT', 'SENT', 'DELIVERED', 'FAILED')),
    assigned_employee_id  UUID REFERENCES users (id),
    is_urgent             BOOLEAN NOT NULL DEFAULT FALSE,
    latest_response       TEXT CHECK (latest_response IS NULL OR latest_response IN (
                              'NO_RESPONSE', 'INTERESTED_IN_RENEWAL', 'RENEWAL_CONFIRMED',
                              'UNDER_DISCUSSION', 'NOT_INTERESTED', 'WILL_VACATE')),
    latest_response_at    TIMESTAMPTZ,
    next_follow_up_date   DATE,
    outcome_contract_id   UUID REFERENCES contracts (id),   -- the new contract once completed
    notes                 TEXT,
    opened_by             UUID REFERENCES users (id),
    opened_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    closed_at             TIMESTAMPTZ,
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- One open case per contract.
CREATE UNIQUE INDEX renewal_cases_open_unique ON renewal_cases (contract_id)
    WHERE status NOT IN ('RENEWAL_COMPLETED', 'CLOSED');
CREATE INDEX renewal_cases_status_idx ON renewal_cases (status);

CREATE TABLE renewal_responses (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id        UUID NOT NULL REFERENCES renewal_cases (id) ON DELETE CASCADE,
    response       TEXT NOT NULL CHECK (response IN (
                       'NO_RESPONSE', 'INTERESTED_IN_RENEWAL', 'RENEWAL_CONFIRMED',
                       'UNDER_DISCUSSION', 'NOT_INTERESTED', 'WILL_VACATE')),
    response_date  DATE NOT NULL,
    notes          TEXT,
    follow_up_date DATE,
    recorded_by    UUID REFERENCES users (id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX renewal_responses_case_idx ON renewal_responses (case_id, created_at DESC);

-- ---------------------------------------------------------------- follow-ups (spec §12)
CREATE TABLE follow_ups (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id               UUID NOT NULL REFERENCES renewal_cases (id) ON DELETE CASCADE,
    due_date              DATE NOT NULL,
    follow_up_type        TEXT NOT NULL
                          CHECK (follow_up_type IN ('PHONE_CALL', 'EMAIL', 'MEETING', 'WHATS_APP', 'INTERNAL_DISCUSSION')),
    assigned_employee_id  UUID REFERENCES users (id),
    notes                 TEXT,
    status                TEXT NOT NULL DEFAULT 'OPEN' CHECK (status IN ('OPEN', 'DONE', 'CANCELLED')),
    completed_at          TIMESTAMPTZ,
    completed_by          UUID REFERENCES users (id),
    created_by            UUID REFERENCES users (id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX follow_ups_due_idx ON follow_ups (status, due_date);
CREATE INDEX follow_ups_case_idx ON follow_ups (case_id);

-- ---------------------------------------------------------------- checklist (spec §13)
-- `key` lets the system tick items automatically; Admin-added items have no key.
CREATE TABLE checklist_template_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key         TEXT UNIQUE,
    label       TEXT NOT NULL,
    sort_order  INT NOT NULL,
    required    BOOLEAN NOT NULL DEFAULT TRUE,
    active      BOOLEAN NOT NULL DEFAULT TRUE
);
INSERT INTO checklist_template_items (key, label, sort_order, required) VALUES
    ('NOTICE_SENT',        'Renewal Notice Sent',        10, FALSE),
    ('RESPONSE_RECEIVED',  'Tenant Response Received',   20, TRUE),
    ('TERMS_FINALIZED',    'Renewal Terms Finalized',    30, TRUE),
    ('CONFIRMED',          'Renewal Confirmed',          40, TRUE),
    ('NEW_CONTRACT',       'New Contract Created',       50, FALSE),
    ('DOCUMENT_UPLOADED',  'Contract Document Uploaded', 60, FALSE),
    ('COMPLETED',          'Renewal Completed',          70, FALSE);

CREATE TABLE renewal_checklist_items (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id     UUID NOT NULL REFERENCES renewal_cases (id) ON DELETE CASCADE,
    key         TEXT,
    label       TEXT NOT NULL,
    sort_order  INT NOT NULL,
    required    BOOLEAN NOT NULL,
    done        BOOLEAN NOT NULL DEFAULT FALSE,
    done_by     UUID REFERENCES users (id),
    done_at     TIMESTAMPTZ
);
CREATE INDEX renewal_checklist_case_idx ON renewal_checklist_items (case_id, sort_order);

-- ---------------------------------------------------------------- derived expiry (PLAN.md §3.1, §3.2)
-- CURRENT_DATE is evaluated in the connection's time zone, which the server sets
-- to the organisation's zone (ORG_TIMEZONE). Thresholds come from settings.
CREATE VIEW v_contract_expiry AS
WITH t AS (
    SELECT COALESCE((value ->> 'expiringSoonDays')::int, 90) AS expiring_soon_days,
           COALESCE((value ->> 'urgentDays')::int, 30)        AS urgent_days
      FROM settings WHERE key = 'expiry.thresholds'
    UNION ALL SELECT 90, 30
    LIMIT 1
)
SELECT c.id AS contract_id,
       (c.end_date - CURRENT_DATE)::int AS remaining_days,
       CASE WHEN c.end_date - CURRENT_DATE < 0   THEN 'EXPIRED'
            WHEN c.end_date - CURRENT_DATE <= 30  THEN 'DAYS_0_TO_30'
            WHEN c.end_date - CURRENT_DATE <= 60  THEN 'DAYS_31_TO_60'
            WHEN c.end_date - CURRENT_DATE <= 90  THEN 'DAYS_61_TO_90'
            WHEN c.end_date - CURRENT_DATE <= 120 THEN 'DAYS_91_TO_120'
            ELSE 'BEYOND_120' END AS band,
       (c.status = 'ACTIVE' AND c.end_date - CURRENT_DATE BETWEEN 0 AND t.expiring_soon_days) AS expiring_soon,
       (c.status = 'ACTIVE' AND c.end_date - CURRENT_DATE BETWEEN 0 AND t.urgent_days)        AS urgent,
       EXISTS (SELECT 1 FROM renewal_cases rc
                WHERE rc.contract_id = c.id AND rc.status NOT IN ('RENEWAL_COMPLETED', 'CLOSED')) AS renewal_in_progress
  FROM contracts c CROSS JOIN t;
