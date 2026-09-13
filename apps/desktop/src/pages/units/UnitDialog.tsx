import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { UnitInput, UnitStatus, UnitSummary } from "@/api/types-domain";
import { FormDialog, SelectField, TextAreaField, TextField, opt, str } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { UNIT_STATUS_LABEL } from "@/lib/format";
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
    if (!unit || !window.confirm(`Archive unit ${unit.unitNumber}?`)) return;
    await api.archiveUnit(unit.id);
    await queryClient.invalidateQueries({ queryKey: ["units"] });
    await queryClient.invalidateQueries({ queryKey: ["buildings"] });
    onOpenChange(false);
    onSaved?.(unit);
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
        <TextField id="u-type" label="Unit type" value={form.unitType} onChange={(v) => setForm({ ...form, unitType: v })} placeholder="Office, Shop, Warehouse…" />
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
          <div className="sm:col-span-2">
            <Button type="button" variant="ghost" size="sm" onClick={() => void archive()}>
              Archive this unit
            </Button>
          </div>
        )}
      </div>
    </FormDialog>
  );
}
