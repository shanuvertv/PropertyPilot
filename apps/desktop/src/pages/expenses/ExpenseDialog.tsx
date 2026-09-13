import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";

import type { Expense, ExpenseCategory, ExpenseInput, SplitMethod } from "@/api/types-domain";
import { FormDialog, SelectField, TextAreaField, TextField } from "@/components/forms";
import { useApp } from "@/lib/app-state";
import { EXPENSE_CATEGORY_LABEL, SPLIT_METHOD_LABEL, keys, todayIso } from "@/lib/format";
import { useBuildingOptions } from "@/lib/queries";

type Form = {
  buildingId: string;
  unitId: string;
  category: ExpenseCategory;
  description: string;
  amount: string;
  expenseDate: string;
  periodStart: string;
  periodEnd: string;
  vendor: string;
  reference: string;
  splitMethod: SplitMethod;
  /** People to split between; "" = the unit's number of tenants. */
  splitCount: string;
  notes: string;
};

function blank(unit?: { id: string; buildingId: string }): Form {
  return {
    buildingId: unit?.buildingId ?? "",
    unitId: unit?.id ?? "",
    category: "ELECTRICITY",
    description: "",
    amount: "",
    expenseDate: todayIso(),
    periodStart: "",
    periodEnd: "",
    vendor: "",
    reference: "",
    splitMethod: "EQUAL",
    splitCount: "",
    notes: "",
  };
}

function fromExpense(e: Expense): Form {
  return {
    buildingId: e.buildingId,
    unitId: e.unitId,
    category: e.category,
    description: e.description,
    amount: e.amount.toFixed(2),
    expenseDate: e.expenseDate,
    periodStart: e.periodStart ?? "",
    periodEnd: e.periodEnd ?? "",
    vendor: e.vendor ?? "",
    reference: e.reference ?? "",
    splitMethod: e.splitMethod,
    splitCount: e.splitCount > 0 ? String(e.splitCount) : "",
    notes: e.notes ?? "",
  };
}

const opt = (s: string) => (s.trim() ? s.trim() : null);

