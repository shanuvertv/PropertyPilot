import { PageHeader } from "@/components/PageHeader";
import { ImportCard } from "@/pages/settings/ImportCard";

/** Bulk-load an existing tenant list (Excel) — buildings, units, tenants and contracts. */
export function ImportPage() {
  return (
    <>
      <PageHeader
        title="Import data"
        description="Load an existing tenant list from Excel: buildings, units, tenants and contracts with their dates, number of occupants (capacity) and rent. Preview first, then confirm; re-running the same file never duplicates."
      />
      <ImportCard />
    </>
  );
}
