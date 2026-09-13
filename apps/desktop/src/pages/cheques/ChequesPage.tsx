import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "react-router";

import type { Cheque } from "@/api/types-domain";
import { DataTable, Paginator, useViewMode, ViewToggle } from "@/components/DataTable";
import { DateRange } from "@/components/DateRange";
import { selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";
import { CHEQUE_STATUS_LABEL, formatMoney, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { ChequeDialog, chequeColumns } from "./ChequeParts";

function Stat({ label, value, hint, tone, active, onClick }: { label: string; value: string; hint?: string; tone?: "danger" | "warn"; active?: boolean; onClick?: () => void }) {
  return (
    <button type="button" onClick={onClick} aria-pressed={active} className={cn("rounded-md border bg-card px-3.5 py-3 text-left transition-colors hover:bg-muted", active && "border-primary ring-2 ring-primary/20")}>
      <div className="text-[11px] font-medium tracking-[0.06em] text-muted-foreground uppercase">{label}</div>
      <div className={cn("mt-1 text-[20px] font-semibold tabular-nums", tone === "danger" && "text-destructive", tone === "warn" && "text-amber-600")}>{value}</div>
      {hint && <div className="text-[12px] text-muted-foreground">{hint}</div>}
    </button>
  );
}

/** Every rent cheque across contracts: what to deposit this week, what is overdue, what bounced. */
export function ChequesPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const buildings = useBuildingOptions();
  const { state, update, toggleSort } = useListParams({ sort: "due_date" });
  const [view, setView] = useViewMode("cheques");
  const [edit, setEdit] = useState<Cheque | null>(null);
  const manage = can("MANAGE_CONTRACTS");

  const summary = useQuery({ queryKey: ["cheques", "summary"], queryFn: () => api.chequeSummary() });
  const scope = state.filters.scope ?? "";
  const query = useQuery({
    queryKey: ["cheques", "list", state],
    queryFn: () =>
      api.cheques({
        q: state.q,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        buildingId: state.filters.buildingId,
        status: scope === "overdue" || scope === "week" || scope === "month" ? "PENDING" : state.filters.status,
        overdue: scope === "overdue" ? true : undefined,
        dueFrom: scope === "week" || scope === "month" ? todayPlus(0) : state.filters.from,
        dueTo: scope === "week" ? todayPlus(6) : scope === "month" ? todayPlus(29) : state.filters.to,
      }),
    placeholderData: (prev) => prev,
  });
  const s = summary.data;
  const setScope = (v: string) => update({ page: 1, filters: { scope: scope === v ? undefined : v, status: undefined, from: undefined, to: undefined } });
  const columns = chequeColumns({ manage, showContract: true, onEdit: (c) => setEdit(c) });

  return (
    <>
      <PageHeader
        title="Cheques"
        description={`Post-dated rent cheques and their deposit dates. A reminder goes to the assigned employee ${s ? `${s.reminderDays} day${s.reminderDays === 1 ? "" : "s"} before` : "before"} each cheque date, on the day, and once when it is overdue.`}
      />

      <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-4">
        <Stat label="Overdue" value={s ? String(s.overdueCount) : "—"} hint={s ? formatMoney(s.overdueAmount) : undefined} tone={s && s.overdueCount > 0 ? "danger" : undefined} active={scope === "overdue"} onClick={() => setScope("overdue")} />
        <Stat label="Due in 7 days" value={s ? String(s.due7Count) : "—"} hint={s ? formatMoney(s.due7Amount) : undefined} tone={s && s.due7Count > 0 ? "warn" : undefined} active={scope === "week"} onClick={() => setScope("week")} />
        <Stat label="Due in 30 days" value={s ? String(s.due30Count) : "—"} hint={s ? formatMoney(s.due30Amount) : undefined} active={scope === "month"} onClick={() => setScope("month")} />
        <Stat label="All pending" value={s ? String(s.pendingCount) : "—"} hint={s ? `${formatMoney(s.pendingAmount)}${s.bouncedCount ? ` · ${s.bouncedCount} bounced` : ""}` : undefined} active={scope === "pending"} onClick={() => update({ page: 1, filters: { scope: scope === "pending" ? undefined : "pending", status: scope === "pending" ? undefined : "PENDING" } })} />
      </div>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q, page: 1 })} placeholder="Search cheque number, bank, tenant or contract" />
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ page: 1, filters: { buildingId: e.target.value || undefined } })} aria-label="Building">
          <option value="">All properties</option>
          {(buildings.data ?? []).map((b) => (
            <option key={b.id} value={b.id}>{b.name}</option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={scope ? "" : (state.filters.status ?? "")} disabled={!!scope} onChange={(e) => update({ page: 1, filters: { status: e.target.value || undefined } })} aria-label="Status">
          <option value="">Any status</option>
          {keys(CHEQUE_STATUS_LABEL).map((st) => (
            <option key={st} value={st}>{CHEQUE_STATUS_LABEL[st]}</option>
          ))}
        </select>
        {!scope && <DateRange from={state.filters.from} to={state.filters.to} onChange={(from, to) => update({ page: 1, filters: { from, to } })} />}
        {(scope || state.q || state.filters.status || state.filters.buildingId || state.filters.from || state.filters.to) && (
          <Button variant="ghost" size="sm" onClick={() => update({ q: "", page: 1, filters: { scope: undefined, status: undefined, buildingId: undefined, from: undefined, to: undefined } })}>
            Clear
          </Button>
        )}
        <ViewToggle value={view} onChange={setView} className="ml-auto" />
      </div>

      {query.data && query.data.total === 0 && !state.q && !scope && !state.filters.status ? (
        <Card>
          <CardContent className="py-8 text-center text-[13.5px] text-muted-foreground">
            No cheques recorded yet. Open a contract → <b>Cheques</b> tab → <b>Set up cheques</b> to split its rent into post-dated cheques.
          </CardContent>
        </Card>
      ) : (
        <DataTable
          view={view}
          columns={columns}
          rows={query.data?.items}
          rowKey={(c) => c.id}
          loading={query.isPending}
          error={query.error ? "Could not load cheques." : null}
          empty="No cheques match these filters."
          sort={state.sort}
          dir={state.dir}
          onSort={toggleSort}
          onRowClick={(c) => navigate(`/contracts/${c.contractId}?tab=cheques`)}
        />
      )}
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      {edit && <ChequeDialog open onOpenChange={(o) => !o && setEdit(null)} contractId={edit.contractId} edit={edit} onSaved={() => { setEdit(null); void query.refetch(); void summary.refetch(); }} />}
    </>
  );
}

function todayPlus(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return d.toISOString().slice(0, 10);
}
