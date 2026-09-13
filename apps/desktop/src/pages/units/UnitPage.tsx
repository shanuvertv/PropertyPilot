import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Pencil, Plus } from "lucide-react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";

import type { Expense, UnitSummary } from "@/api/types-domain";
import { ExpiryChip, RenewalStatusBadge, UnitStatusBadge } from "@/components/badges";
import { HorizontalBars, MonthlyTrend, categoryColor } from "@/components/charts";
import { DataTable, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { errorMessage, selectClass } from "@/components/forms";
import { SearchBox } from "@/components/SearchBox";
import { useLocalFilter } from "@/lib/local-filter";
import { HistoryPanel } from "@/components/HistoryPanel";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, formatDate, formatMoney, formatMonth, keys } from "@/lib/format";
import { ExpenseDialog } from "@/pages/expenses/ExpenseDialog";
import { SplitBadge } from "@/pages/expenses/ExpensesPage";
import { UnitDialog } from "./UnitDialog";

export function UnitPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const [params, setParams] = useSearchParams();
  const canExpenses = can("VIEW_EXPENSES");
  const requested = params.get("tab");
  const tab = requested === "history" || (requested === "expenses" && canExpenses) ? requested : canExpenses ? "expenses" : "history";
  const unit = useQuery({ queryKey: ["units", "detail", id], queryFn: () => api.getUnit(id) });
  const [edit, setEdit] = useState(false);

  if (unit.isPending) return <p className="text-[13px] text-muted-foreground">Loading…</p>;
  if (unit.isError || !unit.data) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(unit.error, "Unit not found.")}</AlertDescription>
      </Alert>
    );
  }
  const u = unit.data;

  return (
    <>
      <PageHeader
        title={`Unit ${u.unitNumber}`}
        description={[u.buildingName, u.floor && `Floor ${u.floor}`, u.unitType].filter(Boolean).join(" · ")}
        actions={
          <>
            <UnitStatusBadge status={u.status} />
            {can("MANAGE_UNITS") && (
              <Button variant="outline" onClick={() => setEdit(true)}>
                <Pencil data-icon="inline-start" />
                Edit
              </Button>
            )}
          </>
        }
      />

      <Card className="mb-4">
        <CardContent className="grid grid-cols-2 gap-x-6 gap-y-3 px-5 text-[13.5px] md:grid-cols-3 xl:grid-cols-6">
          <div>
            <div className="text-[12px] text-muted-foreground">Tenant</div>
            <div>{u.tenantId ? <Link to={`/tenants/${u.tenantId}`} className="text-primary hover:underline">{u.tenantName}</Link> : "Vacant"}</div>
          </div>
          <div>
            <div className="text-[12px] text-muted-foreground">Contract</div>
            <div>{u.contractId ? can("VIEW_CONTRACTS") ? <Link to={`/contracts/${u.contractId}`} className="text-primary hover:underline">{u.contractNumber}</Link> : u.contractNumber : "—"}</div>
          </div>
          <div>
            <div className="text-[12px] text-muted-foreground">Contract ends</div>
            <div className="flex items-center gap-2">{formatDate(u.endDate)} {u.band && <ExpiryChip band={u.band} days={u.remainingDays} />}</div>
          </div>
          <div>
            <div className="text-[12px] text-muted-foreground">Renewal</div>
            <div>{u.caseId ? can("VIEW_RENEWALS") ? <Link to={`/renewals/${u.caseId}`} className="hover:underline"><RenewalStatusBadge status={u.renewalStatus} /></Link> : <RenewalStatusBadge status={u.renewalStatus} /> : <span className="text-muted-foreground">Not started</span>}</div>
          </div>
          <div>
            <div className="text-[12px] text-muted-foreground">Number of occupants</div>
            <TenantCount unit={u} />
          </div>
          <div>
            <div className="text-[12px] text-muted-foreground">Rent (contract)</div>
            <div className="tabular-nums">{u.rentAmount === null ? "—" : formatMoney(u.rentAmount)}</div>
          </div>
        </CardContent>
      </Card>

      <Tabs value={tab} onValueChange={(v) => setParams({ tab: String(v) })}>
        <TabsList>
          {canExpenses && <TabsTrigger value="expenses">Expenses</TabsTrigger>}
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
        {canExpenses && (
          <TabsContent value="expenses" className="pt-4">
            <UnitExpensesTab unit={{ id, buildingId: u.buildingId, label: `${u.buildingName} · ${u.unitNumber}`, occupantCount: u.occupantCount }} />
          </TabsContent>
        )}
        <TabsContent value="history" className="pt-4">
          <HistoryPanel entityType="unit" entityId={id} />
        </TabsContent>
      </Tabs>

      <UnitDialog open={edit} onOpenChange={setEdit} unit={u} onSaved={() => void queryClient.invalidateQueries({ queryKey: ["units"] })} />
    </>
  );
}

// ---------------------------------------------------------------- number of occupants

