import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Minus, Plus } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router";

import { categoryColor } from "@/components/charts";
import { DocumentsPanel } from "@/components/DocumentsPanel";
import { errorMessage, FormDialog } from "@/components/forms";
import { HistoryPanel } from "@/components/HistoryPanel";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, formatDate, formatDateTime, formatMoney } from "@/lib/format";
import { ExpenseDialog } from "./ExpenseDialog";

export function ExpenseDetailPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["expenses", "detail", id], queryFn: () => api.expense(id) });
  const unit = useQuery({ queryKey: ["units", "detail", detail.data?.expense.unitId], queryFn: () => api.getUnit(detail.data!.expense.unitId), enabled: !!detail.data });
  const [edit, setEdit] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [splitCount, setSplitCount] = useState("");
  const [error, setError] = useState<string | null>(null);
  const manage = can("MANAGE_EXPENSES");

  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["expenses"] });
  };

  const splitEqual = useMutation({
    mutationFn: (count?: number) => api.splitExpenseEqually(id, count),
    onSuccess: async (d) => {
      queryClient.setQueryData(["expenses", "detail", id], d);
      setError(null);
      setSplitCount("");
      await refresh();
    },
    onError: (e) => setError(errorMessage(e, "Could not split the expense.")),
  });
  const settle = useMutation({
    mutationFn: (settledCount: number) => api.setExpenseSettled(id, settledCount),
    onSuccess: async (d) => {
      queryClient.setQueryData(["expenses", "detail", id], d);
      setError(null);
      await refresh();
    },
    onError: (e) => setError(errorMessage(e, "Could not update the payments.")),
  });

  if (detail.isPending) return <p className="text-[13px] text-muted-foreground">Loading…</p>;
  if (detail.isError || !detail.data) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Expense not found.")}</AlertDescription>
      </Alert>
    );
  }
  const { expense: e, shares } = detail.data;
  const split = e.splitMethod === "EQUAL" && shares.length > 0;
  const settledTotal = shares.filter((s) => s.settled).reduce((a, s) => a + s.amount, 0);
  const unsettled = e.amount - settledTotal;
  // Equal shares; when the bill does not divide exactly the first few carry one extra fils.
  const perPerson = shares[shares.length - 1]?.amount ?? 0;
  const roundedUp = shares.filter((sh) => sh.amount !== perPerson).length;
  const unitCount = unit.data?.occupantCount ?? 0;

  const requestSplit = () => {
    const n = splitCount.trim() ? Number(splitCount) : undefined;
    if (n !== undefined && (!Number.isInteger(n) || n < 1 || n > 500)) {
      setError("Enter a whole number of people between 1 and 500.");
      return;
    }
    splitEqual.mutate(n);
  };

  return (
    <>
      <PageHeader
        title={e.description}
        description={`${EXPENSE_CATEGORY_LABEL[e.category]} · ${formatDate(e.expenseDate)} · ${e.buildingName}, unit ${e.unitNumber}`}
        actions={
          manage && (
            <>
              <Button variant="outline" onClick={() => setEdit(true)}>Edit</Button>
              <Button variant="outline" className="text-destructive" onClick={() => setConfirmDelete(true)}>Delete</Button>
            </>
          )
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
        <div className="flex min-w-0 flex-col gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Split between the tenants</CardTitle>
              <CardDescription>
                {split
                  ? `${formatMoney(e.amount)} ÷ ${e.splitCount} = ${formatMoney(perPerson)} each · ${formatMoney(settledTotal)} collected, ${formatMoney(unsettled)} outstanding`
                  : "This is a unit cost — it is not divided between the tenants."}
              </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              {split && (
                <div className="flex flex-wrap items-center gap-3">
                  <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                    <div className="rounded-md bg-muted/60 px-3 py-2">
                      <div className="text-[10.5px] font-medium tracking-[0.06em] text-muted-foreground uppercase">Per person</div>
                      <div className="mt-0.5 text-[17px] font-semibold tabular-nums">{formatMoney(perPerson)}</div>
                    </div>
                    <div className="rounded-md bg-muted/60 px-3 py-2">
                      <div className="text-[10.5px] font-medium tracking-[0.06em] text-muted-foreground uppercase">Paid</div>
                      <div className="mt-0.5 text-[17px] font-semibold tabular-nums">
                        {e.settledCount} <span className="text-[13px] font-normal text-muted-foreground">of {e.splitCount}</span>
                      </div>
                    </div>
                    <div className="rounded-md bg-muted/60 px-3 py-2">
                      <div className="text-[10.5px] font-medium tracking-[0.06em] text-muted-foreground uppercase">Outstanding</div>
                      <div className="mt-0.5 text-[17px] font-semibold tabular-nums">{formatMoney(unsettled)}</div>
                    </div>
                  </div>
                  {manage && (
                    <div className="flex items-center gap-1" role="group" aria-label="People who have paid">
                      <Button variant="outline" size="icon-xs" aria-label="One fewer paid" disabled={settle.isPending || e.settledCount === 0} onClick={() => settle.mutate(e.settledCount - 1)}>
                        <Minus />
                      </Button>
                      <span className="w-16 text-center text-[13px] tabular-nums">{e.settledCount} paid</span>
                      <Button variant="outline" size="icon-xs" aria-label="One more paid" disabled={settle.isPending || e.settledCount >= e.splitCount} onClick={() => settle.mutate(e.settledCount + 1)}>
                        <Plus />
                      </Button>
                      <Button variant="ghost" size="sm" disabled={settle.isPending || e.settledCount >= e.splitCount} onClick={() => settle.mutate(e.splitCount)}>
                        All paid
                      </Button>
                    </div>
                  )}
                </div>
              )}
              {split && roundedUp > 0 && (
                <p className="text-[12px] text-muted-foreground">
                  Rounding: {roundedUp} of the {e.splitCount} shares are {formatMoney(shares[0].amount)} so the total adds up exactly.
                </p>
              )}
              {manage && (
                <div className="flex flex-wrap items-end gap-2">
                  <label className="flex flex-col gap-1 text-[12.5px] text-muted-foreground">
                    Split between
                    <Input
                      type="number"
                      min={1}
                      max={500}
                      className="h-8 w-28"
                      value={splitCount}
                      placeholder={unitCount > 0 ? `${unitCount} (unit)` : "people"}
                      onChange={(ev) => setSplitCount(ev.target.value)}
                      aria-label="Number of people to split between"
                    />
                  </label>
                  <Button size="sm" variant="outline" onClick={requestSplit} disabled={splitEqual.isPending || (!splitCount.trim() && unitCount === 0)}>
                    {split ? "Re-split equally" : "Split equally"}
                  </Button>
                  {unitCount === 0 && !splitCount.trim() && (
                    <span className="text-[12px] text-muted-foreground">
                      The unit has no tenants recorded — <Link to={`/units/${e.unitId}`} className="text-primary hover:underline">set the number on the unit</Link> or type it here.
                    </span>
                  )}
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Bills & receipts</CardTitle>
            </CardHeader>
            <CardContent>
              <DocumentsPanel entityType="expense" entityId={e.id} canManage={manage} />
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>History</CardTitle>
            </CardHeader>
            <CardContent>
              <HistoryPanel entityType="expense" entityId={e.id} />
            </CardContent>
          </Card>
        </div>

        <div className="flex min-w-0 flex-col gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Details</CardTitle>
            </CardHeader>
            <CardContent>
              <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[13px]">
                <dt className="text-muted-foreground">Amount</dt>
                <dd className="text-[16px] font-semibold tabular-nums">{formatMoney(e.amount)}</dd>
                <dt className="text-muted-foreground">Category</dt>
                <dd className="inline-flex items-center gap-1.5"><span className="inline-block size-2 rounded-full" style={{ background: categoryColor(e.category) }} aria-hidden="true" />{EXPENSE_CATEGORY_LABEL[e.category]}</dd>
                <dt className="text-muted-foreground">Unit</dt>
                <dd><Link to={`/units/${e.unitId}`} className="text-primary hover:underline">{e.buildingName} · {e.unitNumber}</Link></dd>
                <dt className="text-muted-foreground">Tenants in unit</dt>
                <dd className="tabular-nums">{unit.data ? unit.data.occupantCount : "—"}</dd>
                <dt className="text-muted-foreground">Date</dt>
                <dd>{formatDate(e.expenseDate)}</dd>
                {(e.periodStart || e.periodEnd) && (
                  <>
                    <dt className="text-muted-foreground">Billing period</dt>
                    <dd>{formatDate(e.periodStart)} – {formatDate(e.periodEnd)}</dd>
                  </>
                )}
                {e.vendor && (
                  <>
                    <dt className="text-muted-foreground">Vendor</dt>
                    <dd>{e.vendor}</dd>
                  </>
                )}
                {e.reference && (
                  <>
                    <dt className="text-muted-foreground">Reference</dt>
                    <dd className="font-mono text-[12.5px]">{e.reference}</dd>
                  </>
                )}
                <dt className="text-muted-foreground">Status</dt>
                <dd>
                  {!split ? <Badge variant="outline">Unit cost</Badge> : e.settledCount === e.splitCount ? <Badge variant="secondary">All paid</Badge> : <Badge variant="outline">{e.settledCount}/{e.splitCount} paid</Badge>}
                </dd>
                <dt className="text-muted-foreground">Recorded by</dt>
                <dd>{e.createdByName ?? "—"} · {formatDateTime(e.createdAt)}</dd>
                {e.notes && (
                  <>
                    <dt className="text-muted-foreground">Notes</dt>
                    <dd className="whitespace-pre-wrap">{e.notes}</dd>
                  </>
                )}
              </dl>
            </CardContent>
          </Card>
        </div>
      </div>

      <ExpenseDialog open={edit} onOpenChange={setEdit} edit={e} onSaved={refresh} />
      <FormDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Delete this expense?"
        description="The expense is removed; attached bills are deleted with it."
        submitLabel="Delete expense"
        destructive
        onSubmit={async () => {
          await api.deleteExpense(id);
          await refresh();
          navigate("/expenses");
        }}
      >
        <span className="sr-only">Confirm</span>
      </FormDialog>
    </>
  );
}
