import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import type { Contract, ContractInput, ContractUnitTerms } from "@/api/types-domain";
import { Field, FormDialog, SelectField, TextAreaField, TextField, opt, str } from "@/components/forms";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";
import { UNIT_STATUS_LABEL, addDaysIso, todayIso, formatMoney } from "@/lib/format";
import { useBuildingOptions, useEmployees, useTenantOptions } from "@/lib/queries";

export function ContractDialog({
  open,
  onOpenChange,
  contract,
  onSaved,
  defaults,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  contract?: Contract | null;
  onSaved?: (c: Contract) => void;
  defaults?: { tenantId?: string; buildingId?: string; unitIds?: string[] };
}) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const buildings = useBuildingOptions();
  const tenants = useTenantOptions();
  const employees = useEmployees();
  const [form, setForm] = useState({
    contractNumber: "",
    tenantId: "",
    buildingId: "",
    unitIds: [] as string[],
    /** Tenants and rent per selected unit (text while editing). */
    unitTerms: {} as Record<string, { tenants: string; rent: string }>,
    startDate: todayIso(),
    endDate: addDaysIso(todayIso(), 364),
    rentTerms: "",
    assignedEmployeeId: "",
    notes: "",
    activate: true,
  });

  useEffect(() => {
    if (!open) return;
    if (contract) {
      setForm({
        contractNumber: contract.contractNumber,
        tenantId: contract.tenantId,
        buildingId: contract.buildingId,
        unitIds: contract.unitIds,
        unitTerms: Object.fromEntries(contract.unitTerms.map((t) => [t.unitId, { tenants: String(t.occupantCount), rent: t.rentAmount === null ? "" : t.rentAmount.toFixed(2) }])),
        startDate: contract.startDate,
        endDate: contract.endDate,
        rentTerms: str(contract.rentTerms),
        assignedEmployeeId: str(contract.assignedEmployeeId),
        notes: str(contract.notes),
        activate: contract.status === "ACTIVE",
      });
    } else {
      setForm((f) => ({
        ...f,
        contractNumber: "",
        tenantId: defaults?.tenantId ?? "",
        buildingId: defaults?.buildingId ?? "",
        unitIds: defaults?.unitIds ?? [],
        unitTerms: {},
        startDate: todayIso(),
        endDate: addDaysIso(todayIso(), 364),
        rentTerms: "",
        assignedEmployeeId: "",
        notes: "",
        activate: true,
      }));
      api.suggestContractNumber().then((s) => setForm((f) => (f.contractNumber ? f : { ...f, contractNumber: s.contractNumber }))).catch(() => {});
    }
  }, [open, contract, defaults, api]);

  // Units of the chosen building; an existing contract's own units stay selectable while occupied.
  const units = useQuery({
    queryKey: ["units", "for-building", form.buildingId],
    queryFn: () => api.listUnits({ buildingId: form.buildingId, pageSize: 200, sort: "unit_number" }),
    enabled: open && form.buildingId !== "",
  });
  const selectable = useMemo(
    () => (units.data?.items ?? []).filter((u) => u.status !== "OCCUPIED" || u.contractId === contract?.id || form.unitIds.includes(u.id)),
    [units.data, contract, form.unitIds],
  );
  const locked = contract?.status === "ACTIVE";

  function toggleUnit(id: string, on: boolean) {
    setForm((f) => ({ ...f, unitIds: on ? [...new Set([...f.unitIds, id])] : f.unitIds.filter((u) => u !== id) }));
  }

  async function submit() {
    const unitTerms = form.unitIds.map((id) => parseUnitTerms(id, form.unitTerms[id]));
    const input: ContractInput = {
      contractNumber: form.contractNumber,
      tenantId: form.tenantId,
      buildingId: form.buildingId,
      unitIds: form.unitIds,
      unitTerms,
      startDate: form.startDate,
      endDate: form.endDate,
      rentTerms: opt(form.rentTerms),
      assignedEmployeeId: form.assignedEmployeeId || null,
      notes: opt(form.notes),
      activate: contract ? undefined : form.activate,
    };
    const saved = contract ? await api.updateContract(contract.id, input) : await api.createContract(input);
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["contracts"] }),
      queryClient.invalidateQueries({ queryKey: ["units"] }),
      queryClient.invalidateQueries({ queryKey: ["buildings"] }),
      queryClient.invalidateQueries({ queryKey: ["tenants"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard"] }),
    ]);
    onSaved?.(saved);
  }

  return (
    <FormDialog open={open} onOpenChange={onOpenChange} title={contract ? `Edit contract ${contract.contractNumber}` : "New contract"} submitLabel={contract ? "Save changes" : form.activate ? "Create active contract" : "Save as draft"} onSubmit={submit} wide>
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="c-number" label="Contract number" value={form.contractNumber} onChange={(v) => setForm({ ...form, contractNumber: v })} required />
        <SelectField
          id="c-assigned"
          label="Assigned leasing employee"
          value={form.assignedEmployeeId}
          onChange={(v) => setForm({ ...form, assignedEmployeeId: v })}
          options={(employees.data ?? []).map((e) => ({ value: e.id, label: e.name }))}
          placeholder="Unassigned"
        />
        <SelectField
          id="c-tenant"
          label="Tenant"
          value={form.tenantId}
          onChange={(v) => setForm({ ...form, tenantId: v })}
          options={(tenants.data ?? []).map((t) => ({ value: t.id, label: t.name }))}
          placeholder="Select a tenant"
          required
          disabled={locked}
        />
        <SelectField
          id="c-building"
          label="Building"
          value={form.buildingId}
          onChange={(v) => setForm({ ...form, buildingId: v, unitIds: [], unitTerms: {} })}
          options={(buildings.data ?? []).map((b) => ({ value: b.id, label: `${b.name} (${b.code})` }))}
          placeholder="Select a building"
          required
          disabled={locked}
        />
        <Field label="Units" className="sm:col-span-2" hint={form.buildingId ? "Only units without another active contract are listed." : "Choose a building first."}>
          <div className="grid max-h-44 grid-cols-2 gap-1.5 overflow-y-auto rounded-md border p-2 md:grid-cols-3">
            {selectable.length === 0 && <span className="col-span-full px-1 py-2 text-[12.5px] text-muted-foreground">{form.buildingId ? "No available units in this building." : "—"}</span>}
            {selectable.map((u) => (
              <label key={u.id} className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 text-[13px] hover:bg-muted">
                <Checkbox checked={form.unitIds.includes(u.id)} onCheckedChange={(c) => toggleUnit(u.id, c === true)} />
                <span className="font-medium">{u.unitNumber}</span>
                <span className="truncate text-muted-foreground">{u.unitType ?? UNIT_STATUS_LABEL[u.status]}</span>
              </label>
            ))}
          </div>
        </Field>
        {form.unitIds.length > 0 && (
          <Field label="Tenants and rent per unit" className="sm:col-span-2" hint="People living in each unit (its bills are split equally between them) and that unit's rent for the contract period.">
            <UnitTermsTable
              units={form.unitIds.map((id) => ({ id, label: units.data?.items.find((x) => x.id === id)?.unitNumber ?? "…" }))}
              value={form.unitTerms}
              onChange={(unitTerms) => setForm((f) => ({ ...f, unitTerms }))}
            />
          </Field>
        )}
        <TextField id="c-start" label="Start date" type="date" value={form.startDate} onChange={(v) => setForm({ ...form, startDate: v })} required />
        <TextField id="c-end" label="End date" type="date" value={form.endDate} onChange={(v) => setForm({ ...form, endDate: v })} required />
        <TextField id="c-rent" label="Payment terms (optional)" value={form.rentTerms} onChange={(v) => setForm({ ...form, rentTerms: v })} placeholder="e.g. 4 cheques, AED 800 per bed per month" className="sm:col-span-2" hint="Display only — this system does not do accounting." />
        <TextAreaField id="c-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
        {!contract && (
          <label className="flex items-center gap-2 text-[13.5px] sm:col-span-2">
            <Checkbox checked={form.activate} onCheckedChange={(c) => setForm({ ...form, activate: c === true })} />
            Activate now (marks the units Occupied). Untick to save as a draft.
          </label>
        )}
      </div>
    </FormDialog>
  );
}

