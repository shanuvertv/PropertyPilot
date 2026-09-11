-- A formal renewal notice should name the tenant entity, not only the contact person.
-- Only touches the seeded text: templates an Admin has already edited are left alone.
UPDATE email_templates
SET body_text = replace(
        body_text,
        'We refer to the tenancy contract {{ContractNumber}} for Unit {{UnitNumber}}, {{BuildingName}}, which expires',
        'We refer to the tenancy contract {{ContractNumber}} between {{CompanyName}} and {{TenantName}} for Unit {{UnitNumber}}, {{BuildingName}}, which expires'
    )
WHERE key = 'RENEWAL_NOTICE' AND updated_by IS NULL;
