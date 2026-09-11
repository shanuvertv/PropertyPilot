-- API sessions: opaque bearer tokens issued by POST /api/auth/login.
-- Only a SHA-256 hash of the token is stored; the token itself lives in the
-- client's secure store (Windows Credential Manager / Android keystore).

CREATE TABLE sessions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash     TEXT NOT NULL UNIQUE,
    user_agent     TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at     TIMESTAMPTZ NOT NULL,
    revoked_at     TIMESTAMPTZ
);
CREATE INDEX sessions_user_idx ON sessions (user_id);
CREATE INDEX sessions_expires_idx ON sessions (expires_at);
