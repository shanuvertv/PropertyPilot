import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router";

import type { Cheque, ChequeInput, ChequeStatus } from "@/api/types-domain";
import { type Column } from "@/components/DataTable";
import { FormDialog, TextAreaField, TextField, errorMessage } from "@/components/forms";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useApp } from "@/lib/app-state";
import { CHEQUE_STATUS_LABEL, formatDate, formatMoney, todayIso } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Status pill; a pending cheque past its date reads as overdue. */
export function ChequeStatusBadge({ cheque }: { cheque: Cheque }) {
  const overdue = cheque.status === "PENDING" && cheque.daysUntilDue < 0;
  const variant = overdue || cheque.status === "BOUNCED" ? "destructive" : cheque.status === "CLEARED" ? "default" : cheque.status === "DEPOSITED" ? "secondary" : "outline";
  return <Badge variant={variant}>{overdue ? `Overdue ${-cheque.daysUntilDue}d` : CHEQUE_STATUS_LABEL[cheque.status]}</Badge>;
}

/** Cheque date with how soon it is, coloured when it is close or past. */
export function DueCell({ cheque }: { cheque: Cheque }) {
  const d = cheque.daysUntilDue;
  const pending = cheque.status === "PENDING";
  const hint = !pending ? null : d < 0 ? `${-d} days ago` : d === 0 ? "today" : d === 1 ? "tomorrow" : `in ${d} days`;
  return (
    <span className={cn("tabular-nums whitespace-nowrap", pending && d < 0 && "text-destructive", pending && d >= 0 && d <= 7 && "font-medium")}>
      {formatDate(cheque.dueDate)}
      {hint && <span className="text-[12px] font-normal text-muted-foreground"> · {hint}</span>}
    </span>
  );
}

/** Deposit / clear / bounce actions; `manage` false shows nothing. */
export function ChequeActions({ cheque, manage, onEdit, onChanged }: { cheque: Cheque; manage: boolean; onEdit?: (c: Cheque) => void; onChanged?: () => void }) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const setStatus = useMutation({
    mutationFn: (status: ChequeStatus) => api.setChequeStatus(cheque.id, status),
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["cheques"] });
      await queryClient.invalidateQueries({ queryKey: ["notifications"] });
      onChanged?.();
    },
    onError: (e) => setError(errorMessage(e, "Could not update the cheque.")),
  });
  if (!manage) return null;
  const busy = setStatus.isPending;
  return (
    <span className="inline-flex flex-wrap items-center justify-end gap-1" onClick={(e) => e.stopPropagation()}>
      {cheque.status === "PENDING" && (
        <Button size="xs" variant="outline" disabled={busy} onClick={() => setStatus.mutate("DEPOSITED")}>
          Deposited
        </Button>
      )}
      {cheque.status === "DEPOSITED" && (
        <>
          <Button size="xs" variant="outline" disabled={busy} onClick={() => setStatus.mutate("CLEARED")}>
            Cleared
          </Button>
          <Button size="xs" variant="ghost" className="text-destructive" disabled={busy} onClick={() => setStatus.mutate("BOUNCED")}>
            Bounced
          </Button>
        </>
      )}
      {(cheque.status === "BOUNCED" || cheque.status === "CANCELLED" || cheque.status === "CLEARED") && (
        <Button size="xs" variant="ghost" disabled={busy} onClick={() => setStatus.mutate("PENDING")}>
          Back to pending
        </Button>
      )}
      {cheque.status === "PENDING" && (
        <Button size="xs" variant="ghost" disabled={busy} onClick={() => setStatus.mutate("CANCELLED")}>
          Cancel
        </Button>
      )}
      {onEdit && (
        <Button size="xs" variant="ghost" onClick={() => onEdit(cheque)}>
          Edit
        </Button>
      )}
      {error && <span className="text-[12px] text-destructive">{error}</span>}
    </span>
  );
}

