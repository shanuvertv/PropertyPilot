-- A unit's type becomes a category (Residential / Commercial). Bed counts are the number
-- of occupants on the contract, so the old free text ("8 beds", "Office") is classified
-- where possible and kept in the notes so nothing is lost.
UPDATE units
   SET notes = trim(both E'\n' from COALESCE(notes || E'\n', '') || 'Unit type before: ' || unit_type)
 WHERE unit_type IS NOT NULL AND btrim(unit_type) <> ''
   AND upper(btrim(unit_type)) NOT IN ('RESIDENTIAL', 'COMMERCIAL');

UPDATE units SET unit_type = CASE
        WHEN unit_type IS NULL OR btrim(unit_type) = '' THEN NULL
        WHEN lower(unit_type) ~ '(bed|room|resid|apart|flat|studio|villa|accommod|labour|labor|staff)' THEN 'RESIDENTIAL'
        WHEN lower(unit_type) ~ '(office|shop|retail|warehouse|commercial|store|showroom|industrial|workshop)' THEN 'COMMERCIAL'
        ELSE NULL
    END;

ALTER TABLE units ADD CONSTRAINT units_unit_type_check
    CHECK (unit_type IS NULL OR unit_type IN ('RESIDENTIAL', 'COMMERCIAL'));
