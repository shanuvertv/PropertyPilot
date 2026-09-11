import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { EmailTemplate } from "@/api/types-domain";
import { errorMessage, selectClass } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useApp } from "@/lib/app-state";

/** Spec §10: editable templates with dynamic fields. */
export function EmailTemplatesCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const templates = useQuery({ queryKey: ["email-templates"], queryFn: () => api.emailTemplates() });
  const placeholders = useQuery({ queryKey: ["placeholders"], queryFn: () => api.placeholders(), staleTime: Infinity });
  const mail = useQuery({ queryKey: ["mail-status"], queryFn: () => api.mailStatus(), refetchInterval: 30_000 });
  const [key, setKey] = useState("");
  const [form, setForm] = useState({ name: "", subject: "", bodyText: "", active: true });
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ subject: string; bodyText: string } | null>(null);

  const current: EmailTemplate | undefined = templates.data?.find((t) => t.key === key) ?? templates.data?.[0];
  useEffect(() => {
    if (current && !dirty) {
      setKey(current.key);
      setForm({ name: current.name, subject: current.subject, bodyText: current.bodyText, active: current.active });
      setPreview(null);
    }
  }, [current, dirty]);

  const save = useMutation({
    mutationFn: () => api.updateEmailTemplate(key, form),
    onSuccess: () => {
      setDirty(false);
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["email-templates"] });
    },
    onError: (e) => setError(errorMessage(e)),
  });
  const doPreview = useMutation({
    mutationFn: () => api.previewTemplate(key, { caseId: null, subject: form.subject, bodyText: form.bodyText }),
    onSuccess: (p) => {
      setError(null);
      setPreview({ subject: p.subject, bodyText: p.bodyText });
    },
    onError: (e) => setError(errorMessage(e)),
  });

  const patch = (p: Partial<typeof form>) => {
    setForm((f) => ({ ...f, ...p }));
    setDirty(true);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Email templates</CardTitle>
        <CardDescription>
          Sending via <b>{mail.data?.provider ?? "…"}</b> from {mail.data?.sender ?? "…"} · {mail.data?.queued ?? 0} queued · {mail.data?.failed ?? 0} failed. Templates use placeholders like{" "}
          <code className="rounded bg-muted px-1">{"{{TenantName}}"}</code>.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_260px]">
        <div className="flex flex-col gap-3">
          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tpl-key">Template</Label>
            <select
              id="tpl-key"
              className={selectClass}
              value={key}
              onChange={(e) => {
                if (dirty && !window.confirm("Discard unsaved changes?")) return;
                setDirty(false);
                setKey(e.target.value);
              }}
            >
              {templates.data?.map((t) => (
                <option key={t.key} value={t.key}>
                  {t.name}
                  {t.active ? "" : " (inactive)"}
                </option>
              ))}
            </select>
          </div>
          <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto]">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="tpl-name">Name</Label>
              <Input id="tpl-name" value={form.name} onChange={(e) => patch({ name: e.target.value })} />
            </div>
            <label className="flex items-center gap-2 self-end pb-1.5 text-[13px]">
              <Checkbox checked={form.active} onCheckedChange={(c) => patch({ active: c === true })} />
              Active
            </label>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tpl-subject">Subject</Label>
            <Input id="tpl-subject" value={form.subject} onChange={(e) => patch({ subject: e.target.value })} />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="tpl-body">Body</Label>
            <Textarea id="tpl-body" rows={12} value={form.bodyText} onChange={(e) => patch({ bodyText: e.target.value })} className="font-mono text-[12.5px]" />
          </div>
          <div className="flex items-center gap-2">
            <Button size="sm" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
              {save.isPending ? "Saving…" : "Save template"}
            </Button>
            <Button size="sm" variant="outline" onClick={() => doPreview.mutate()}>
              Preview with sample data
            </Button>
          </div>
          {preview && (
            <div className="rounded-md border bg-muted/30 p-3 text-[13px]">
              <div className="font-medium">{preview.subject}</div>
              <pre className="mt-2 font-sans whitespace-pre-wrap text-muted-foreground">{preview.bodyText}</pre>
            </div>
          )}
        </div>
        <div>
          <div className="mb-2 text-[11px] font-medium tracking-[0.08em] text-muted-foreground uppercase">Placeholders</div>
          <ul className="flex flex-col gap-1 text-[12.5px]">
            {placeholders.data?.map((p) => (
              <li key={p.name}>
                <button type="button" className="font-mono text-primary hover:underline" onClick={() => patch({ bodyText: `${form.bodyText}{{${p.name}}}` })} title="Append to body">
                  {`{{${p.name}}}`}
                </button>
                <span className="ml-1 text-muted-foreground">{p.description}</span>
              </li>
            ))}
          </ul>
        </div>
      </CardContent>
    </Card>
  );
}