/** Columns shared by the contract tab, the Cheques page and the dashboard. */
export function chequeColumns(opts: { manage: boolean; showContract: boolean; onEdit?: (c: Cheque) => void; onChanged?: () => void }): Column<Cheque>[] {
  const cols: Column<Cheque>[] = [
    { key: "seq", header: "#", className: "w-10 text-right tabular-nums", card: "hidden", render: (c) => c.seq },
    { key: "number", header: "Cheque no.", card: "title", render: (c) => <span className="font-medium">{c.chequeNumber ?? `Cheque ${c.seq}`}</span> },
  ];
  if (opts.showContract) {
    cols.push({
      key: "contract",
      header: "Tenant · contract",
      card: "subtitle",
      render: (c) => (
        <span>
          <Link to={`/tenants/${c.tenantId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{c.tenantName}</Link>
          <span className="text-muted-foreground"> · </span>
          <Link to={`/contracts/${c.contractId}?tab=cheques`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{c.contractNumber}</Link>
          <span className="text-muted-foreground"> · {c.buildingName} {c.unitNumbers}</span>
        </span>
      ),
    });
  }
  cols.push(
    { key: "bank", header: "Bank", render: (c) => c.bankName ?? "—" },
    { key: "amount", header: "Amount", sort: "amount", className: "text-right", card: "metric", render: (c) => <span className="tabular-nums whitespace-nowrap">{formatMoney(c.amount)}</span> },
    { key: "due", header: "Cheque date", sort: "due_date", card: "metric", render: (c) => <DueCell cheque={c} /> },
    { key: "status", header: "Status", sort: "status", card: "badge", render: (c) => <ChequeStatusBadge cheque={c} /> },
  );
  if (opts.manage) {
    cols.push({ key: "actions", header: "", className: "text-right whitespace-nowrap", render: (c) => <ChequeActions cheque={c} manage onEdit={opts.onEdit} onChanged={opts.onChanged} /> });
  }
  return cols;
}

// ---------------------------------------------------------------- dialogs

const opt = (s: string) => (s.trim() ? s.trim() : null);

/** Add one cheque to a contract, or edit an existing one. */
export function ChequeDialog({ open, onOpenChange, contractId, edit, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; contractId: string; edit?: Cheque | null; onSaved: () => Promise<unknown> | unknown }) {
  const { api } = useApp();
  const [form, setForm] = useState({ chequeNumber: "", bankName: "", amount: "", dueDate: todayIso(), notes: "" });
  useEffect(() => {
    if (open) setForm(edit ? { chequeNumber: edit.chequeNumber ?? "", bankName: edit.bankName ?? "", amount: edit.amount.toFixed(2), dueDate: edit.dueDate, notes: edit.notes ?? "" } : { chequeNumber: "", bankName: "", amount: "", dueDate: todayIso(), notes: "" });
  }, [open, edit]);
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={edit ? `Edit cheque ${edit.chequeNumber ?? edit.seq}` : "Add cheque"}
      submitLabel={edit ? "Save changes" : "Add cheque"}
      onSubmit={async () => {
        const amount = Number(form.amount);
        if (!Number.isFinite(amount) || amount <= 0) throw new Error("Enter the cheque amount (greater than zero).");
        const input: ChequeInput = { chequeNumber: opt(form.chequeNumber), bankName: opt(form.bankName), amount: Math.round(amount * 100) / 100, dueDate: form.dueDate, notes: opt(form.notes) };
        if (edit) await api.updateCheque(edit.id, input);
        else await api.createCheque(contractId, input);
        await onSaved();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="q-number" label="Cheque number" value={form.chequeNumber} onChange={(v) => setForm({ ...form, chequeNumber: v })} placeholder="e.g. 000123" autoFocus />
        <TextField id="q-bank" label="Bank" value={form.bankName} onChange={(v) => setForm({ ...form, bankName: v })} placeholder="e.g. Emirates NBD" />
        <TextField id="q-amount" label="Amount (AED)" type="number" value={form.amount} onChange={(v) => setForm({ ...form, amount: v })} required />
        <TextField id="q-date" label="Cheque date" type="date" value={form.dueDate} onChange={(v) => setForm({ ...form, dueDate: v })} required hint="The date on the cheque — the day it should be deposited." />
        <TextAreaField id="q-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}

/** Split the rent into N cheques spread over the contract; each can be edited afterwards. */
export function GenerateChequesDialog({ open, onOpenChange, contractId, rentAmount, startDate, hasPending, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; contractId: string; rentAmount: number | null; startDate: string; hasPending: boolean; onSaved: () => Promise<unknown> | unknown }) {
  const { api } = useApp();
  const [form, setForm] = useState({ count: "4", firstDate: startDate, everyMonths: "", total: "", bankName: "", replacePending: true });
  useEffect(() => {
    if (open) setForm({ count: "4", firstDate: startDate, everyMonths: "", total: rentAmount === null ? "" : rentAmount.toFixed(2), bankName: "", replacePending: true });
  }, [open, startDate, rentAmount]);
  const count = Number(form.count) || 0;
  const total = Number(form.total) || 0;
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Set up cheques"
      description="Splits the rent into equal cheques (any rounding goes on the first ones). You can change each cheque's number, amount and date afterwards."
      submitLabel={count > 0 ? `Create ${count} cheque${count === 1 ? "" : "s"}` : "Create cheques"}
      onSubmit={async () => {
        if (!Number.isInteger(count) || count < 1 || count > 60) throw new Error("Enter between 1 and 60 cheques.");
        if (!(total > 0)) throw new Error("Enter the total amount to split (the contract has no rent recorded).");
        const every = form.everyMonths.trim() ? Number(form.everyMonths) : null;
        if (every !== null && (!Number.isInteger(every) || every < 0 || every > 24)) throw new Error("Months between cheques must be between 0 and 24.");
        await api.generateCheques(contractId, { count, firstDate: form.firstDate || null, everyMonths: every, total: Math.round(total * 100) / 100, bankName: opt(form.bankName), replacePending: form.replacePending });
        await onSaved();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="g-count" label="Number of cheques" type="number" value={form.count} onChange={(v) => setForm({ ...form, count: v })} required autoFocus />
        <TextField id="g-total" label="Total amount (AED)" type="number" value={form.total} onChange={(v) => setForm({ ...form, total: v })} required hint={rentAmount === null ? "The contract has no rent recorded — enter the total here." : `Contract rent: ${formatMoney(rentAmount)}.`} />
        <TextField id="g-first" label="First cheque date" type="date" value={form.firstDate} onChange={(v) => setForm({ ...form, firstDate: v })} required />
        <TextField id="g-every" label="Months between cheques" type="number" value={form.everyMonths} onChange={(v) => setForm({ ...form, everyMonths: v })} placeholder="auto" hint="Leave empty to spread them evenly over the contract (e.g. 4 cheques on a 1-year contract → every 3 months)." />
        <TextField id="g-bank" label="Bank (optional)" value={form.bankName} onChange={(v) => setForm({ ...form, bankName: v })} className="sm:col-span-2" />
        {count > 0 && total > 0 && (
          <p className="text-[12.5px] text-muted-foreground sm:col-span-2">
            {count} × about {formatMoney(Math.round((total / count) * 100) / 100)}
          </p>
        )}
        {hasPending && (
          <label className="flex items-center gap-2 text-[13px] sm:col-span-2">
            <Checkbox checked={form.replacePending} onCheckedChange={(c) => setForm({ ...form, replacePending: c === true })} />
            Replace the pending cheques already on this contract
          </label>
        )}
      </div>
    </FormDialog>
  );
}
