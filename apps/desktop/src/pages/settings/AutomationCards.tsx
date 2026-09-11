import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, X } from "lucide-react";

import type { OrgSettings, ReminderRule, SweepSummary } from "@/api/types-domain";
import { Field, TextAreaField, TextField, errorMessage } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";

/** Spec §8: the reminder schedule, configurable by Admin. */
export function ReminderScheduleCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const rules = useQuery({ queryKey: ["reminder-rules"], queryFn: () => api.reminderRules() });
  const [items, setItems] = useState<ReminderRule[]>([]);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sweep, setSweep] = useState<SweepSummary | null>(null);

  useEffect(() => {
    if (rules.data && !dirty) setItems(rules.data.filter((r) => r.active).sort((a, b) => b.daysBefore - a.daysBefore));
  }, [rules.data, dirty]);

  const save = useMutation({
    mutationFn: () => api.saveReminderRules(items.map((r) => ({ ...r, active: true }))),
    onSuccess: () => {
      setDirty(false);
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["reminder-rules"] });
    },
    onError: (e) => setError(errorMessage(e)),
  });
  const run = useMutation({
    mutationFn: () => api.runSweep(),
    onSuccess: (s) => {
      setSweep(s);
      setError(null);
      void queryClient.invalidateQueries();
    },
    onError: (e) => setError(errorMessage(e)),
  });

  const patch = (idx: number, p: Partial<ReminderRule>) => {
    setItems((list) => list.map((r, i) => (i === idx ? { ...r, ...p } : r)));
    setDirty(true);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Reminder schedule</CardTitle>
        <CardDescription>Milestones before contract expiry. Each fires once per contract; when a contract enters late, only the nearest milestone fires. The sweep runs daily at 00:05.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className="overflow-x-auto">
          <table className="w-full text-[13px]">
            <thead className="text-[11px] tracking-[0.04em] text-muted-foreground uppercase">
              <tr>
                <th className="pb-1.5 pr-2 text-left font-medium">Days before</th>
                <th className="pb-1.5 pr-2 text-left font-medium">What happens</th>
                <th className="pb-1.5 px-2 font-medium">In-app</th>
                <th className="pb-1.5 px-2 font-medium">Email employee</th>
                <th className="pb-1.5 px-2 font-medium">Mark urgent</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {items.map((r, idx) => (
                <tr key={r.id ?? `new-${idx}`}>
                  <td className="py-1 pr-2">
                    <Input type="number" min={0} max={730} value={r.daysBefore} onChange={(e) => patch(idx, { daysBefore: Number(e.target.value) })} className="w-24 tabular-nums" aria-label="Days before expiry" />
                  </td>
                  <td className="py-1 pr-2">
                    <Input value={r.label} onChange={(e) => patch(idx, { label: e.target.value })} aria-label="Label" />
                  </td>
                  <td className="py-1 px-2 text-center">
                    <Checkbox checked={r.notifyInApp} onCheckedChange={(c) => patch(idx, { notifyInApp: c === true })} aria-label="In-app" />
                  </td>
                  <td className="py-1 px-2 text-center">
                    <Checkbox checked={r.emailAssignedEmployee} onCheckedChange={(c) => patch(idx, { emailAssignedEmployee: c === true })} aria-label="Email" />
                  </td>
                  <td className="py-1 px-2 text-center">
                    <Checkbox checked={r.markUrgent} onCheckedChange={(c) => patch(idx, { markUrgent: c === true })} aria-label="Urgent" />
                  </td>
                  <td className="py-1 text-right">
                    <Button size="icon-xs" variant="ghost" aria-label="Remove" onClick={() => { setItems((l) => l.filter((_, i) => i !== idx)); setDirty(true); }}>
                      <X />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" variant="outline" onClick={() => { setItems((l) => [...l, { id: null, daysBefore: 45, label: "", notifyInApp: true, emailAssignedEmployee: false, markUrgent: false, active: true }]); setDirty(true); }}>
            <Plus data-icon="inline-start" />
            Add milestone
          </Button>
          <Button size="sm" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
            {save.isPending ? "Saving…" : "Save schedule"}
          </Button>
          <span className="mx-2 h-5 border-l" />
          <Button size="sm" variant="outline" onClick={() => run.mutate()} disabled={run.isPending}>
            {run.isPending ? "Running…" : "Run expiry sweep now"}
          </Button>
          {sweep && (
            <span className="text-[12.5px] text-muted-foreground">
              {sweep.contractsChecked} contracts · {sweep.remindersFired} reminders · {sweep.casesOpened} cases opened · {sweep.contractsExpired} expired · {sweep.notificationsCreated} notifications · {sweep.emailsQueued} emails
            </span>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

/** Organisation profile, thresholds and letterhead (spec §21 "configurable by Admin"). */
export function OrganisationCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const org = useQuery({ queryKey: ["org-settings"], queryFn: () => api.orgSettings() });
  const [form, setForm] = useState<OrgSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (org.data && !form) setForm(org.data);
  }, [org.data, form]);

  const save = useMutation({
    mutationFn: () => api.saveOrgSettings(form!),
    onSuccess: (o) => {
      setForm(o);
      setError(null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
      void queryClient.invalidateQueries();
    },
    onError: (e) => setError(errorMessage(e)),
  });

  if (!form) return null;
  const set = (p: Partial<OrgSettings>) => setForm({ ...form, ...p });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Organisation</CardTitle>
        <CardDescription>Expiry thresholds drive the bands everywhere; the letterhead goes on every notice PDF.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField id="org-name" label="Organisation name" value={form.orgName} onChange={(v) => set({ orgName: v })} required />
          <TextField id="org-tz" label="Time zone (IANA)" value={form.timezone} onChange={(v) => set({ timezone: v })} hint="Display only; the server's ORG_TIMEZONE drives the calendar." />
          <TextField id="org-soon" label="Expiring soon (days)" type="number" value={String(form.thresholds.expiringSoonDays)} onChange={(v) => set({ thresholds: { ...form.thresholds, expiringSoonDays: Number(v) } })} hint="Contracts within this window show on the Renewal Dashboard." />
          <TextField id="org-urgent" label="Urgent (days)" type="number" value={String(form.thresholds.urgentDays)} onChange={(v) => set({ thresholds: { ...form.thresholds, urgentDays: Number(v) } })} />
          <TextField id="org-window" label="Completed renewals window (days)" type="number" value={String(form.completedWindowDays)} onChange={(v) => set({ completedWindowDays: Number(v) })} hint="Period for the dashboard card." />
          <Field label="Renewal cases">
            <label className="flex items-center gap-2 pt-1.5 text-[13px]">
              <Checkbox checked={form.autoOpenCase} onCheckedChange={(c) => set({ autoOpenCase: c === true })} />
              Open a case automatically when a contract enters the expiring-soon window
            </label>
          </Field>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField id="lh-company" label="Letterhead: company name" value={form.letterhead.companyName} onChange={(v) => set({ letterhead: { ...form.letterhead, companyName: v } })} />
          <TextField id="lh-phone" label="Letterhead: phone" value={form.letterhead.phone} onChange={(v) => set({ letterhead: { ...form.letterhead, phone: v } })} />
          <TextField id="lh-email" label="Letterhead: email" value={form.letterhead.email} onChange={(v) => set({ letterhead: { ...form.letterhead, email: v } })} />
          <TextAreaField id="lh-address" label="Letterhead: address (one line per row)" value={form.letterhead.addressLines.join("\n")} onChange={(v) => set({ letterhead: { ...form.letterhead, addressLines: v.split("\n") } })} rows={2} />
          <TextAreaField id="lh-footer" label="Letterhead: footer" value={form.letterhead.footer} onChange={(v) => set({ letterhead: { ...form.letterhead, footer: v } })} rows={2} className="sm:col-span-2" />
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" onClick={() => save.mutate()} disabled={save.isPending}>
            {save.isPending ? "Saving…" : "Save organisation settings"}
          </Button>
          {saved && <span className="text-[12.5px] text-muted-foreground">Saved.</span>}
        </div>
      </CardContent>
    </Card>
  );
}
