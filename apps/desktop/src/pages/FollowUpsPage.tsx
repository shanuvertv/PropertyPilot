import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";

import type { FollowUp } from "@/api/types-domain";
import { FollowUpStatusBadge } from "@/components/badges";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { errorMessage } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DateRange } from "@/components/DateRange";
import { SearchBox } from "@/components/SearchBox";
import { selectClass } from "@/components/forms";
import { useBuildingOptions, useEmployees } from "@/lib/queries";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { FOLLOW_UP_TYPE_LABEL, formatDate, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { FollowUpDialog } from "@/pages/renewals/RenewalCasePage";
import { cn } from "@/lib/utils";

type Scope = "today" | "overdue" | "upcoming" | "all";

/** Spec §12: Today's, overdue and upcoming follow-ups. */
export function FollowUpsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { state, update } = useListParams({ sort: "due_date" });
  const buildings = useBuildingOptions();
  const employees = useEmployees();
  const scope = (state.filters.scope as Scope | undefined) ?? "today";
  const mine = state.filters.mine === "true";
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState<FollowUp | null>(null);

  const counts = useQuery({ queryKey: ["follow-ups", "counts", mine], queryFn: () => api.followUpCounts(mine) });
  const query = useQuery({
    queryKey: ["follow-ups", "list", state, scope, mine],
    queryFn: () =>
      api.listFollowUps({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: "due_date",
        dir: scope === "overdue" ? "desc" : "asc",
        scope,
        mine,
        followUpType: state.filters.type,
        assignedEmployeeId: state.filters.assignedEmployeeId,
        buildingId: state.filters.buildingId,
        from: state.filters.from,
        to: state.filters.to,
      }),
    placeholderData: (prev) => prev,
  });
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["follow-ups"] });
  const setStatus = useMutation({
    mutationFn: ({ id, status }: { id: string; status: "DONE" | "CANCELLED" }) => api.setFollowUpStatus(id, status),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e)),
  });

  const columns: Column<FollowUp>[] = [
    { key: "due", header: "Due", render: (f) => (
      <span className={cn("tabular-nums", f.status === "OPEN" && f.daysUntilDue < 0 && "text-destructive")}>
        {formatDate(f.dueDate)}
        {f.status === "OPEN" && f.daysUntilDue < 0 && ` · ${-f.daysUntilDue}d overdue`}
      </span>
    ) },
    { key: "type", header: "Type", render: (f) => FOLLOW_UP_TYPE_LABEL[f.followUpType] },
    { key: "tenant", header: "Tenant", card: "title", render: (f) => <Link to={`/tenants/${f.tenantId}`} className="font-medium hover:underline" onClick={(e) => e.stopPropagation()}>{f.tenantName}</Link> },
    { key: "contract", header: "Contract", render: (f) => `${f.contractNumber} · ${f.buildingName} ${f.unitNumbers}` },
    { key: "notes", header: "Notes", render: (f) => <span className="line-clamp-1 max-w-[320px]">{f.notes ?? "—"}</span> },
    { key: "assigned", header: "Assigned", render: (f) => f.assignedEmployeeName ?? "—" },
    { key: "status", header: "Status", render: (f) => <FollowUpStatusBadge status={f.status} /> },
    {
      key: "actions",
      header: "",
      className: "text-right whitespace-nowrap",
      render: (f) =>
        f.status === "OPEN" && can("MANAGE_FOLLOW_UPS") ? (
          <span className="inline-flex gap-1" onClick={(e) => e.stopPropagation()}>
            <Button size="xs" variant="ghost" onClick={() => setEdit(f)}>
              Edit
            </Button>
            <Button size="xs" variant="outline" onClick={() => setStatus.mutate({ id: f.id, status: "DONE" })}>
              Done
            </Button>
          </span>
        ) : null,
    },
  ];

  const tab = (s: Scope, label: string, n?: number) => (
    <TabsTrigger value={s}>
      {label}
      {n !== undefined && <span className="ml-1.5 rounded-full bg-muted px-1.5 text-[11px] tabular-nums">{n}</span>}
    </TabsTrigger>
  );

  return (
    <>
      <PageHeader title="Follow-Ups" description="Calls, emails, meetings and internal discussions scheduled on renewal cases." />
      {error && (
        <Alert variant="destructive" className="mb-3">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      <div className="mb-3 flex flex-wrap items-center gap-3">
        <Tabs value={scope} onValueChange={(v) => update({ filters: { scope: String(v) } })}>
          <TabsList>
            {tab("today", "Today", counts.data?.today)}
            {tab("overdue", "Overdue", counts.data?.overdue)}
            {tab("upcoming", "Upcoming", counts.data?.upcoming)}
            {tab("all", "All")}
          </TabsList>
        </Tabs>
        <label className="flex items-center gap-2 text-[13px]">
          <Checkbox checked={mine} onCheckedChange={(c) => update({ filters: { mine: c === true ? "true" : undefined } })} />
          Only mine
        </label>
      </div>
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q, page: 1 })} placeholder="Search tenant, contract, unit, building or notes" />
        <select className={`${selectClass} w-auto`} value={state.filters.type ?? ""} onChange={(e) => update({ page: 1, filters: { type: e.target.value || undefined } })} aria-label="Type">
          <option value="">Any type</option>
          {keys(FOLLOW_UP_TYPE_LABEL).map((t) => (
            <option key={t} value={t}>{FOLLOW_UP_TYPE_LABEL[t]}</option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ page: 1, filters: { buildingId: e.target.value || undefined } })} aria-label="Building">
          <option value="">All properties</option>
          {(buildings.data ?? []).map((b) => (
            <option key={b.id} value={b.id}>{b.name}</option>
          ))}
        </select>
        {!mine && (
          <select className={`${selectClass} w-auto`} value={state.filters.assignedEmployeeId ?? ""} onChange={(e) => update({ page: 1, filters: { assignedEmployeeId: e.target.value || undefined } })} aria-label="Assigned to">
            <option value="">Anyone</option>
            {(employees.data ?? []).map((u) => (
              <option key={u.id} value={u.id}>{u.name}</option>
            ))}
          </select>
        )}
        <DateRange from={state.filters.from} to={state.filters.to} onChange={(from, to) => update({ page: 1, filters: { from, to } })} />
      </div>
      <DataTable
        columns={columns}
        rows={query.data?.items}
        rowKey={(f) => f.id}
        loading={query.isPending}
        error={query.error ? "Could not load follow-ups." : null}
        empty={scope === "today" ? "Nothing due today." : scope === "overdue" ? "Nothing overdue — nice." : "No follow-ups."}
        onRowClick={(f) => navigate(`/renewals/${f.caseId}`)}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      <FollowUpDialog open={edit !== null} onOpenChange={(o) => !o && setEdit(null)} caseId={edit?.caseId ?? ""} edit={edit} onSaved={async () => { await invalidate(); }} />
    </>
  );
}
