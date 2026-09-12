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
        {/* One block per milestone: inputs on the first line, the three switches on the second
            (phones); on wider screens everything sits on one line. */}
        <div className="flex flex-col divide-y rounded-md border">
          <div className="hidden grid-cols-[6rem_minmax(0,1fr)_auto_auto_auto_2rem] items-center gap-x-3 px-3 py-1.5 text-[11px] tracking-[0.04em] text-muted-foreground uppercase md:grid">
            <span>Days before</span>
            <span>What happens</span>
            <span className="text-center">In-app</span>
            <span className="text-center">Email employee</span>
            <span className="text-center">Mark urgent</span>
            <span />
          </div>
          {items.map((r, idx) => (
            <div key={r.id ?? `new-${idx}`} className="grid grid-cols-[6rem_minmax(0,1fr)_2rem] items-center gap-x-3 gap-y-2 px-3 py-2 text-[13px] md:grid-cols-[6rem_minmax(0,1fr)_auto_auto_auto_2rem]">
              <Input type="number" min={0} max={730} value={r.daysBefore} onChange={(e) => patch(idx, { daysBefore: Number(e.target.value) })} className="tabular-nums" aria-label="Days before expiry" />
              <Input value={r.label} onChange={(e) => patch(idx, { label: e.target.value })} aria-label="Label" placeholder="What happens" />
              <div className="row-start-1 col-start-3 justify-self-end md:col-start-6">
                <Button size="icon-xs" variant="ghost" aria-label="Remove" onClick={() => { setItems((l) => l.filter((_, i) => i !== idx)); setDirty(true); }}>
                  <X />
                </Button>
              </div>
              <label className="col-span-3 flex items-center gap-2 md:col-span-1 md:col-start-3 md:row-start-1 md:justify-center [&>span]:md:sr-only">
                <Checkbox checked={r.notifyInApp} onCheckedChange={(c) => patch(idx, { notifyInApp: c === true })} aria-label="In-app" />
                <span className="text-[12.5px] text-muted-foreground">In-app notification</span>
              </label>
              <label className="col-span-3 flex items-center gap-2 md:col-span-1 md:col-start-4 md:row-start-1 md:justify-center [&>span]:md:sr-only">
                <Checkbox checked={r.emailAssignedEmployee} onCheckedChange={(c) => patch(idx, { emailAssignedEmployee: c === true })} aria-label="Email" />
                <span className="text-[12.5px] text-muted-foreground">Email the assigned employee</span>
              </label>
              <label className="col-span-3 flex items-center gap-2 md:col-span-1 md:col-start-5 md:row-start-1 md:justify-center [&>span]:md:sr-only">
                <Checkbox checked={r.markUrgent} onCheckedChange={(c) => patch(idx, { markUrgent: c === true })} aria-label="Urgent" />
                <span className="text-[12.5px] text-muted-foreground">Mark as urgent</span>
              </label>
            </div>
          ))}
          {items.length === 0 && <p className="px-3 py-4 text-center text-[13px] text-muted-foreground">No milestones — nothing will remind the team.</p>}
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
