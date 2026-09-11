import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useNavigate } from "react-router";

import type { Tenant } from "@/api/types-domain";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { useListParams } from "@/lib/list-params";
import { TenantDialog } from "./TenantDialog";

export function TenantsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const { state, update, toggleSort } = useListParams({ sort: "name" });
  const [dialog, setDialog] = useState(false);

  const query = useQuery({
    queryKey: ["tenants", "list", state],
    queryFn: () => api.listTenants({ q: state.q, page: state.page, pageSize: state.pageSize, sort: state.sort, dir: state.dir }),
    placeholderData: (prev) => prev,
  });

  const columns: Column<Tenant>[] = [
    { key: "name", header: "Tenant", sort: "name", render: (t) => <span className="font-medium">{t.name}</span> },
    { key: "contact", header: "Contact person", sort: "contact_person", render: (t) => t.contactPerson ?? "—" },
    { key: "mobile", header: "Mobile", sort: "mobile", render: (t) => t.mobile ?? "—" },
    { key: "email", header: "Email", sort: "email", render: (t) => t.email ?? "—" },
    { key: "contracts", header: "Active contracts", sort: "active_contracts", className: "text-right tabular-nums", render: (t) => t.activeContracts },
    { key: "units", header: "Units", className: "text-right tabular-nums", render: (t) => t.currentUnits },
  ];

  return (
    <>
      <PageHeader
        title="Tenants"
        description="Tenant master. Open a tenant for current units, contract history, renewals and documents."
        actions={
          can("MANAGE_TENANTS") && (
            <Button onClick={() => setDialog(true)}>
              <Plus data-icon="inline-start" />
              Add tenant
            </Button>
          )
        }
      />
      <div className="mb-3">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search name, contact, email or mobile" />
      </div>
      <DataTable
        columns={columns}
        rows={query.data?.items}
        rowKey={(t) => t.id}
        loading={query.isPending}
        error={query.error ? "Could not load tenants." : null}
        empty="No tenants yet."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(t) => navigate(`/tenants/${t.id}`)}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      <TenantDialog open={dialog} onOpenChange={setDialog} onSaved={(t) => navigate(`/tenants/${t.id}`)} />
    </>
  );
}