/** The number of occupants comes from the unit's active contract; it is edited there. */
function TenantCount({ unit }: { unit: UnitSummary }) {
  const { can } = useApp();
  if (!unit.contractId) return <span className="text-muted-foreground">— (no active contract)</span>;
  return (
    <div className="flex flex-wrap items-baseline gap-x-2">
      <span className="tabular-nums">{unit.occupantCount}</span>
      <span className="text-[12px] text-muted-foreground">
        {can("MANAGE_CONTRACTS") && can("VIEW_CONTRACTS") ? (
          <>
            set on <Link to={`/contracts/${unit.contractId}`} className="text-primary hover:underline">contract {unit.contractNumber}</Link>
          </>
        ) : (
          <>from contract {unit.contractNumber}</>
        )}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------- expenses

function UnitExpensesTab({ unit }: { unit: { id: string; buildingId: string; label: string; occupantCount: number } }) {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [dialog, setDialog] = useState(false);
  const summary = useQuery({ queryKey: ["expenses", "summary", "unit", unit.id], queryFn: () => api.expenseSummary({ unitId: unit.id }) });
  const list = useQuery({ queryKey: ["expenses", "unit", unit.id], queryFn: () => api.expenses({ unitId: unit.id, pageSize: 100, sort: "expense_date", dir: "desc" }) });
  const filter = useLocalFilter(list.data?.items, (e) => [e.description, EXPENSE_CATEGORY_LABEL[e.category], e.vendor, e.reference, e.expenseDate, e.amount]);
  const [category, setCategory] = useState("");
  const monthly = useMemo(() => (summary.data?.monthly ?? []).map((m) => ({ label: formatMonth(m.month), amount: m.amount, count: m.expenseCount })), [summary.data]);
  const byCategory = useMemo(() => (summary.data?.byCategory ?? []).map((c) => ({ id: c.category, label: EXPENSE_CATEGORY_LABEL[c.category], amount: c.amount, count: c.expenseCount, color: categoryColor(c.category) })), [summary.data]);

  const [view, setView] = useViewMode("unit-expenses");
  const columns: Column<Expense>[] = [
    { key: "date", header: "Date", card: "metric", render: (e) => <span className="tabular-nums whitespace-nowrap">{formatDate(e.expenseDate)}</span> },
    { key: "desc", header: "Description", card: "title", render: (e) => <span className="font-medium">{e.description}</span> },
    { key: "category", header: "Category", card: "badge", render: (e) => <span className="inline-flex items-center gap-1.5 text-[12.5px]"><span className="inline-block size-2 rounded-full" style={{ background: categoryColor(e.category) }} aria-hidden="true" />{EXPENSE_CATEGORY_LABEL[e.category]}</span> },
    { key: "amount", header: "Amount", className: "text-right", card: "metric", render: (e) => <span className="tabular-nums">{formatMoney(e.amount)}</span> },
    { key: "split", header: "Split", card: "badge", render: (e) => <SplitBadge e={e} /> },
  ];

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Last 12 months · {formatMoney(summary.data?.total)}</CardTitle>
          </CardHeader>
          <CardContent><MonthlyTrend data={monthly} /></CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>By category</CardTitle>
          </CardHeader>
          <CardContent><HorizontalBars data={byCategory} /></CardContent>
        </Card>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-2">
          <SearchBox value={filter.q} onChange={filter.setQ} placeholder="Search description, vendor or reference" className="max-w-[280px]" />
          <select className={`${selectClass} w-auto`} value={category} onChange={(e) => setCategory(e.target.value)} aria-label="Category">
            <option value="">All categories</option>
            {keys(EXPENSE_CATEGORY_LABEL).map((c) => (
              <option key={c} value={c}>{EXPENSE_CATEGORY_LABEL[c]}</option>
            ))}
          </select>
          <span className="text-[12.5px] text-muted-foreground">
            {summary.data && summary.data.outstanding > 0 ? `${formatMoney(summary.data.outstanding)} still to be collected from the occupants.` : "Nothing outstanding from the occupants."}
          </span>
          <ViewToggle value={view} onChange={setView} />
        </div>
        {can("MANAGE_EXPENSES") && (
          <Button size="sm" onClick={() => setDialog(true)}>
            <Plus data-icon="inline-start" />
            Add expense
          </Button>
        )}
      </div>
      <DataTable view={view} columns={columns} rows={filter.filtered?.filter((e) => !category || e.category === category)} rowKey={(e) => e.id} loading={list.isPending} error={list.isError ? errorMessage(list.error, "Could not load expenses.") : null} empty={filter.q || category ? "No expense matches the filters." : "No expenses for this unit yet."} onRowClick={(e) => navigate(`/expenses/${e.id}`)} />
      <ExpenseDialog
        open={dialog}
        onOpenChange={setDialog}
        unit={unit}
        onSaved={async (e) => {
          await queryClient.invalidateQueries({ queryKey: ["expenses"] });
          navigate(`/expenses/${e.id}`);
        }}
      />
    </div>
  );
}
