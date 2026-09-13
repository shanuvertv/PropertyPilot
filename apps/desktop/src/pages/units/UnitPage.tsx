import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Pencil, Plus } from "lucide-react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";

import type { Expense, Occupant, OccupantInput } from "@/api/types-domain";
import { ExpiryChip, RenewalStatusBadge, UnitStatusBadge } from "@/components/badges";
import { HorizontalBars, MonthlyTrend, categoryColor } from "@/components/charts";
import { DataTable, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { errorMessage, FormDialog, selectClass, TextAreaField, TextField } from "@/components/forms";
import { SearchBox } from "@/components/SearchBox";
import { useLocalFilter } from "@/lib/local-filter";
import { HistoryPanel } from "@/components/HistoryPanel";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, formatDate, formatMoney, formatMonth, keys, todayIso } from "@/lib/format";
import { ExpenseDialog } from "@/pages/expenses/ExpenseDialog";
import { SplitBadge } from "@/pages/expenses/ExpensesPage";
import { UnitDialog } from "./UnitDialog";

export function UnitPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") ?? "occupants";
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
        <CardContent className="grid grid-cols-2 gap-x-6 gap-y-3 px-5 text-[13.5px] md:grid-cols-4">
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
        </CardContent>
      </Card>

      <Tabs value={tab} onValueChange={(v) => setParams({ tab: String(v) })}>
        <TabsList>
          <TabsTrigger value="occupants">Occupants</TabsTrigger>
          {can("VIEW_EXPENSES") && <TabsTrigger value="expenses">Expenses</TabsTrigger>}
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
        <TabsContent value="occupants" className="pt-4">
          <OccupantsTab unitId={id} />
        </TabsContent>
        {can("VIEW_EXPENSES") && (
          <TabsContent value="expenses" className="pt-4">
            <UnitExpensesTab unit={{ id, buildingId: u.buildingId, label: `${u.buildingName} · ${u.unitNumber}` }} />
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

// ---------------------------------------------------------------- occupants

const emptyOccupant = (): OccupantInput => ({ tenantId: null, fullName: "", idNumber: null, phone: null, email: null, bedLabel: null, moveIn: todayIso(), moveOut: null, notes: null });

function OccupantsTab({ unitId }: { unitId: string }) {
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const [showPast, setShowPast] = useState(false);
  const [dialog, setDialog] = useState<{ edit: Occupant | null } | null>(null);
  const [moveOut, setMoveOut] = useState<Occupant | null>(null);
  const [error, setError] = useState<string | null>(null);
  const manage = can("MANAGE_OCCUPANTS");
  const list = useQuery({ queryKey: ["occupants", unitId, showPast], queryFn: () => api.occupants(unitId, showPast) });
  const filter = useLocalFilter(list.data, (o) => [o.fullName, o.bedLabel, o.tenantName, o.phone, o.idNumber, o.email]);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["occupants", unitId] });

  const reactivate = useMutation({
    mutationFn: (o: Occupant) => api.moveOutOccupant(o.id, null),
    onSuccess: refresh,
    onError: (e) => setError(errorMessage(e, "Could not update the occupant.")),
  });

  const [view, setView] = useViewMode("unit-occupants");
  const columns: Column<Occupant>[] = [
    { key: "name", header: "Name", card: "title", render: (o) => <span className="font-medium">{o.fullName}{!o.current && <Badge variant="outline" className="ml-2">Moved out</Badge>}</span> },
    { key: "bed", header: "Bed", card: "metric", render: (o) => o.bedLabel ?? "—" },
    { key: "tenant", header: "Company", card: "subtitle", render: (o) => o.tenantName ?? "—" },
    { key: "phone", header: "Phone", render: (o) => o.phone ?? "—" },
    { key: "idn", header: "ID number", render: (o) => o.idNumber ?? "—" },
    { key: "in", header: "Moved in", card: "metric", render: (o) => formatDate(o.moveIn) },
    { key: "out", header: "Moved out", card: "metric", render: (o) => (o.moveOut ? formatDate(o.moveOut) : "—") },
    {
      key: "actions",
      header: "",
      className: "text-right",
      render: (o) =>
        manage ? (
          <span className="inline-flex gap-1">
            <Button size="xs" variant="ghost" onClick={() => setDialog({ edit: o })}>Edit</Button>
            {o.current ? (
              <Button size="xs" variant="ghost" onClick={() => setMoveOut(o)}>Move out</Button>
            ) : (
              <Button size="xs" variant="ghost" onClick={() => reactivate.mutate(o)}>Reactivate</Button>
            )}
          </span>
        ) : null,
    },
  ];

  const current = (list.data ?? []).filter((o) => o.current).length;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-[13px] text-muted-foreground">
          {current} living here now. Bills split equally go to the people present on the bill date.
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <SearchBox value={filter.q} onChange={filter.setQ} placeholder="Search name, bed, phone or ID" className="max-w-[260px]" />
          <label className="flex items-center gap-2 text-[13px]">
            <input type="checkbox" checked={showPast} onChange={(e) => setShowPast(e.target.checked)} />
            Show past occupants
          </label>
          <ViewToggle value={view} onChange={setView} />
          {manage && (
            <Button size="sm" onClick={() => setDialog({ edit: null })}>
              <Plus data-icon="inline-start" />
              Add occupant
            </Button>
          )}
        </div>
      </div>
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      <DataTable view={view} columns={columns} rows={filter.filtered} rowKey={(o) => o.id} loading={list.isPending} error={list.isError ? errorMessage(list.error, "Could not load occupants.") : null} empty={filter.q ? "No occupant matches the search." : "Nobody is recorded in this unit yet."} />

      <OccupantDialog open={dialog !== null} onOpenChange={(o) => !o && setDialog(null)} unitId={unitId} edit={dialog?.edit ?? null} onSaved={refresh} />
      <MoveOutDialog occupant={moveOut} onOpenChange={(o) => !o && setMoveOut(null)} onSaved={refresh} />
    </div>
  );
}

