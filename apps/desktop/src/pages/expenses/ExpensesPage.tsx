import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { Link, useNavigate } from "react-router";

import type { Expense, ExpenseCategory } from "@/api/types-domain";
import { HorizontalBars, MonthlyTrend, categoryColor } from "@/components/charts";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { errorMessage, selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, formatDate, formatMoney, formatMonth, keys } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions } from "@/lib/queries";
import { ExpenseDialog } from "./ExpenseDialog";

type Period = "12m" | "ytd" | "3m" | "all";

function periodRange(p: Period): { from?: string; to?: string } {
  const now = new Date();
  const iso = (d: Date) => d.toISOString().slice(0, 10);
  if (p === "ytd") return { from: `${now.getFullYear()}-01-01`, to: iso(now) };
  if (p === "3m") return { from: iso(new Date(now.getFullYear(), now.getMonth() - 2, 1)), to: iso(now) };
  if (p === "all") return { from: "2000-01-01", to: iso(now) };
  return {};
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <Card className="gap-1 py-4">
      <CardContent className="px-4">
        <div className="text-[12.5px] text-muted-foreground">{label}</div>
        <div className="mt-1 text-[18px] font-semibold tracking-tight tabular-nums md:text-[22px]">{value}</div>
        {hint && <div className="text-[12px] text-muted-foreground">{hint}</div>}
      </CardContent>
    </Card>
  );
}

export function SplitBadge({ e }: { e: Expense }) {
  if (e.splitMethod === "NONE" || e.shareCount === 0) return <Badge variant="outline">Unit cost</Badge>;
  const done = e.settledCount === e.shareCount;
  return <Badge variant={done ? "secondary" : "outline"}>{done ? `Settled ${e.shareCount}/${e.shareCount}` : `${e.settledCount}/${e.shareCount} settled`}</Badge>;
}

