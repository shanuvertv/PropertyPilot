-- Occupants become a number: each unit records how many people live in it, and a bill
-- is split equally between that many people with a "paid so far" count. The per-person
-- register (names, beds, move-in/out) and per-person shares are dropped.

ALTER TABLE units ADD COLUMN occupant_count INTEGER NOT NULL DEFAULT 0 CHECK (occupant_count >= 0);
UPDATE units u SET occupant_count = (
    SELECT count(*) FROM occupants o
     WHERE o.unit_id = u.id AND (o.move_out IS NULL OR o.move_out >= CURRENT_DATE));

ALTER TABLE expenses
    ADD COLUMN split_count   INTEGER NOT NULL DEFAULT 0 CHECK (split_count >= 0),
    ADD COLUMN settled_count INTEGER NOT NULL DEFAULT 0 CHECK (settled_count >= 0);
UPDATE expenses x SET
    split_count   = (SELECT count(*) FROM expense_shares s WHERE s.expense_id = x.id),
    settled_count = (SELECT count(*) FROM expense_shares s WHERE s.expense_id = x.id AND s.settled_at IS NOT NULL);
-- Hand-entered splits become equal splits between the same number of people.
UPDATE expenses SET split_method = 'EQUAL' WHERE split_method = 'CUSTOM';
ALTER TABLE expenses DROP CONSTRAINT expenses_split_method_check;
ALTER TABLE expenses ADD CONSTRAINT expenses_split_method_check CHECK (split_method IN ('NONE', 'EQUAL'));
ALTER TABLE expenses ADD CONSTRAINT expenses_settled_within_split CHECK (settled_count <= split_count);

DROP TABLE expense_shares;
DROP TABLE occupants;
