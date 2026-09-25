import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { UnitInput, UnitStatus, UnitSummary, UnitType } from "@/api/types-domain";
import { errorMessage, FormDialog, SelectField, TextAreaField, TextField, opt, str } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { UNIT_STATUS_LABEL, UNIT_TYPE_LABEL, keys } from "@/lib/format";
import { useBuildingOptions } from "@/lib/queries";

const MANUAL_STATUSES: UnitStatus[] = ["VACANT", "RESERVED", "MAINTENANCE"];

export function UnitDialog({
  open,
  onOpenChange,
  unit,
  buildingId,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  unit?: UnitSummary | null;
  /** Preselects (and locks) the building when opened from a building page. */
  buildingId?: string;
  onSaved?: (u: UnitSummary) => void;
}) {
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const buildings = useBuildingOptions();
  const [error, setError] = useState<string | null>(null);
  const [form, setForm] = useState({ buildingId: "", unitNumber: "", floor: "", unitType: "", status: "VACANT" as UnitStatus, notes: "" });

  useEffect(() => {
    if (open) {
      setForm({
        buildingId: unit?.buildingId ?? buildingId ?? "",
        unitNumber: str(unit?.unitNumber),
        floor: str(unit?.floor),
        unitType: str(unit?.unitType),
        status: unit?.status ?? "VACANT",
        notes: str(unit?.notes),
      });
    }
  }, [open, unit, buildingId]);

  const occupied = unit?.status === "OCCUPIED";

  async function submit() {
    const input: UnitInput = {
      buildingId: form.buildingId,
      unitNumber: form.unitNumber,
      floor: opt(form.floor),
      unitType: opt(form.unitType),
      status: form.status,
      notes: opt(form.notes),
    };
    const saved = unit ? await api.updateUnit(unit.id, input) : await api.createUnit(input);
    await queryClient.invalidateQueries({ queryKey: ["units"] });
    await queryClient.invalidateQueries({ queryKey: ["buildings"] });
    onSaved?.(saved);
  }

  async function archive() {
    if (!unit || !window.confirm(`Archive unit ${unit.unitNumber}? It stays in past contracts and reports.`)) return;
    await api.archiveUnit(unit.id);
    await queryClient.invalidateQueries({ queryKey: ["units"] });
    await queryClient.invalidateQueries({ queryKey: ["buildings"] });
    onOpenChange(false);
    onSaved?.(unit);
  }

  /** For units added by mistake: only possible while nothing references them. */
  async function remove() {
    if (!unit || !window.confirm(`Delete unit ${unit.unitNumber} for good? Use this only for a unit added by mistake.`)) return;
    setError(null);
    try {
      await api.deleteUnit(unit.id);
      await queryClient.invalidateQueries({ queryKey: ["units"] });
      await queryClient.invalidateQueries({ queryKey: ["buildings"] });
      onOpenChange(false);
      onSaved?.(unit);
    } catch (e) {
      setError(errorMessage(e, "Could not delete this unit."));
    }
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={unit ? `Edit unit ${unit.unitNumber}` : "Add unit"}
      submitLabel={unit ? "Save changes" : "Add unit"}
      onSubmit={submit}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField
          id="u-building"
          label="Building"
          value={form.buildingId}
          onChange={(v) => setForm({ ...form, buildingId: v })}
          options={(buildings.data ?? []).map((b) => ({ value: b.id, label: `${b.name} (${b.code})` }))}
          placeholder="Select a building"
          required
          disabled={!!buildingId || occupied}
          className="sm:col-span-2"
        />
        <TextField id="u-number" label="Unit number" value={form.unitNumber} onChange={(v) => setForm({ ...form, unitNumber: v })} required autoFocus />
        <TextField id="u-floor" label="Floor" value={form.floor} onChange={(v) => setForm({ ...form, floor: v })} placeholder="Optional" />
        <SelectField<UnitType>
          id="u-type"
          label="Unit type"
          value={(form.unitType || "") as UnitType | ""}
          onChange={(v) => setForm({ ...form, unitType: v })}
          options={keys(UNIT_TYPE_LABEL).map((t) => ({ value: t, label: UNIT_TYPE_LABEL[t] }))}
          placeholder="Not set"
          hint="How many people live there is the number of occupants on the contract."
        />
        <SelectField
          id="u-status"
          label="Status"
          value={form.status}
          onChange={(v) => setForm({ ...form, status: (v || "VACANT") as UnitStatus })}
          options={(occupied ? (["OCCUPIED"] as UnitStatus[]) : MANUAL_STATUSES).map((s) => ({ value: s, label: UNIT_STATUS_LABEL[s] }))}
          disabled={occupied}
          hint={occupied ? "Occupied follows the active contract." : undefined}
        />
        <TextAreaField id="u-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
        {unit && can("MANAGE_UNITS") && !occupied && (
          <div className="flex flex-wrap items-center gap-2 sm:col-span-2">
            <Button type="button" variant="ghost" size="sm" onClick={() => void archive()}>
              Archive this unit
            </Button>
            <Button type="button" variant="ghost" size="sm" className="text-destructive hover:text-destructive" onClick={() => void remove()}>
              Delete permanently
            </Button>
            <span className="text-[12px] text-muted-foreground">Archive keeps it in past contracts; delete is only for a unit added by mistake.</span>
          </div>
        )}
        {error && <p className="text-[12.5px] text-destructive sm:col-span-2">{error}</p>}
      </div>
    </FormDialog>
  );
}
