import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import type { Contract, ContractInput } from "@/api/types-domain";
import { Field, FormDialog, SelectField, TextAreaField, TextField, opt, str } from "@/components/forms";
import { Checkbox } from "@/components/ui/checkbox";
import { useApp } from "@/lib/app-state";
import { UNIT_STATUS_LABEL, addDaysIso, todayIso } from "@/lib/format";
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
    const input: ContractInput = {
      contractNumber: form.contractNumber,
      tenantId: form.tenantId,
      buildingId: form.buildingId,
      unitIds: form.unitIds,
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
          onChange={(v) => setForm({ ...form, buildingId: v, unitIds: [] })}
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
        <TextField id="c-start" label="Start date" type="date" value={form.startDate} onChange={(v) => setForm({ ...form, startDate: v })} required />
        <TextField id="c-end" label="End date" type="date" value={form.endDate} onChange={(v) => setForm({ ...form, endDate: v })} required />
        <TextField id="c-rent" label="Rental terms / rent amount (optional)" value={form.rentTerms} onChange={(v) => setForm({ ...form, rentTerms: v })} placeholder="e.g. AED 120,000 per year, 4 cheques" className="sm:col-span-2" hint="Display only — this system does not do accounting." />
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
