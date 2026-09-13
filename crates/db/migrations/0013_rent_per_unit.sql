-- Rent is recorded per unit under the contract (like the number of tenants); the
-- contract's rent is the sum. A contract-level amount on a single-unit contract moves
-- to that unit; on a multi-unit contract the split is unknown, so the units start empty
-- (re-importing the tenant list fills them in per unit).
ALTER TABLE contract_units ADD COLUMN rent_amount_minor BIGINT CHECK (rent_amount_minor IS NULL OR rent_amount_minor >= 0);
UPDATE contract_units cu SET rent_amount_minor = c.rent_amount_minor
  FROM contracts c
 WHERE cu.contract_id = c.id AND c.rent_amount_minor IS NOT NULL
   AND (SELECT count(*) FROM contract_units x WHERE x.contract_id = c.id) = 1;
ALTER TABLE contracts DROP COLUMN rent_amount_minor;
