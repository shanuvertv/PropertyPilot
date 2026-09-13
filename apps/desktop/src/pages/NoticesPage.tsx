import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";

import type { Band, NoticeStatus, RenewalCase } from "@/api/types-domain";
import { ExpiryChip, NoticeStatusBadge, RenewalStatusBadge } from "@/components/badges";
import { DataTable, Paginator, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { BAND_LABEL } from "@/lib/bands";
import { BAND_ORDER, NOTICE_STATUS_LABEL, TENANT_RESPONSE_LABEL, formatDate, formatDateTime, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions, useEmployees } from "@/lib/queries";

/** Spec §9: Renewal Notice Tracking. */
export function NoticesPage() {
  const { api } = useApp();
  const navigate = useNavigate();
  const buildings = useBuildingOptions();
  const employees = useEmployees();
  const { state, update, toggleSort } = useListParams({ sort: "remaining_days" });

  const query = useQuery({
    queryKey: ["renewals", "notices", state],
    queryFn: () =>
      api.listCases({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        openOnly: state.filters.includeClosed === "true" ? undefined : true,
        noticeStatus: state.filters.noticeStatus as NoticeStatus | undefined,
        buildingId: state.filters.buildingId,
        assignedEmployeeId: state.filters.assignedEmployeeId,
        band: state.filters.band as Band | undefined,
      }),
    placeholderData: (prev) => prev,
  });

  const [view, setView] = useViewMode("notices");
  const columns: Column<RenewalCase>[] = [
    { key: "tenant", header: "Tenant", sort: "tenant_name", card: "title", render: (c) => <span className="font-medium">{c.tenantName}</span> },
    { key: "building", header: "Building", card: "subtitle", render: (c) => c.buildingName },
    { key: "unit", header: "Unit", render: (c) => c.unitNumbers },
    { key: "expiry", header: "Expiry", sort: "end_date", card: "metric", render: (c) => <span>{formatDate(c.endDate)} <ExpiryChip band={c.band} days={c.remainingDays} /></span> },
    { key: "required", header: "Notice required", render: (c) => (c.noticeStatus === "NOT_REQUIRED" ? "No" : "Yes") },
    { key: "sent", header: "Notice sent", render: (c) => (c.noticeSentAt ? "Yes" : "No") },
    { key: "sentDate", header: "Sent date", render: (c) => (c.noticeSentAt ? formatDateTime(c.noticeSentAt) : "—") },
    { key: "response", header: "Response", render: (c) => (c.latestResponse ? TENANT_RESPONSE_LABEL[c.latestResponse] : "—") },
    { key: "status", header: "Notice status", sort: "status", card: "badge", render: (c) => <NoticeStatusBadge status={c.noticeStatus} /> },
    { key: "case", header: "Case", card: "badge", render: (c) => <RenewalStatusBadge status={c.status} /> },
    { key: "assigned", header: "Employee", render: (c) => c.assignedEmployeeName ?? "—" },
    { key: "actions", header: "", className: "text-right", card: "hidden", render: (c) => <span onClick={(e) => e.stopPropagation()}><Button size="xs" variant="ghost" onClick={() => navigate(`/renewals/${c.id}`)}>Open</Button></span> },
  ];

  return (
    <>
      <PageHeader
        title="Renewal Notices"
        description="Which tenants have been notified, when, and what they answered. Open a case to prepare or send its notice."
        actions={<Link to="/emails" className="text-[13px] text-primary hover:underline">Email log →</Link>}
      />
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search tenant, contract or building" />
        <select className={`${selectClass} w-auto`} value={state.filters.noticeStatus ?? ""} onChange={(e) => update({ filters: { noticeStatus: e.target.value } })} aria-label="Notice status">
          <option value="">Any notice status</option>
          {keys(NOTICE_STATUS_LABEL).map((s) => (
            <option key={s} value={s}>
              {NOTICE_STATUS_LABEL[s]}
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
        <select className={`${selectClass} w-auto`} value={state.filters.assignedEmployeeId ?? ""} onChange={(e) => update({ filters: { assignedEmployeeId: e.target.value } })} aria-label="Employee">
          <option value="">Any employee</option>
          {employees.data?.map((e) => (
            <option key={e.id} value={e.id}>
              {e.name}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.band ?? ""} onChange={(e) => update({ filters: { band: e.target.value } })} aria-label="Expiry period">
          <option value="">Any expiry period</option>
          {BAND_ORDER.map((b) => (
            <option key={b} value={b}>
              {BAND_LABEL[b]}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-1.5 text-[12.5px]">
          <input type="checkbox" checked={state.filters.includeClosed === "true"} onChange={(e) => update({ filters: { includeClosed: e.target.checked ? "true" : undefined } })} />
          Include closed cases
        </label>
        <ViewToggle value={view} onChange={setView} className="ml-auto" />
      </div>
      <DataTable
        view={view}
        columns={columns}
        rows={query.data?.items}
        rowKey={(c) => c.id}
        loading={query.isPending}
        error={query.error ? "Could not load notices." : null}
        empty="No renewal cases match these filters."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(c) => navigate(`/renewals/${c.id}`)}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
    </>
  );
}