/** Add or edit a unit expense. `unit` pre-selects (and locks) the unit when opened from a unit page. */
export function ExpenseDialog({
  open,
  onOpenChange,
  edit,
  unit,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  edit?: Expense | null;
  unit?: { id: string; buildingId: string; label: string; occupantCount?: number } | null;
  onSaved: (e: Expense) => Promise<void> | void;
}) {
  const { api } = useApp();
  const buildings = useBuildingOptions();
  const [form, setForm] = useState<Form>(blank(unit ?? undefined));

  useEffect(() => {
    if (open) setForm(edit ? fromExpense(edit) : blank(unit ?? undefined));
  }, [open, edit, unit]);

  const units = useQuery({
    queryKey: ["units", "options", form.buildingId],
    queryFn: () => api.listUnits({ buildingId: form.buildingId, pageSize: 200, sort: "unit_number" }),
    enabled: open && !!form.buildingId && !unit,
  });

  // The unit's own head count, for the "split between N" default and hint.
  const chosen = unit ? { occupantCount: unit.occupantCount } : units.data?.items.find((u) => u.id === form.unitId);
  const unitCount = chosen?.occupantCount;

  async function submit() {
    const amount = Number(form.amount);
    if (!form.unitId) throw new Error("Choose the unit this expense belongs to.");
    if (!Number.isFinite(amount) || amount <= 0) throw new Error("Enter an amount greater than zero.");
    const splitCount = form.splitCount.trim() ? Number(form.splitCount) : 0;
    if (!Number.isInteger(splitCount) || splitCount < 0 || splitCount > 500) throw new Error("The number of people must be a whole number between 1 and 500.");
    if (form.splitMethod === "EQUAL" && splitCount === 0 && !unitCount) {
      throw new Error("Set the number of tenants in the unit first, or enter how many people share this bill.");
    }
    const input: ExpenseInput = {
      unitId: form.unitId,
      category: form.category,
      description: form.description,
      amount: Math.round(amount * 100) / 100,
      expenseDate: form.expenseDate,
      periodStart: opt(form.periodStart),
      periodEnd: opt(form.periodEnd),
      vendor: opt(form.vendor),
      reference: opt(form.reference),
      splitMethod: form.splitMethod,
      splitCount: form.splitMethod === "EQUAL" ? splitCount : 0,
      notes: opt(form.notes),
    };
    const saved = edit ? await api.updateExpense(edit.id, input) : await api.createExpense(input);
    await onSaved(saved);
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={edit ? "Edit expense" : "Add expense"}
      description={
        unit
          ? `For unit ${unit.label}. A bill can be split equally between the tenants living there.`
          : "A bill or cost that belongs to one unit. It can be split equally between the tenants living there."
      }
      submitLabel={edit ? "Save changes" : "Add expense"}
      onSubmit={submit}
      wide
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {!unit && (
          <>
            <SelectField
              id="x-building"
              label="Building"
              value={form.buildingId}
              onChange={(v) => setForm({ ...form, buildingId: v, unitId: "" })}
              options={(buildings.data ?? []).map((b) => ({ value: b.id, label: b.name }))}
              placeholder="Choose a building"
              required
            />
            <SelectField
              id="x-unit"
              label="Unit"
              value={form.unitId}
              onChange={(v) => setForm({ ...form, unitId: v })}
              options={(units.data?.items ?? []).map((u) => ({ value: u.id, label: u.tenantName ? `${u.unitNumber} · ${u.tenantName}` : u.unitNumber }))}
              placeholder={form.buildingId ? "Choose a unit" : "Choose a building first"}
              disabled={!form.buildingId}
              required
            />
          </>
        )}
        <SelectField<ExpenseCategory>
          id="x-category"
          label="Category"
          value={form.category}
          onChange={(v) => setForm({ ...form, category: (v || "OTHER") as ExpenseCategory })}
          options={keys(EXPENSE_CATEGORY_LABEL).map((c) => ({ value: c, label: EXPENSE_CATEGORY_LABEL[c] }))}
        />
        <TextField id="x-amount" label="Amount (AED)" type="number" value={form.amount} onChange={(v) => setForm({ ...form, amount: v })} placeholder="0.00" required />
        <TextField id="x-desc" label="Description" value={form.description} onChange={(v) => setForm({ ...form, description: v })} placeholder="e.g. DEWA bill September" required className="sm:col-span-2" />
        <TextField id="x-date" label="Expense date" type="date" value={form.expenseDate} onChange={(v) => setForm({ ...form, expenseDate: v })} required />
        <SelectField<SplitMethod>
          id="x-split"
          label="Split the bill"
          value={form.splitMethod}
          onChange={(v) => setForm({ ...form, splitMethod: (v || "NONE") as SplitMethod })}
          options={keys(SPLIT_METHOD_LABEL).map((m) => ({ value: m, label: SPLIT_METHOD_LABEL[m] }))}
        />
        {form.splitMethod === "EQUAL" && (
          <TextField
            id="x-split-count"
            label="Between how many people"
            type="number"
            value={form.splitCount}
            onChange={(v) => setForm({ ...form, splitCount: v })}
            placeholder={unitCount !== undefined ? String(unitCount) : ""}
            hint={
              unitCount === undefined
                ? "Leave empty to use the unit's number of tenants."
                : unitCount > 0
                  ? `Leave empty to use the unit's number of tenants (${unitCount}).`
                  : "This unit has no tenants recorded yet — enter the number here or set it on the unit."
            }
          />
        )}
        <TextField id="x-pstart" label="Billing period from" type="date" value={form.periodStart} onChange={(v) => setForm({ ...form, periodStart: v })} />
        <TextField id="x-pend" label="Billing period to" type="date" value={form.periodEnd} onChange={(v) => setForm({ ...form, periodEnd: v })} />
        <TextField id="x-vendor" label="Vendor" value={form.vendor} onChange={(v) => setForm({ ...form, vendor: v })} placeholder="DEWA, Etisalat, contractor…" />
        <TextField id="x-ref" label="Invoice / reference" value={form.reference} onChange={(v) => setForm({ ...form, reference: v })} />
        <TextAreaField id="x-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}