export function ExpensesPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const buildings = useBuildingOptions();
  const { state, update, toggleSort } = useListParams({ sort: "expense_date", dir: "desc" });
  const [dialog, setDialog] = useState(false);
  const period = (state.filters.period as Period | undefined) ?? "12m";
  const buildingId = state.filters.buildingId;
  const range = periodRange(period);

  const summary = useQuery({
    queryKey: ["expenses", "summary", buildingId ?? "", period],
    queryFn: () => api.expenseSummary({ buildingId, ...range }),
  });
  const list = useQuery({
    queryKey: ["expenses", "list", state, range],
    queryFn: () =>
      api.expenses({
        q: state.q || undefined,
        page: state.page,
        pageSize: state.pageSize,
        sort: state.sort,
        dir: state.dir,
        buildingId,
        category: (state.filters.category as ExpenseCategory | undefined) ?? undefined,
        outstanding: state.filters.outstanding === "1" ? true : undefined,
        ...range,
      }),
  });

  const s = summary.data;
  const monthly = useMemo(() => (s?.monthly ?? []).map((m) => ({ label: formatMonth(m.month), amount: m.amount, count: m.expenseCount })), [s]);
  const byBuilding = useMemo(() => (s?.byBuilding ?? []).map((g) => ({ id: g.id, label: g.label, sublabel: g.sublabel, amount: g.amount, count: g.expenseCount })), [s]);
  const byUnit = useMemo(() => (s?.byUnit ?? []).map((g) => ({ id: g.id, label: `${g.label}`, sublabel: g.sublabel, amount: g.amount, count: g.expenseCount })), [s]);
  const byCategory = useMemo(
    () => (s?.byCategory ?? []).map((c) => ({ id: c.category, label: EXPENSE_CATEGORY_LABEL[c.category] ?? c.category, amount: c.amount, count: c.expenseCount, color: categoryColor(c.category) })),
    [s],
  );
  const delta = s && s.lastMonth > 0 ? ((s.thisMonth - s.lastMonth) / s.lastMonth) * 100 : null;

  const columns: Column<Expense>[] = [
    { key: "date", header: "Date", sort: "expense_date", render: (e) => <span className="tabular-nums whitespace-nowrap">{formatDate(e.expenseDate)}</span> },
    { key: "desc", header: "Description", card: "title", render: (e) => <span className="font-medium">{e.description}</span> },
    { key: "unit", header: "Unit", sort: "unit", render: (e) => <Link to={`/units/${e.unitId}`} className="hover:underline" onClick={(ev) => ev.stopPropagation()}>{e.buildingName} · {e.unitNumber}</Link> },
    { key: "category", header: "Category", sort: "category", render: (e) => <span className="inline-flex items-center gap-1.5"><span className="inline-block size-2 rounded-full" style={{ background: categoryColor(e.category) }} aria-hidden="true" />{EXPENSE_CATEGORY_LABEL[e.category]}</span> },
    { key: "amount", header: "Amount", sort: "amount", className: "text-right", render: (e) => <span className="tabular-nums whitespace-nowrap">{formatMoney(e.amount)}</span> },
    { key: "split", header: "Split", render: (e) => <SplitBadge e={e} /> },
  ];

  return (
    <>
      <PageHeader
        title="Expenses"
        description="Bills and costs per unit, split between the people living there. Totals by month, property and unit."
        actions={can("MANAGE_EXPENSES") && (
          <Button onClick={() => setDialog(true)}>
            <Plus data-icon="inline-start" />
            Add expense
          </Button>
        )}
      />

      <div className="mb-4 flex flex-wrap gap-2">
        <select className={`${selectClass} w-auto min-w-44`} value={buildingId ?? ""} onChange={(e) => update({ page: 1, filters: { buildingId: e.target.value || undefined } })} aria-label="Building">
          <option value="">All properties</option>
          {(buildings.data ?? []).map((b) => (
            <option key={b.id} value={b.id}>{b.name}</option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={period} onChange={(e) => update({ page: 1, filters: { period: e.target.value === "12m" ? undefined : e.target.value } })} aria-label="Period">
          <option value="12m">Last 12 months</option>
          <option value="ytd">This year</option>
          <option value="3m">Last 3 months</option>
          <option value="all">All time</option>
        </select>
      </div>

      {summary.isError && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{errorMessage(summary.error, "Could not load the expenses summary.")}</AlertDescription>
        </Alert>
      )}

      <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-4">
        <Stat label={period === "12m" ? "Last 12 months" : period === "ytd" ? "This year" : period === "3m" ? "Last 3 months" : "All time"} value={formatMoney(s?.total)} hint={s ? `${s.expenseCount} expenses` : undefined} />
        <Stat label="This month" value={formatMoney(s?.thisMonth)} hint={delta === null ? undefined : `${delta >= 0 ? "+" : ""}${delta.toFixed(0)}% vs last month`} />
        <Stat label="Last month" value={formatMoney(s?.lastMonth)} />
        <Stat label="Outstanding from occupants" value={formatMoney(s?.outstanding)} hint={s ? `${s.outstandingShares} unpaid shares` : undefined} />
      </div>

      <div className="mb-6 grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>By month</CardTitle>
            <CardDescription>Total expenses per month{buildingId ? " for this property" : ""}.</CardDescription>
          </CardHeader>
          <CardContent><MonthlyTrend data={monthly} /></CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>By category</CardTitle>
            <CardDescription>What the money went on.</CardDescription>
          </CardHeader>
          <CardContent><HorizontalBars data={byCategory} onClick={(d) => update({ page: 1, filters: { category: d.id } })} /></CardContent>
        </Card>
        {!buildingId && (
          <Card>
            <CardHeader>
              <CardTitle>By property</CardTitle>
              <CardDescription>Click a bar to focus on that property.</CardDescription>
            </CardHeader>
            <CardContent><HorizontalBars data={byBuilding} onClick={(d) => update({ page: 1, filters: { buildingId: d.id } })} /></CardContent>
          </Card>
        )}
        <Card>
          <CardHeader>
            <CardTitle>Top units</CardTitle>
            <CardDescription>Units with the highest expenses in the period. Click a bar to open the unit.</CardDescription>
          </CardHeader>
          <CardContent><HorizontalBars data={byUnit.map((u) => ({ ...u, label: u.sublabel ? `${u.label} · ${u.sublabel}` : u.label }))} onClick={(d) => navigate(`/units/${d.id}`)} /></CardContent>
        </Card>
      </div>

      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q, page: 1 })} placeholder="Search description, vendor, reference or unit" />
        <select className={`${selectClass} w-auto`} value={state.filters.category ?? ""} onChange={(e) => update({ page: 1, filters: { category: e.target.value || undefined } })} aria-label="Category">
          <option value="">All categories</option>
          {keys(EXPENSE_CATEGORY_LABEL).map((c) => (
            <option key={c} value={c}>{EXPENSE_CATEGORY_LABEL[c]}</option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-[13px]">
          <input type="checkbox" checked={state.filters.outstanding === "1"} onChange={(e) => update({ page: 1, filters: { outstanding: e.target.checked ? "1" : undefined } })} />
          Unpaid shares only
        </label>
      </div>
      <DataTable
        columns={columns}
        rows={list.data?.items}
        rowKey={(e) => e.id}
        loading={list.isPending}
        error={list.isError ? errorMessage(list.error, "Could not load expenses.") : null}
        empty="No expenses recorded for this selection."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(e) => navigate(`/expenses/${e.id}`)}
      />
      {list.data && <Paginator page={state.page} pageSize={state.pageSize} total={list.data.total} onPage={(p) => update({ page: p })} />}

      <ExpenseDialog
        open={dialog}
        onOpenChange={setDialog}
        onSaved={async (e) => {
          await queryClient.invalidateQueries({ queryKey: ["expenses"] });
          navigate(`/expenses/${e.id}`);
        }}
      />
    </>
  );
}
