import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";

import type { Band, Contract, RenewalStatus } from "@/api/types-domain";
import { BandDot, ExpiryChip, NoticeStatusBadge, RenewalStatusBadge } from "@/components/badges";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { BAND_LABEL } from "@/lib/bands";
import { RENEWAL_STATUS_LABEL, formatDate, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions, useEmployees } from "@/lib/queries";
import { cn } from "@/lib/utils";

/** Spec §6: contracts approaching expiry, grouped by band, with their renewal case state. */
const BANDS: Band[] = ["EXPIRED", "DAYS_0_TO_30", "DAYS_31_TO_60", "DAYS_61_TO_90", "DAYS_91_TO_120"];

export function RenewalsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const buildings = useBuildingOptions();
  const employees = useEmployees();
  const { state, update, toggleSort } = useListParams({ sort: "remaining_days" });

  const dashboard = useQuery({ queryKey: ["dashboard"], queryFn: () => api.dashboard(), staleTime: 15_000 });
  const query = useQuery({
    queryKey: ["contracts", "renewals", state],
    queryFn: () =>
      api.listContracts({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        status: "ACTIVE",
        band: state.filters.band as Band | undefined,
        buildingId: state.filters.buildingId,
        renewalStatus: state.filters.renewalStatus as RenewalStatus | undefined,
        hasOpenCase: state.filters.hasOpenCase === "true" ? true : state.filters.hasOpenCase === "false" ? false : undefined,
      }),
    placeholderData: (prev) => prev,
  });

  const bandCount = (b: Band) => dashboard.data?.bands.find((x) => x.band === b)?.count ?? 0;

  const columns: Column<Contract>[] = [
    { key: "tenant", header: "Tenant", sort: "tenant_name", render: (c) => (
      <div>
        <div className="font-medium">{c.tenantName}</div>
        <div className="text-[12px] text-muted-foreground">{c.tenantContact ?? "—"}</div>
      </div>
    ) },
    { key: "building", header: "Building", sort: "building_name", render: (c) => c.buildingName },
    { key: "units", header: "Unit", render: (c) => c.unitNumbers },
    { key: "start", header: "Start", sort: "start_date", render: (c) => formatDate(c.startDate) },
    { key: "end", header: "End", sort: "end_date", render: (c) => formatDate(c.endDate) },
    { key: "remaining", header: "Remaining", sort: "remaining_days", render: (c) => <ExpiryChip band={c.band} days={c.remainingDays} /> },
    { key: "assigned", header: "Assigned", render: (c) => c.caseAssignedEmployeeName ?? c.assignedEmployeeName ?? "—" },
    { key: "status", header: "Renewal status", render: (c) => (c.caseId ? <RenewalStatusBadge status={c.renewalStatus} /> : <span className="text-muted-foreground">Not started</span>) },
    { key: "notice", header: "Notice", render: (c) => <NoticeStatusBadge status={c.caseId ? c.noticeStatus : null} /> },
    {
      key: "actions",
      header: "",
      className: "text-right whitespace-nowrap",
      render: (c) => (
        <span onClick={(e) => e.stopPropagation()}>
          {c.caseId ? (
            <Button variant="ghost" size="xs" onClick={() => navigate(`/renewals/${c.caseId}`)}>
              Open case
            </Button>
          ) : can("MANAGE_RENEWALS") ? (
            <Button variant="outline" size="xs" onClick={() => navigate(`/contracts/${c.id}?start=renewal`)}>
              Start renewal
            </Button>
          ) : null}
        </span>
      ),
    },
  ];

  return (
    <>
      <PageHeader title="Renewals" description="Active contracts by remaining days. Start a case, then work it through notice, tenant response, follow-ups and completion." />

      <div className="mb-4 grid grid-cols-2 gap-2 md:grid-cols-5">
        {BANDS.map((b) => {
          const active = state.filters.band === b;
          return (
            <button
              key={b}
              type="button"
              onClick={() => update({ filters: { band: active ? undefined : b } })}
              className={cn(
                "flex items-center justify-between rounded-md border bg-card px-3 py-2.5 text-left transition-colors hover:bg-muted",
                active && "border-primary ring-2 ring-primary/20",
              )}
              aria-pressed={active}
            >
              <span className="flex items-center gap-2 text-[13px]">
                <BandDot band={b} />
                {BAND_LABEL[b]}
              </span>
              <span className="text-[18px] font-semibold tabular-nums">{bandCount(b)}</span>
            </button>
          );
        })}
      </div>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search tenant, contract or building" />
        <select className={`${selectClass} w-auto`} value={state.filters.hasOpenCase ?? ""} onChange={(e) => update({ filters: { hasOpenCase: e.target.value } })} aria-label="Case">
          <option value="">With or without a case</option>
          <option value="true">Case open</option>
          <option value="false">No case yet</option>
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.renewalStatus ?? ""} onChange={(e) => update({ filters: { renewalStatus: e.target.value } })} aria-label="Renewal status">
          <option value="">Any renewal status</option>
          {keys(RENEWAL_STATUS_LABEL).map((s) => (
            <option key={s} value={s}>
              {RENEWAL_STATUS_LABEL[s]}
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
        {employees.data && (
          <Link to="/follow-ups" className="ml-auto text-[13px] text-primary hover:underline">
            Follow-ups →
          </Link>
        )}
      </div>

      <DataTable
        columns={columns}
        rows={query.data?.items}
        rowKey={(c) => c.id}
        loading={query.isPending}
        error={query.error ? "Could not load renewals." : null}
        empty="No active contracts in this range."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(c) => navigate(c.caseId ? `/renewals/${c.caseId}` : `/contracts/${c.id}`)}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
    </>
  );
}
