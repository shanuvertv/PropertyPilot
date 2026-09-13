-- Users are never removed (the audit trail and created_by columns point at them);
-- deleting a user anonymises the login and hides the row from the users list.
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;
CREATE INDEX users_live ON users (name) WHERE deleted_at IS NULL;
