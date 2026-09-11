-- Phase 1: buildings, units, tenants, documents (spec §2, §3, §4).
-- Records are archived (soft-deleted) rather than removed so contracts and the
-- audit trail keep their references.

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE buildings (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name           TEXT NOT NULL,
    code           TEXT NOT NULL,
    location       TEXT,
    building_type  TEXT,
    notes          TEXT,
    created_by     UUID REFERENCES users (id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at    TIMESTAMPTZ
);
CREATE UNIQUE INDEX buildings_code_unique ON buildings (lower(code)) WHERE archived_at IS NULL;
CREATE INDEX buildings_name_trgm ON buildings USING gin (name gin_trgm_ops);

CREATE TABLE units (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    building_id    UUID NOT NULL REFERENCES buildings (id),
    unit_number    TEXT NOT NULL,
    floor          TEXT,
    unit_type      TEXT,
    status         TEXT NOT NULL DEFAULT 'VACANT'
                   CHECK (status IN ('VACANT', 'OCCUPIED', 'RESERVED', 'MAINTENANCE')),
    notes          TEXT,
    created_by     UUID REFERENCES users (id),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at    TIMESTAMPTZ
);
CREATE UNIQUE INDEX units_building_number_unique ON units (building_id, lower(unit_number)) WHERE archived_at IS NULL;
CREATE INDEX units_building_idx ON units (building_id) WHERE archived_at IS NULL;
CREATE INDEX units_number_trgm ON units USING gin (unit_number gin_trgm_ops);

CREATE TABLE tenants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    contact_person  TEXT,
    mobile          TEXT,
    email           TEXT,
    alt_contact     TEXT,
    address         TEXT,
    notes           TEXT,
    created_by      UUID REFERENCES users (id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at     TIMESTAMPTZ
);
CREATE INDEX tenants_name_trgm ON tenants USING gin (name gin_trgm_ops);
CREATE INDEX tenants_contact_trgm ON tenants USING gin (coalesce(contact_person, '') gin_trgm_ops);

-- Attachments on buildings, tenants, contracts and notices. Bytes live in
-- document_blobs (the default StorageProvider) keyed by storage_key, so the
-- provider can be swapped for a file share or Azure Blob without a schema change.
CREATE TABLE documents (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type   TEXT NOT NULL CHECK (entity_type IN ('building', 'tenant', 'contract', 'notice')),
    entity_id     UUID NOT NULL,
    file_name     TEXT NOT NULL,
    content_type  TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    storage_key   TEXT NOT NULL UNIQUE,
    uploaded_by   UUID REFERENCES users (id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX documents_entity_idx ON documents (entity_type, entity_id, created_at DESC);

CREATE TABLE document_blobs (
    storage_key   TEXT PRIMARY KEY,
    content_type  TEXT NOT NULL,
    bytes         BYTEA NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
