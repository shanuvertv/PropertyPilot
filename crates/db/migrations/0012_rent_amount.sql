-- Rent as a number on the contract (minor units, e.g. fils), next to the free-text terms.
ALTER TABLE contracts ADD COLUMN rent_amount_minor BIGINT CHECK (rent_amount_minor IS NULL OR rent_amount_minor >= 0);