function OccupantDialog({ open, onOpenChange, unitId, edit, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; unitId: string; edit: Occupant | null; onSaved: () => Promise<unknown> | unknown }) {
  const { api } = useApp();
  const [form, setForm] = useState<OccupantInput>(emptyOccupant());
  const key = `${open}-${edit?.id ?? "new"}`;
  const [seeded, setSeeded] = useState("");
  if (open && seeded !== key) {
    setSeeded(key);
    setForm(edit ? { tenantId: edit.tenantId, fullName: edit.fullName, idNumber: edit.idNumber, phone: edit.phone, email: edit.email, bedLabel: edit.bedLabel, moveIn: edit.moveIn, moveOut: edit.moveOut, notes: edit.notes } : emptyOccupant());
  }
  const set = (p: Partial<OccupantInput>) => setForm((f) => ({ ...f, ...p }));
  const s = (v: string) => (v.trim() ? v.trim() : null);
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={edit ? "Edit occupant" : "Add occupant"}
      description="A person living in this unit. The company defaults to the unit's current tenant."
      submitLabel={edit ? "Save" : "Add occupant"}
      onSubmit={async () => {
        if (edit) await api.updateOccupant(edit.id, form);
        else await api.createOccupant(unitId, form);
        await onSaved();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="o-name" label="Full name" value={form.fullName} onChange={(v) => set({ fullName: v })} required autoFocus className="sm:col-span-2" />
        <TextField id="o-bed" label="Bed / room" value={form.bedLabel ?? ""} onChange={(v) => set({ bedLabel: s(v) })} placeholder="e.g. A, 3, upper" />
        <TextField id="o-id" label="ID / passport number" value={form.idNumber ?? ""} onChange={(v) => set({ idNumber: s(v) })} />
        <TextField id="o-phone" label="Phone" value={form.phone ?? ""} onChange={(v) => set({ phone: s(v) })} />
        <TextField id="o-email" label="Email" type="email" value={form.email ?? ""} onChange={(v) => set({ email: s(v) })} />
        <TextField id="o-in" label="Moved in" type="date" value={form.moveIn} onChange={(v) => set({ moveIn: v })} required />
        <TextField id="o-out" label="Moved out" type="date" value={form.moveOut ?? ""} onChange={(v) => set({ moveOut: v || null })} hint="Leave empty while they live here." />
        <TextAreaField id="o-notes" label="Notes" value={form.notes ?? ""} onChange={(v) => set({ notes: s(v) })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}

function MoveOutDialog({ occupant, onOpenChange, onSaved }: { occupant: Occupant | null; onOpenChange: (o: boolean) => void; onSaved: () => Promise<unknown> | unknown }) {
  const { api } = useApp();
  const [date, setDate] = useState(todayIso());
  return (
    <FormDialog
      open={occupant !== null}
      onOpenChange={onOpenChange}
      title={occupant ? `Move out ${occupant.fullName}` : "Move out"}
      description="Bills dated after this day are no longer split with them; their past shares stay."
      submitLabel="Record move-out"
      onSubmit={async () => {
        if (!occupant) return;
        await api.moveOutOccupant(occupant.id, date);
        await onSaved();
      }}
    >
      <TextField id="mo-date" label="Move-out date" type="date" value={date} onChange={setDate} required />
    </FormDialog>
  );
}

// ---------------------------------------------------------------- expenses

function UnitExpensesTab({ unit }: { unit: { id: string; buildingId: string; label: string } }) {
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
            {summary.data && summary.data.outstanding > 0 ? `${formatMoney(summary.data.outstanding)} still to be collected.` : "All shares settled."}
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
