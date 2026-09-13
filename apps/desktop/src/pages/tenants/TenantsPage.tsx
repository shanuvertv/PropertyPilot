import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useNavigate } from "react-router";

import type { Tenant } from "@/api/types-domain";
import { DataTable, Paginator, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { selectClass } from "@/components/forms";
import { useBuildingOptions } from "@/lib/queries";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { useListParams } from "@/lib/list-params";
import { TenantDialog } from "./TenantDialog";

export function TenantsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const { state, update, toggleSort } = useListParams({ sort: "name" });
  const buildings = useBuildingOptions();
  const [dialog, setDialog] = useState(false);

  const query = useQuery({
    queryKey: ["tenants", "list", state],
    queryFn: () =>
      api.listTenants({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        buildingId: state.filters.buildingId,
        active: state.filters.active === "yes" ? true : state.filters.active === "no" ? false : undefined,
      }),
    placeholderData: (prev) => prev,
  });

  const [view, setView] = useViewMode("tenants");
  const columns: Column<Tenant>[] = [
    { key: "name", header: "Tenant", sort: "name", render: (t) => <span className="font-medium">{t.name}</span> },
    { key: "contact", header: "Contact person", sort: "contact_person", render: (t) => t.contactPerson ?? "—" },
    { key: "mobile", header: "Mobile", sort: "mobile", render: (t) => t.mobile ?? "—" },
    { key: "email", header: "Email", sort: "email", render: (t) => t.email ?? "—" },
    { key: "contracts", header: "Active contracts", sort: "active_contracts", className: "text-right tabular-nums", card: "metric", render: (t) => t.activeContracts },
    { key: "units", header: "Units", className: "text-right tabular-nums", card: "metric", render: (t) => t.currentUnits },
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
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q, page: 1 })} placeholder="Search name, contact, email or mobile" />
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ page: 1, filters: { buildingId: e.target.value || undefined } })} aria-label="Building">
          <option value="">All properties</option>
          {(buildings.data ?? []).map((b) => (
            <option key={b.id} value={b.id}>{b.name}</option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.active ?? ""} onChange={(e) => update({ page: 1, filters: { active: e.target.value || undefined } })} aria-label="Contract status">
          <option value="">Any status</option>
          <option value="yes">With an active contract</option>
          <option value="no">Without an active contract</option>
        </select>
        <ViewToggle value={view} onChange={setView} className="ml-auto" />
      </div>
      <DataTable
        view={view}
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
