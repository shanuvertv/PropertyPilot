-- Post-dated rent cheques per contract, with a deposit reminder.
CREATE TABLE cheques (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id       UUID NOT NULL REFERENCES contracts (id) ON DELETE CASCADE,
    seq               INTEGER NOT NULL,                 -- 1st, 2nd … cheque of the contract
    cheque_number     TEXT,
    bank_name         TEXT,
    amount_minor      BIGINT NOT NULL CHECK (amount_minor >= 0),
    due_date          DATE NOT NULL,                    -- the date on the cheque = deposit date
    status            TEXT NOT NULL DEFAULT 'PENDING'
                      CHECK (status IN ('PENDING', 'DEPOSITED', 'CLEARED', 'BOUNCED', 'CANCELLED')),
    status_changed_at TIMESTAMPTZ,
    notes             TEXT,
    created_by        UUID REFERENCES users (id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX cheques_contract ON cheques (contract_id, seq);
CREATE INDEX cheques_due ON cheques (status, due_date);

-- Cheque images / deposit slips can be attached like any other document.
ALTER TABLE documents DROP CONSTRAINT IF EXISTS documents_entity_type_check;
ALTER TABLE documents ADD CONSTRAINT documents_entity_type_check
    CHECK (entity_type IN ('building', 'tenant', 'contract', 'notice', 'expense', 'cheque'));

-- Deposit reminders in the notification centre.
ALTER TABLE notifications DROP CONSTRAINT notifications_kind_check;
ALTER TABLE notifications ADD CONSTRAINT notifications_kind_check CHECK (kind IN (
    'CONTRACT_EXPIRING_SOON', 'RENEWAL_NOTICE_PENDING', 'TENANT_RESPONSE_PENDING',
    'FOLLOW_UP_DUE_TODAY', 'OVERDUE_FOLLOW_UP', 'CONTRACT_EXPIRED', 'RENEWAL_REMINDER',
    'RENEWAL_COMPLETED', 'CASE_ASSIGNED', 'CHEQUE_DUE', 'CHEQUE_OVERDUE'));

-- How many days before the cheque date to remind (Settings → Organisation).
INSERT INTO settings (key, value) VALUES ('cheques.reminderDaysBefore', '3'::jsonb)
ON CONFLICT (key) DO NOTHING;