export type UnitTermsDraft = Record<string, { tenants: string; rent: string }>;

/** Validates one unit's draft (tenants 0–500, rent ≥ 0) into the API shape. */
export function parseUnitTerms(unitId: string, draft: { tenants: string; rent: string } | undefined): ContractUnitTerms {
  const n = Number((draft?.tenants ?? "").trim() || "0");
  if (!Number.isInteger(n) || n < 0 || n > 500) throw new Error("The number of tenants must be a whole number between 0 and 500.");
  const rentText = (draft?.rent ?? "").trim();
  const rent = rentText ? Number(rentText) : null;
  if (rent !== null && (!Number.isFinite(rent) || rent < 0)) throw new Error("Enter each unit's rent as a number of AED (0 or more).");
  return { unitId, occupantCount: n, rentAmount: rent === null ? null : Math.round(rent * 100) / 100 };
}

/** Unit | tenants | rent — one row per unit on the contract. */
export function UnitTermsTable({ units, value, onChange }: { units: { id: string; label: string }[]; value: UnitTermsDraft; onChange: (v: UnitTermsDraft) => void }) {
  const set = (id: string, patch: Partial<{ tenants: string; rent: string }>) => onChange({ ...value, [id]: { ...(value[id] ?? { tenants: "", rent: "" }), ...patch } });
  const total = units.reduce((a, u) => a + (Number((value[u.id]?.rent ?? "").trim() || "0") || 0), 0);
  return (
    <div className="overflow-x-auto rounded-md border">
      <table className="w-full text-[13px]">
        <thead className="bg-muted/50 text-[11px] font-medium tracking-[0.06em] text-muted-foreground uppercase">
          <tr>
            <th className="px-3 py-1.5 text-left">Unit</th>
            <th className="px-3 py-1.5 text-right">No. of tenants</th>
            <th className="px-3 py-1.5 text-right">Rent (AED)</th>
          </tr>
        </thead>
        <tbody className="divide-y">
          {units.map((u) => (
            <tr key={u.id}>
              <td className="px-3 py-1.5 font-medium">Unit {u.label}</td>
              <td className="px-3 py-1.5 text-right">
                <Input type="number" min={0} max={500} inputMode="numeric" className="ml-auto h-7 w-20 text-right tabular-nums" value={value[u.id]?.tenants ?? ""} placeholder="0" onChange={(e) => set(u.id, { tenants: e.target.value })} aria-label={`Number of tenants in unit ${u.label}`} />
              </td>
              <td className="px-3 py-1.5 text-right">
                <Input type="number" min={0} step="0.01" inputMode="decimal" className="ml-auto h-7 w-32 text-right tabular-nums" value={value[u.id]?.rent ?? ""} placeholder="0.00" onChange={(e) => set(u.id, { rent: e.target.value })} aria-label={`Rent for unit ${u.label}`} />
              </td>
            </tr>
          ))}
        </tbody>
        {units.length > 1 && (
          <tfoot>
            <tr className="border-t bg-muted/30">
              <td className="px-3 py-1.5 text-muted-foreground" colSpan={2}>Contract total</td>
              <td className="px-3 py-1.5 text-right font-medium tabular-nums">{formatMoney(total)}</td>
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  );
}
