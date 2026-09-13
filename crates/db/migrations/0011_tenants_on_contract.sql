-- The number of tenants is entered on the contract (per unit it covers), not on the unit.
-- A unit's number of tenants is read from its active contract.

ALTER TABLE contract_units ADD COLUMN occupant_count INTEGER NOT NULL DEFAULT 0 CHECK (occupant_count >= 0);
UPDATE contract_units cu SET occupant_count = u.occupant_count
  FROM units u, contracts c
 WHERE cu.unit_id = u.id AND cu.contract_id = c.id AND c.status = 'ACTIVE' AND u.occupant_count > 0;
ALTER TABLE units DROP COLUMN occupant_count;
