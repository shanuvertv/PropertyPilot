import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import type { Tenant, TenantInput } from "@/api/types-domain";
import { FormDialog, TextAreaField, TextField, opt, str } from "@/components/forms";
import { useApp } from "@/lib/app-state";

export function TenantDialog({
  open,
  onOpenChange,
  tenant,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  tenant?: Tenant | null;
  onSaved?: (t: Tenant) => void;
}) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const [form, setForm] = useState({ name: "", contactPerson: "", mobile: "", email: "", altContact: "", address: "", notes: "" });

  useEffect(() => {
    if (open) {
      setForm({
        name: str(tenant?.name),
        contactPerson: str(tenant?.contactPerson),
        mobile: str(tenant?.mobile),
        email: str(tenant?.email),
        altContact: str(tenant?.altContact),
        address: str(tenant?.address),
        notes: str(tenant?.notes),
      });
    }
  }, [open, tenant]);

  async function submit() {
    const input: TenantInput = {
      name: form.name,
      contactPerson: opt(form.contactPerson),
      mobile: opt(form.mobile),
      email: opt(form.email),
      altContact: opt(form.altContact),
      address: opt(form.address),
      notes: opt(form.notes),
    };
    const saved = tenant ? await api.updateTenant(tenant.id, input) : await api.createTenant(input);
    await queryClient.invalidateQueries({ queryKey: ["tenants"] });
    onSaved?.(saved);
  }

  return (
    <FormDialog open={open} onOpenChange={onOpenChange} title={tenant ? "Edit tenant" : "Add tenant"} submitLabel={tenant ? "Save changes" : "Add tenant"} onSubmit={submit} wide>
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="t-name" label="Tenant / company name" value={form.name} onChange={(v) => setForm({ ...form, name: v })} required autoFocus className="sm:col-span-2" />
        <TextField id="t-contact" label="Contact person" value={form.contactPerson} onChange={(v) => setForm({ ...form, contactPerson: v })} />
        <TextField id="t-mobile" label="Mobile number" value={form.mobile} onChange={(v) => setForm({ ...form, mobile: v })} type="tel" />
        <TextField id="t-email" label="Email address" value={form.email} onChange={(v) => setForm({ ...form, email: v })} type="email" hint="Renewal notices are sent here." />
        <TextField id="t-alt" label="Alternative contact" value={form.altContact} onChange={(v) => setForm({ ...form, altContact: v })} />
        <TextAreaField id="t-address" label="Address" value={form.address} onChange={(v) => setForm({ ...form, address: v })} rows={2} className="sm:col-span-2" />
        <TextAreaField id="t-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}
