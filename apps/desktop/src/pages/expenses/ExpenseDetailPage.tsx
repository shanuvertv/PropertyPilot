import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useParams } from "react-router";

import type { ExpenseDetail } from "@/api/types-domain";
import { categoryColor } from "@/components/charts";
import { DocumentsPanel } from "@/components/DocumentsPanel";
import { errorMessage, FormDialog } from "@/components/forms";
import { HistoryPanel } from "@/components/HistoryPanel";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, SPLIT_METHOD_LABEL, formatDate, formatDateTime, formatMoney } from "@/lib/format";
import { ExpenseDialog } from "./ExpenseDialog";

export function ExpenseDetailPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const detail = useQuery({ queryKey: ["expenses", "detail", id], queryFn: () => api.expense(id) });
  const [edit, setEdit] = useState(false);
  const [custom, setCustom] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const manage = can("MANAGE_EXPENSES");

  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["expenses"] });
  };

  const splitEqual = useMutation({
    mutationFn: () => api.splitExpenseEqually(id),
    onSuccess: async (d) => {
      queryClient.setQueryData(["expenses", "detail", id], d);
      setError(null);
      await refresh();
    },
    onError: (e) => setError(errorMessage(e, "Could not split the expense.")),
  });
  const settle = useMutation({
    mutationFn: ({ occupantId, settled }: { occupantId: string; settled: boolean }) => api.settleShare(id, occupantId, settled),
    onSuccess: async (d) => {
      queryClient.setQueryData(["expenses", "detail", id], d);
      await refresh();
    },
    onError: (e) => setError(errorMessage(e, "Could not update the share.")),
  });

  if (detail.isPending) return <p className="text-[13px] text-muted-foreground">Loading…</p>;
  if (detail.isError || !detail.data) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Expense not found.")}</AlertDescription>
      </Alert>
    );
  }
  const { expense: e, shares, occupants } = detail.data;
  const settledTotal = shares.filter((s) => s.settledAt).reduce((a, s) => a + s.amount, 0);
  const unsettled = shares.reduce((a, s) => a + s.amount, 0) - settledTotal;

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
              <CardTitle>Split between occupants</CardTitle>
              <CardDescription>
                {SPLIT_METHOD_LABEL[e.splitMethod]}
                {shares.length > 0 && ` · ${formatMoney(settledTotal)} paid, ${formatMoney(unsettled)} outstanding`}
              </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-3">
              {shares.length === 0 ? (
                <p className="text-[13px] text-muted-foreground">
                  {occupants.length === 0
                    ? "No occupants are recorded for this unit on the expense date, so there is nobody to split the bill between."
                    : "This cost is not split. Split it equally, or enter each person's share."}
                </p>
              ) : (
                <div className="overflow-x-auto rounded-md border">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Occupant</TableHead>
                        <TableHead>Bed</TableHead>
                        <TableHead className="text-right">Share</TableHead>
                        <TableHead>Paid</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {shares.map((s) => (
                        <TableRow key={s.occupantId}>
                          <TableCell className="font-medium">{s.occupantName}</TableCell>
                          <TableCell className="text-muted-foreground">{s.bedLabel ?? "—"}</TableCell>
                          <TableCell className="text-right tabular-nums">{formatMoney(s.amount)}</TableCell>
                          <TableCell>
                            <label className="flex items-center gap-2 text-[12.5px]">
                              <Checkbox
                                checked={!!s.settledAt}
                                disabled={!manage || settle.isPending}
                                onCheckedChange={(c) => settle.mutate({ occupantId: s.occupantId, settled: c === true })}
                                aria-label={`${s.occupantName} paid`}
                              />
                              <span className="text-muted-foreground">{s.settledAt ? formatDateTime(s.settledAt) : "not yet"}</span>
                            </label>
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>
              )}
              {manage && occupants.length > 0 && (
                <div className="flex flex-wrap gap-2">
                  <Button size="sm" variant="outline" onClick={() => splitEqual.mutate()} disabled={splitEqual.isPending}>
                    Split equally between {occupants.length}
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => setCustom(true)}>
                    Enter amounts…
                  </Button>
                </div>
              )}
              {occupants.length > 0 && (
                <p className="text-[12px] text-muted-foreground">
                  Living here on {formatDate(e.expenseDate)}: {occupants.map((o) => o.fullName).join(", ")}.{" "}
                  <Link to={`/units/${e.unitId}`} className="text-primary hover:underline">Manage occupants →</Link>
                </p>
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
                  {shares.length === 0 ? <Badge variant="outline">Unit cost</Badge> : e.settledCount === e.shareCount ? <Badge variant="secondary">All shares paid</Badge> : <Badge variant="outline">{e.settledCount}/{e.shareCount} paid</Badge>}
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
      <CustomSplitDialog open={custom} onOpenChange={setCustom} detail={detail.data} onSaved={(d) => { queryClient.setQueryData(["expenses", "detail", id], d); void refresh(); }} />
      <FormDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Delete this expense?"
        description="The expense and its shares are removed; attached bills are deleted with it."
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

/** Enter each occupant's share by hand; the amounts must add up to the bill. */
function CustomSplitDialog({ open, onOpenChange, detail, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; detail: ExpenseDetail; onSaved: (d: ExpenseDetail) => void }) {
  const { api } = useApp();
  const [amounts, setAmounts] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!open) return;
    const initial: Record<string, string> = {};
    for (const o of detail.occupants) {
      const existing = detail.shares.find((s) => s.occupantId === o.id);
      initial[o.id] = existing ? existing.amount.toFixed(2) : "";
    }
    setAmounts(initial);
  }, [open, detail]);
  const total = detail.expense.amount;
  const sum = Object.values(amounts).reduce((a, v) => a + (Number(v) || 0), 0);
  const remaining = Math.round((total - sum) * 100) / 100;

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Enter each person's share"
      description={`The shares must add up to ${formatMoney(total)}. Leave someone at 0 if they do not pay for this bill.`}
      submitLabel="Save shares"
      onSubmit={async () => {
        if (Math.abs(remaining) >= 0.005) throw new Error(`The shares add up to ${formatMoney(sum)}; ${formatMoney(remaining)} is still unassigned.`);
        const d = await api.setExpenseShares(
          detail.expense.id,
          detail.occupants.map((o) => ({ occupantId: o.id, amount: Math.round((Number(amounts[o.id]) || 0) * 100) / 100 })),
        );
        onSaved(d);
      }}
    >
      <div className="flex flex-col gap-2">
        {detail.occupants.map((o) => (
          <label key={o.id} className="grid grid-cols-[minmax(0,1fr)_120px] items-center gap-3 text-[13px]">
            <span className="truncate">
              {o.fullName}
              {o.bedLabel && <span className="text-muted-foreground"> · bed {o.bedLabel}</span>}
            </span>
            <Input type="number" step="0.01" min={0} value={amounts[o.id] ?? ""} onChange={(e) => setAmounts({ ...amounts, [o.id]: e.target.value })} className="text-right tabular-nums" />
          </label>
        ))}
        <div className={`mt-1 text-right text-[12.5px] tabular-nums ${Math.abs(remaining) < 0.005 ? "text-muted-foreground" : "text-destructive"}`}>
          {Math.abs(remaining) < 0.005 ? "Adds up." : `${formatMoney(remaining)} unassigned`}
        </div>
      </div>
    </FormDialog>
  );
}
