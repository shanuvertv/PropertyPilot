import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Send } from "lucide-react";

import type { MailConfigInput, MailConfigView, MailProviderKind, SmtpSecurity } from "@/api/types-domain";
import { errorMessage, SelectField, TextField } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useApp } from "@/lib/app-state";

type Form = Omit<MailConfigInput, "smtpPassword"> & { smtpPassword: string };

function toForm(v: MailConfigView): Form {
  return {
    provider: v.provider,
    fromName: v.fromName,
    fromAddress: v.fromAddress,
    smtpHost: v.smtpHost,
    smtpPort: v.smtpPort,
    smtpSecurity: v.smtpSecurity,
    smtpUsername: v.smtpUsername,
    smtpPassword: "",
    imapEnabled: v.imapEnabled,
    imapHost: v.imapHost,
    imapPort: v.imapPort,
    imapSentFolder: v.imapSentFolder,
  };
}

/**
 * Settings → Email sending (Admin). The mailbox notices and reminders go out from.
 * Saved on the server, so every desktop and phone uses the same mailbox; the server's
 * `.env` only applies until something is saved here.
 */
export function MailSettingsCard() {
  const { api, session } = useApp();
  const queryClient = useQueryClient();
  const current = useQuery({ queryKey: ["mail-settings"], queryFn: () => api.mailSettings() });
  const [form, setForm] = useState<Form | null>(null);
  const [dirty, setDirty] = useState(false);
  const [testTo, setTestTo] = useState(session?.email ?? "");
  const [testResult, setTestResult] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    if (current.data && !dirty) setForm(toForm(current.data));
  }, [current.data, dirty]);

  const save = useMutation({
    mutationFn: (f: Form) => api.saveMailSettings({ ...f, smtpPassword: f.smtpPassword || null }),
    onSuccess: () => {
      setDirty(false);
      setTestResult(null);
      void queryClient.invalidateQueries({ queryKey: ["mail-settings"] });
      void queryClient.invalidateQueries({ queryKey: ["mail-status"] });
    },
  });

  const test = useMutation({
    mutationFn: () => api.sendTestMail(testTo),
    onSuccess: (r) => setTestResult({ ok: true, text: `Sent via ${r.provider}${r.providerMessageId ? ` (id ${r.providerMessageId})` : ""}. Check the inbox of ${testTo}.` }),
    onError: (e) => setTestResult({ ok: false, text: errorMessage(e, "The test email could not be sent.") }),
  });

  function patch(p: Partial<Form>) {
    setForm((f) => (f ? { ...f, ...p } : f));
    setDirty(true);
  }

  if (!form) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Email sending</CardTitle>
        </CardHeader>
        <CardContent className="text-[13px] text-muted-foreground">{current.isError ? errorMessage(current.error, "Could not load the email settings.") : "Loading…"}</CardContent>
      </Card>
    );
  }

  const smtp = form.provider === "smtp";

  return (
    <Card>
      <CardHeader>
        <CardTitle>Email sending</CardTitle>
        <CardDescription>
          The mailbox renewal notices, reminders and internal emails are sent from. Use the SMTP details of your own
          mailbox (the same account you read with IMAP or Outlook).
          {!current.data?.configured && " Nothing is saved yet — the server's own configuration is in use."}
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {save.isError && (
          <Alert variant="destructive">
            <AlertDescription>{errorMessage(save.error, "Could not save the email settings.")}</AlertDescription>
          </Alert>
        )}
        <div className="grid gap-4 sm:grid-cols-2">
          <SelectField<MailProviderKind>
            id="mail-provider"
            label="Provider"
            value={form.provider}
            onChange={(v) => patch({ provider: (v || "smtp") as MailProviderKind })}
            options={[
              { value: "smtp", label: "SMTP (your mailbox)" },
              { value: "log", label: "Log only — do not send (testing)" },
            ]}
          />
          <div />
          <TextField id="mail-from-name" label="Sender name" value={form.fromName} onChange={(v) => patch({ fromName: v })} placeholder="Leasing Department" required />
          <TextField id="mail-from" label="Sender address" type="email" value={form.fromAddress} onChange={(v) => patch({ fromAddress: v })} placeholder="leasing@yourcompany.com" required />
          {smtp && (
            <>
              <TextField id="smtp-host" label="SMTP host" value={form.smtpHost} onChange={(v) => patch({ smtpHost: v })} placeholder="mail.yourcompany.com" required hint="Microsoft 365: smtp.office365.com · Google: smtp.gmail.com" />
              <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-3">
                <TextField id="smtp-port" label="Port" type="number" value={String(form.smtpPort)} onChange={(v) => patch({ smtpPort: Number(v) || 0 })} required />
                <SelectField<SmtpSecurity>
                  id="smtp-security"
                  label="Security"
                  value={form.smtpSecurity}
                  onChange={(v) => patch({ smtpSecurity: (v || "starttls") as SmtpSecurity, smtpPort: v === "ssl" ? 465 : 587 })}
                  options={[
                    { value: "starttls", label: "STARTTLS (587)" },
                    { value: "ssl", label: "SSL / TLS (465)" },
                  ]}
                />
              </div>
              <TextField id="smtp-user" label="Username" value={form.smtpUsername} onChange={(v) => patch({ smtpUsername: v })} placeholder="usually the full email address" />
              <TextField
                id="smtp-pass"
                label="Password"
                type="password"
                value={form.smtpPassword}
                onChange={(v) => patch({ smtpPassword: v })}
                placeholder={current.data?.hasPassword ? "•••••••• (saved — leave blank to keep)" : "mailbox or app password"}
                hint={current.data?.hasPassword ? "A password is saved. Type a new one only to replace it." : "For Google or Microsoft accounts with 2-step verification, use an app password."}
              />
              <label className="flex items-start gap-2 sm:col-span-2">
                <Checkbox checked={form.imapEnabled} onCheckedChange={(c) => patch({ imapEnabled: c === true })} aria-label="Copy to Sent folder" className="mt-0.5" />
                <span className="text-[13px]">
                  Also file every sent email in the mailbox's <b>Sent</b> folder (IMAP), so Outlook and phones show what PropertyPilot sent.
                </span>
              </label>
              {form.imapEnabled && (
                <>
                  <TextField id="imap-host" label="IMAP host" value={form.imapHost} onChange={(v) => patch({ imapHost: v })} placeholder="mail.yourcompany.com" required />
                  <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] gap-3">
                    <TextField id="imap-port" label="IMAP port" type="number" value={String(form.imapPort)} onChange={(v) => patch({ imapPort: Number(v) || 0 })} />
                    <TextField id="imap-folder" label="Sent folder" value={form.imapSentFolder} onChange={(v) => patch({ imapSentFolder: v })} placeholder="auto-detect" />
                  </div>
                </>
              )}
            </>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" onClick={() => form && save.mutate(form)} disabled={!dirty || save.isPending}>
            {save.isPending ? "Saving…" : "Save email settings"}
          </Button>
          {dirty && <span className="text-[12.5px] text-muted-foreground">Unsaved changes</span>}
          {!dirty && save.isSuccess && <span className="text-[12.5px] text-muted-foreground">Saved — the server now sends with these settings.</span>}
        </div>

        <div className="rounded-md border bg-muted/30 p-3">
          <Label htmlFor="mail-test-to" className="text-[13px]">
            Send a test email
          </Label>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <Input id="mail-test-to" type="email" value={testTo} onChange={(e) => setTestTo(e.target.value)} placeholder="you@yourcompany.com" className="w-full sm:w-72" />
            <Button size="sm" variant="outline" onClick={() => test.mutate()} disabled={test.isPending || dirty || !testTo.trim()}>
              <Send data-icon="inline-start" />
              {test.isPending ? "Sending…" : "Send test"}
            </Button>
            {dirty && <span className="text-[12px] text-muted-foreground">Save first.</span>}
          </div>
          {testResult && (
            <Alert variant={testResult.ok ? "default" : "destructive"} className="mt-3">
              <AlertDescription>{testResult.text}</AlertDescription>
            </Alert>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
