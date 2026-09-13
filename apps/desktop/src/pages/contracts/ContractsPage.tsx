import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { Link, useNavigate } from "react-router";

import type { Band, Contract, ContractStatus } from "@/api/types-domain";
import { ContractStatusBadge, ExpiryChip, RenewalStatusBadge } from "@/components/badges";
import { DataTable, Paginator, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { BAND_LABEL } from "@/lib/bands";
import { BAND_ORDER, CONTRACT_STATUS_LABEL, formatDate, keys, formatMoney } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions } from "@/lib/queries";
import { ContractDialog } from "./ContractDialog";

export function ContractsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const buildings = useBuildingOptions();
  const { state, update, toggleSort } = useListParams({ sort: "end_date" });
  const [dialog, setDialog] = useState(false);

  const query = useQuery({
    queryKey: ["contracts", "list", state],
    queryFn: () =>
      api.listContracts({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        status: (state.filters.status as ContractStatus | undefined) ?? undefined,
        band: state.filters.band as Band | undefined,
        buildingId: state.filters.buildingId,
      }),
    placeholderData: (prev) => prev,
  });

  const [view, setView] = useViewMode("contracts");
  const columns: Column<Contract>[] = [
    { key: "no", header: "Contract", sort: "contract_number", card: "title", render: (c) => <span className="font-medium">{c.contractNumber}</span> },
    { key: "tenant", header: "Tenant", sort: "tenant_name", card: "subtitle", render: (c) => <Link to={`/tenants/${c.tenantId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{c.tenantName}</Link> },
    { key: "building", header: "Building", sort: "building_name", render: (c) => c.buildingName },
    { key: "units", header: "Units", render: (c) => c.unitNumbers },
    { key: "people", header: "No. of occupants", className: "text-right tabular-nums", card: "metric", render: (c) => c.occupantCount },
    { key: "rent", header: "Rent", className: "text-right tabular-nums", card: "metric", render: (c) => (c.rentAmount === null ? "—" : formatMoney(c.rentAmount)) },
    { key: "start", header: "Start", sort: "start_date", render: (c) => formatDate(c.startDate) },
    { key: "end", header: "End", sort: "end_date", card: "metric", render: (c) => formatDate(c.endDate) },
    { key: "remaining", header: "Remaining", sort: "remaining_days", card: "metric", render: (c) => (c.status === "ACTIVE" ? <ExpiryChip band={c.band} days={c.remainingDays} /> : "—") },
    { key: "status", header: "Status", sort: "status", card: "badge", render: (c) => <ContractStatusBadge status={c.status} /> },
    { key: "renewal", header: "Renewal", card: "badge", render: (c) => <RenewalStatusBadge status={c.renewalStatus} /> },
    { key: "assigned", header: "Assigned to", render: (c) => c.caseAssignedEmployeeName ?? c.assignedEmployeeName ?? "—" },
  ];

  return (
    <>
      <PageHeader
        title="Contracts"
        description="Rental contracts with automatic expiry tracking. Renewals create linked contracts so the full timeline stays visible."
        actions={
          can("MANAGE_CONTRACTS") && (
            <Button onClick={() => setDialog(true)}>
              <Plus data-icon="inline-start" />
              New contract
            </Button>
          )
        }
      />
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search contract number, tenant or building" />
        <select className={`${selectClass} w-auto`} value={state.filters.status ?? ""} onChange={(e) => update({ filters: { status: e.target.value } })} aria-label="Contract status">
          <option value="">Any status</option>
          {keys(CONTRACT_STATUS_LABEL).map((s) => (
            <option key={s} value={s}>
              {CONTRACT_STATUS_LABEL[s]}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.band ?? ""} onChange={(e) => update({ filters: { band: e.target.value } })} aria-label="Expiry">
          <option value="">Any expiry</option>
          {BAND_ORDER.map((b) => (
            <option key={b} value={b}>
              {BAND_LABEL[b]}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ filters: { buildingId: e.target.value } })} aria-label="Building">
          <option value="">All buildings</option>
          {buildings.data?.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <ViewToggle value={view} onChange={setView} className="ml-auto" />
      </div>
      <DataTable
        view={view}
        columns={columns}
        rows={query.data?.items}
        rowKey={(c) => c.id}
        loading={query.isPending}
        error={query.error ? "Could not load contracts." : null}
        empty="No contracts match these filters."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(c) => navigate(`/contracts/${c.id}`)}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      <ContractDialog open={dialog} onOpenChange={setDialog} onSaved={(c) => navigate(`/contracts/${c.id}`)} />
    </>
  );
}
