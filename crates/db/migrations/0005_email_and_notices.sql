-- Phase 5: email templates, outbound messages, renewal notices (spec §7, §9, §10).
-- Bodies are plain text with {{Placeholders}}; the server renders HTML for email
-- and a Typst letter for the PDF, so nothing user-editable is raw HTML.

CREATE TABLE email_templates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    subject     TEXT NOT NULL,
    body_text   TEXT NOT NULL,
    active      BOOLEAN NOT NULL DEFAULT TRUE,
    updated_by  UUID REFERENCES users (id),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO email_templates (key, name, subject, body_text) VALUES
('RENEWAL_NOTICE', 'Renewal Notice',
 'Renewal of tenancy contract {{ContractNumber}} – {{BuildingName}}, Unit {{UnitNumber}}',
 'Dear {{ContactPerson}},

We refer to the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}}, which expires on {{ContractEndDate}} ({{RemainingDays}} days from today).

We would be pleased to renew the contract for {{ProposedRenewalPeriod}}. {{OtherRenewalTerms}}

Kindly confirm your intention to renew at your earliest convenience so that we can prepare the renewal documents in good time.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('FIRST_REMINDER', 'First Reminder',
 'Reminder: renewal of contract {{ContractNumber}} – {{BuildingName}}, Unit {{UnitNumber}}',
 'Dear {{ContactPerson}},

This is a friendly reminder that the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}} expires on {{ContractEndDate}} ({{RemainingDays}} days from today).

We have not yet received your response to our renewal notice. Please let us know whether you wish to renew.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('SECOND_REMINDER', 'Second Reminder',
 'Second reminder: contract {{ContractNumber}} expires on {{ContractEndDate}}',
 'Dear {{ContactPerson}},

The tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}} expires in {{RemainingDays}} days, on {{ContractEndDate}}.

To avoid any interruption, please confirm your renewal decision as soon as possible.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('FINAL_REMINDER', 'Final Reminder',
 'Final reminder: contract {{ContractNumber}} expires on {{ContractEndDate}}',
 'Dear {{ContactPerson}},

This is our final reminder that the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}} expires on {{ContractEndDate}} ({{RemainingDays}} days from today).

If we do not hear from you, we will assume you do not intend to renew and will make arrangements for the unit accordingly.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('RENEWAL_CONFIRMATION', 'Renewal Confirmation',
 'Renewal confirmed – contract {{ContractNumber}}, Unit {{UnitNumber}}',
 'Dear {{ContactPerson}},

Thank you for confirming the renewal of the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}}. We will prepare the renewal documents and contact you shortly.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('NON_RENEWAL_CONFIRMATION', 'Non-Renewal Confirmation',
 'Non-renewal acknowledged – contract {{ContractNumber}}, Unit {{UnitNumber}}',
 'Dear {{ContactPerson}},

We acknowledge that the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}} will not be renewed and will end on {{ContractEndDate}}. Our team will contact you regarding the handover of the unit.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('FOLLOW_UP_REMINDER', 'Follow-up Reminder',
 'Following up: renewal of contract {{ContractNumber}}',
 'Dear {{ContactPerson}},

Following our recent conversation about the renewal of contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}}, we would appreciate your update.

Kind regards,
{{ResponsibleEmployee}}
{{CompanyName}}'),
('INTERNAL_REMINDER', 'Internal Reminder (to employee)',
 '[PropertyPilot] {{TenantName}} – {{BuildingName}} {{UnitNumber}} expires in {{RemainingDays}} days',
 'Contract {{ContractNumber}} ({{TenantName}}, {{BuildingName}} unit {{UnitNumber}}) ends on {{ContractEndDate}} – {{RemainingDays}} days from today.

Renewal status: {{RenewalStatus}}. Notice: {{NoticeStatus}}.

Open the case in PropertyPilot to take action.');

CREATE TABLE email_messages (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_type           TEXT NOT NULL,                 -- template key or CUSTOM
    tenant_id            UUID REFERENCES tenants (id),
    contract_id          UUID REFERENCES contracts (id),
    case_id              UUID REFERENCES renewal_cases (id),
    to_addresses         TEXT[] NOT NULL,
    cc_addresses         TEXT[] NOT NULL DEFAULT '{}',
    subject              TEXT NOT NULL,
    body_text            TEXT NOT NULL,
    body_html            TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'QUEUED' CHECK (status IN ('QUEUED', 'SENT', 'DELIVERED', 'FAILED')),
    attempts             INT NOT NULL DEFAULT 0,
    last_error           TEXT,
    provider             TEXT,
    provider_message_id  TEXT,
    sent_by              UUID REFERENCES users (id),
    queued_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    next_attempt_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at              TIMESTAMPTZ
);
CREATE INDEX email_messages_queue_idx ON email_messages (status, next_attempt_at) WHERE status = 'QUEUED';
CREATE INDEX email_messages_tenant_idx ON email_messages (tenant_id, queued_at DESC);
CREATE INDEX email_messages_contract_idx ON email_messages (contract_id, queued_at DESC);
CREATE INDEX email_messages_case_idx ON email_messages (case_id, queued_at DESC);

CREATE TABLE email_attachments (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id   UUID NOT NULL REFERENCES email_messages (id) ON DELETE CASCADE,
    document_id  UUID REFERENCES documents (id),
    file_name    TEXT NOT NULL,
    content_type TEXT NOT NULL,
    storage_key  TEXT NOT NULL
);
CREATE INDEX email_attachments_message_idx ON email_attachments (message_id);

-- Spec §7 steps 3–4: the prepared letter, its PDF and the email it went out with.
CREATE TABLE renewal_notices (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id            UUID NOT NULL REFERENCES renewal_cases (id) ON DELETE CASCADE,
    status             TEXT NOT NULL DEFAULT 'DRAFT' CHECK (status IN ('DRAFT', 'SENT', 'DELIVERED', 'FAILED')),
    proposed_period    TEXT,
    other_terms        TEXT,
    subject            TEXT NOT NULL,
    body_text          TEXT NOT NULL,
    recipient          TEXT,
    cc_addresses       TEXT[] NOT NULL DEFAULT '{}',
    pdf_document_id    UUID REFERENCES documents (id),
    email_message_id   UUID REFERENCES email_messages (id),
    sent_by            UUID REFERENCES users (id),
    sent_at            TIMESTAMPTZ,
    created_by         UUID REFERENCES users (id),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX renewal_notices_draft_unique ON renewal_notices (case_id) WHERE status = 'DRAFT';
CREATE INDEX renewal_notices_case_idx ON renewal_notices (case_id, created_at DESC);

INSERT INTO settings (key, value) VALUES
    ('org.letterhead', '{"companyName": "Leasing Department", "addressLines": ["P.O. Box 0000", "Dubai, United Arab Emirates"], "phone": "", "email": "", "footer": "This notice is issued in accordance with the terms of the tenancy contract."}'),
    ('mail.sender', '{"name": "Leasing Department", "address": "leasing@example.com"}');
