import { useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { UnitType } from "@/api/types-domain";
import { Field, FormDialog, SelectField, TextField } from "@/components/forms";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";
import { UNIT_TYPE_LABEL, keys } from "@/lib/format";
import { useBuildingOptions } from "@/lib/queries";

/** "101-110", "101,103, 105", "1F-01 … 1F-06" — ranges and lists, numbers or letters kept as typed. */
export function expandUnitNumbers(input: string): string[] {
  const out: string[] = [];
  for (const partRaw of input.split(/[,\n]/)) {
    const part = partRaw.trim();
    if (!part) continue;
    const range = part.match(/^(.*?)(\d+)\s*(?:-|–|to|\.\.\.?)\s*(?:\1)?(\d+)$/i);
    if (range) {
      const [, prefix, fromText, toText] = range;
      const from = Number(fromText);
      const to = Number(toText);
      if (Number.isInteger(from) && Number.isInteger(to) && to >= from && to - from < 500) {
        const width = fromText.startsWith("0") ? fromText.length : 0;
        for (let n = from; n <= to; n++) out.push(`${prefix.trim()}${width ? String(n).padStart(width, "0") : n}`);
        continue;
      }
    }
    out.push(part);
  }
  return [...new Set(out)];
}

/**
 * Adds a whole floor or building of units at once — the slow part of setting up a new
 * property. Numbers already in use are skipped, never duplicated.
 */
export function BulkUnitsDialog({ open, onOpenChange, buildingId, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; buildingId?: string; onSaved?: () => void }) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const buildings = useBuildingOptions();
  const [form, setForm] = useState({ buildingId: buildingId ?? "", numbers: "", floor: "", unitType: "RESIDENTIAL" as UnitType | "", notes: "" });
  const [result, setResult] = useState<{ created: number; skipped: string[] } | null>(null);

  useEffect(() => {
    if (open) {
      setForm({ buildingId: buildingId ?? "", numbers: "", floor: "", unitType: "RESIDENTIAL", notes: "" });
      setResult(null);
    }
  }, [open, buildingId]);

  const preview = useMemo(() => expandUnitNumbers(form.numbers), [form.numbers]);

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Add units"
      description="Type a range like 101-120, or a list like 101, 102, 205. Units that already exist in the building are skipped."
      submitLabel={preview.length > 1 ? `Add ${preview.length} units` : "Add units"}
      onSubmit={async () => {
        if (!form.buildingId) throw new Error("Choose the building.");
        if (preview.length === 0) throw new Error("Enter the unit numbers, e.g. 101-110.");
        if (preview.length > 500) throw new Error("At most 500 units can be added at once.");
        const res = await api.createUnitsBulk(form.buildingId, {
          numbers: preview,
          floors: form.floor.trim() ? [form.floor.trim()] : [],
          unitType: form.unitType || null,
          notes: form.notes.trim() || null,
        });
        setResult(res);
        await queryClient.invalidateQueries({ queryKey: ["units"] });
        await queryClient.invalidateQueries({ queryKey: ["buildings"] });
        onSaved?.();
        if (res.skipped.length > 0) {
          throw new Error(`Added ${res.created}. Already there, skipped: ${res.skipped.slice(0, 12).join(", ")}${res.skipped.length > 12 ? "…" : ""}`);
        }
      }}
      wide
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {!buildingId && (
          <SelectField
            id="bu-building"
            label="Building"
            value={form.buildingId}
            onChange={(v) => setForm({ ...form, buildingId: v })}
            options={(buildings.data ?? []).map((b) => ({ value: b.id, label: `${b.name} (${b.code})` }))}
            placeholder="Select a building"
            required
            className="sm:col-span-2"
          />
        )}
        <Field label="Unit numbers" className="sm:col-span-2" hint="Ranges (101-120), lists (101, 102, 205) or both, one per line if you prefer.">
          <Input value={form.numbers} onChange={(e) => setForm({ ...form, numbers: e.target.value })} placeholder="101-120" autoFocus aria-label="Unit numbers" />
        </Field>
        <TextField id="bu-floor" label="Floor (optional)" value={form.floor} onChange={(v) => setForm({ ...form, floor: v })} placeholder="e.g. 1" hint="Applied to all of them; edit individual units later." />
        <SelectField<UnitType>
          id="bu-type"
          label="Unit type"
          value={form.unitType}
          onChange={(v) => setForm({ ...form, unitType: (v || "") as UnitType | "" })}
          options={keys(UNIT_TYPE_LABEL).map((t) => ({ value: t, label: UNIT_TYPE_LABEL[t] }))}
          placeholder="Not set"
        />
        <TextField id="bu-notes" label="Notes (optional)" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
        {preview.length > 0 && (
          <p className="text-[12.5px] text-muted-foreground sm:col-span-2">
            {preview.length} unit{preview.length === 1 ? "" : "s"}: {preview.slice(0, 14).join(", ")}
            {preview.length > 14 ? ` … ${preview[preview.length - 1]}` : ""}
          </p>
        )}
        {result && result.created > 0 && (
          <p className="text-[12.5px] text-muted-foreground sm:col-span-2">Added {result.created} unit{result.created === 1 ? "" : "s"}.</p>
        )}
      </div>
    </FormDialog>
  );
}
