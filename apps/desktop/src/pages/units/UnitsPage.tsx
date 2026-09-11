import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { Link, useNavigate } from "react-router";

import type { Band, RenewalStatus, UnitStatus, UnitSummary } from "@/api/types-domain";
import { ExpiryChip, RenewalStatusBadge, UnitStatusBadge } from "@/components/badges";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { errorMessage, selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { BAND_LABEL } from "@/lib/bands";
import { BAND_ORDER, RENEWAL_STATUS_LABEL, UNIT_STATUS_LABEL, formatDate, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions } from "@/lib/queries";
import { UnitDialog } from "./UnitDialog";

/** Spec §3 + §16: the Unit-Wise Summary — one row per unit with its live contract and renewal state. */
export function UnitsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const buildings = useBuildingOptions();
  const { state, update, toggleSort } = useListParams({ sort: "building_name" });
  const [dialog, setDialog] = useState<{ open: boolean; unit?: UnitSummary | null }>({ open: false });
  const [error, setError] = useState<string | null>(null);

  const query = useQuery({
    queryKey: ["units", "list", state],
    queryFn: () =>
      api.listUnits({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        buildingId: state.filters.buildingId,
        status: state.filters.status as UnitStatus | undefined,
        band: state.filters.band as Band | undefined,
        renewalStatus: state.filters.renewalStatus as RenewalStatus | undefined,
        expiringSoon: state.filters.expiringSoon === "true" ? true : undefined,
      }),
    placeholderData: (prev) => prev,
  });

  const setStatus = useMutation({
    mutationFn: ({ id, status }: { id: string; status: UnitStatus }) => api.setUnitStatus(id, status),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["units"] }),
    onError: (e) => setError(errorMessage(e)),
  });

  const columns: Column<UnitSummary>[] = [
    { key: "building", header: "Building", sort: "building_name", card: "hidden", render: (u) => <Link to={`/buildings/${u.buildingId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{u.buildingName}</Link> },
    { key: "unit", header: "Unit", sort: "unit_number", card: "title", render: (u) => <span className="font-medium">{u.unitNumber}<span className="font-normal text-muted-foreground md:hidden"> · {u.buildingName}</span></span> },
    { key: "type", header: "Type", sort: "unit_type", render: (u) => u.unitType ?? "—" },
    { key: "tenant", header: "Tenant", sort: "tenant_name", render: (u) => (u.tenantId ? <Link to={`/tenants/${u.tenantId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{u.tenantName}</Link> : <span className="text-muted-foreground">—</span>) },
    { key: "start", header: "Start", sort: "start_date", render: (u) => formatDate(u.startDate) },
    { key: "end", header: "End", sort: "end_date", render: (u) => formatDate(u.endDate) },
    { key: "remaining", header: "Remaining", sort: "remaining_days", render: (u) => <ExpiryChip band={u.band} days={u.remainingDays} /> },
    { key: "status", header: "Status", sort: "status", render: (u) => <UnitStatusBadge status={u.status} /> },
    { key: "renewal", header: "Renewal", sort: "renewal_status", render: (u) => <RenewalStatusBadge status={u.renewalStatus} /> },
    { key: "assigned", header: "Assigned to", render: (u) => u.assignedEmployeeName ?? "—" },
    {
      key: "actions",
      header: "",
      className: "text-right whitespace-nowrap",
      render: (u) => (
        <span className="inline-flex gap-1" onClick={(e) => e.stopPropagation()}>
          {u.contractId && can("VIEW_CONTRACTS") && (
            <Button variant="ghost" size="xs" onClick={() => navigate(`/contracts/${u.contractId}`)}>
              Contract
            </Button>
          )}
          {u.caseId && can("VIEW_RENEWALS") && (
            <Button variant="ghost" size="xs" onClick={() => navigate(`/renewals/${u.caseId}`)}>
              Renewal
            </Button>
          )}
          {!u.caseId && u.contractId && u.expiringSoon && can("MANAGE_RENEWALS") && (
            <Button variant="outline" size="xs" onClick={() => navigate(`/contracts/${u.contractId}?start=renewal`)}>
              Start renewal
            </Button>
          )}
          {!u.contractId && can("UPDATE_UNIT_STATUS") && (
            <select
              className={`${selectClass} h-6 w-auto text-[12px]`}
              value={u.status}
              aria-label={`Status of unit ${u.unitNumber}`}
              onChange={(e) => setStatus.mutate({ id: u.id, status: e.target.value as UnitStatus })}
            >
              {(["VACANT", "RESERVED", "MAINTENANCE"] as UnitStatus[]).map((s) => (
                <option key={s} value={s}>
                  {UNIT_STATUS_LABEL[s]}
                </option>
              ))}
            </select>
          )}
        </span>
      ),
    },
  ];

  return (
    <>
      <PageHeader
        title="Units"
        description="Unit-wise summary: every unit with its current tenant, contract dates, remaining days and renewal status."
        actions={
          can("MANAGE_UNITS") && (
            <Button onClick={() => setDialog({ open: true, unit: null })}>
              <Plus data-icon="inline-start" />
              Add unit
            </Button>
          )
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-3">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search unit, building, tenant or contract" />
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ filters: { buildingId: e.target.value } })} aria-label="Building">
          <option value="">All buildings</option>
          {buildings.data?.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.status ?? ""} onChange={(e) => update({ filters: { status: e.target.value } })} aria-label="Unit status">
          <option value="">Any status</option>
          {keys(UNIT_STATUS_LABEL).map((s) => (
            <option key={s} value={s}>
              {UNIT_STATUS_LABEL[s]}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.band ?? ""} onChange={(e) => update({ filters: { band: e.target.value } })} aria-label="Contract expiry">
          <option value="">Any expiry</option>
          {BAND_ORDER.map((b) => (
            <option key={b} value={b}>
              {BAND_LABEL[b]}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.renewalStatus ?? ""} onChange={(e) => update({ filters: { renewalStatus: e.target.value } })} aria-label="Renewal status">
          <option value="">Any renewal status</option>
          {keys(RENEWAL_STATUS_LABEL).map((s) => (
            <option key={s} value={s}>
              {RENEWAL_STATUS_LABEL[s]}
            </option>
          ))}
        </select>
        {(state.q || Object.keys(state.filters).length > 0) && (
          <Button variant="ghost" size="sm" onClick={() => update({ q: "", filters: { buildingId: undefined, status: undefined, band: undefined, renewalStatus: undefined, expiringSoon: undefined } })}>
            Clear
          </Button>
        )}
      </div>
      <DataTable
        columns={columns}
        rows={query.data?.items}
        rowKey={(u) => u.id}
        loading={query.isPending}
        error={query.error ? "Could not load units." : null}
        empty="No units match these filters."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={can("MANAGE_UNITS") ? (u) => setDialog({ open: true, unit: u }) : undefined}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      <UnitDialog open={dialog.open} onOpenChange={(o) => setDialog((d) => ({ ...d, open: o }))} unit={dialog.unit} />
    </>
  );
}
