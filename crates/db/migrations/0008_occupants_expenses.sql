-- Occupants (the people living in a unit — shared accommodation) and expenses per unit
-- with optional splitting of each bill between the occupants.
-- Money is stored in minor units (fils) as integers so shares always add up exactly.

CREATE TABLE occupants (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_id      UUID NOT NULL REFERENCES units (id),
    tenant_id    UUID REFERENCES tenants (id),
    full_name    TEXT NOT NULL,
    id_number    TEXT,
    phone        TEXT,
    email        TEXT,
    bed_label    TEXT,
    move_in      DATE NOT NULL DEFAULT CURRENT_DATE,
    move_out     DATE,
    notes        TEXT,
    created_by   UUID REFERENCES users (id),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (move_out IS NULL OR move_out >= move_in)
);
CREATE INDEX occupants_unit ON occupants (unit_id, move_out);

CREATE TABLE expenses (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_id       UUID NOT NULL REFERENCES units (id),
    category      TEXT NOT NULL
                  CHECK (category IN ('ELECTRICITY', 'WATER', 'GAS', 'INTERNET', 'MAINTENANCE',
                                      'CLEANING', 'MUNICIPALITY', 'OTHER')),
    description   TEXT NOT NULL,
    amount_minor  BIGINT NOT NULL CHECK (amount_minor >= 0),
    expense_date  DATE NOT NULL,
    period_start  DATE,
    period_end    DATE,
    vendor        TEXT,
    reference     TEXT,
    split_method  TEXT NOT NULL DEFAULT 'NONE' CHECK (split_method IN ('NONE', 'EQUAL', 'CUSTOM')),
    notes         TEXT,
    created_by    UUID REFERENCES users (id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (period_end IS NULL OR period_start IS NULL OR period_end >= period_start)
);
CREATE INDEX expenses_unit_date ON expenses (unit_id, expense_date DESC);
CREATE INDEX expenses_date ON expenses (expense_date DESC);

CREATE TABLE expense_shares (
    expense_id    UUID NOT NULL REFERENCES expenses (id) ON DELETE CASCADE,
    occupant_id   UUID NOT NULL REFERENCES occupants (id),
    amount_minor  BIGINT NOT NULL CHECK (amount_minor >= 0),
    settled_at    TIMESTAMPTZ,
    PRIMARY KEY (expense_id, occupant_id)
);
CREATE INDEX expense_shares_occupant ON expense_shares (occupant_id);

-- Documents (bills, receipts) can be attached to an expense like any other record.
ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_entity_type_check;
ALTER TABLE documents ADD CONSTRAINT documents_entity_type_check
    CHECK (entity_type IN ('building', 'tenant', 'contract', 'notice', 'expense'));
