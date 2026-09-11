import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { Building, BuildingInput } from "@/api/types-domain";
import { FormDialog, TextAreaField, TextField, opt, str } from "@/components/forms";
import { useApp } from "@/lib/app-state";

export function BuildingDialog({
  open,
  onOpenChange,
  building,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  building?: Building | null;
  onSaved?: (b: Building) => void;
}) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const [form, setForm] = useState({ name: "", code: "", location: "", buildingType: "", notes: "" });

  useEffect(() => {
    if (open) {
      setForm({
        name: str(building?.name),
        code: str(building?.code),
        location: str(building?.location),
        buildingType: str(building?.buildingType),
        notes: str(building?.notes),
      });
    }
  }, [open, building]);

  async function submit() {
    const input: BuildingInput = {
      name: form.name,
      code: form.code,
      location: opt(form.location),
      buildingType: opt(form.buildingType),
      notes: opt(form.notes),
    };
    const saved = building ? await api.updateBuilding(building.id, input) : await api.createBuilding(input);
    await queryClient.invalidateQueries({ queryKey: ["buildings"] });
    onSaved?.(saved);
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={building ? "Edit building" : "Add building"}
      submitLabel={building ? "Save changes" : "Add building"}
      onSubmit={submit}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="b-name" label="Property name" value={form.name} onChange={(v) => setForm({ ...form, name: v })} required autoFocus />
        <TextField id="b-code" label="Property code" value={form.code} onChange={(v) => setForm({ ...form, code: v })} required hint="Short unique code, e.g. ANT" />
        <TextField id="b-location" label="Location" value={form.location} onChange={(v) => setForm({ ...form, location: v })} />
        <TextField id="b-type" label="Building type" value={form.buildingType} onChange={(v) => setForm({ ...form, buildingType: v })} placeholder="Office, Retail, Residential…" />
        <TextAreaField id="b-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}
