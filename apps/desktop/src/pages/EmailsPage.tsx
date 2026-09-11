import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, RefreshCw } from "lucide-react";
import { Link } from "react-router";

import type { EmailMessage, EmailStatus } from "@/api/types-domain";
import { DataTable, Paginator, type Column } from "@/components/DataTable";
import { FormDialog, SelectField, TextAreaField, TextField, errorMessage, selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { useListParams } from "@/lib/list-params";

export function EmailStatusBadge({ status }: { status: EmailStatus }) {
  const variant = status === "SENT" || status === "DELIVERED" ? "default" : status === "FAILED" ? "destructive" : "secondary";
  return <Badge variant={variant}>{status.charAt(0) + status.slice(1).toLowerCase()}</Badge>;
}

/** Spec §10: the communication log, with a compose dialog driven by templates. */
export function EmailsPage() {
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const { state, update } = useListParams();
  const [compose, setCompose] = useState(false);
  const [view, setView] = useState<EmailMessage | null>(null);
  const [error, setError] = useState<string | null>(null);

  const query = useQuery({
    queryKey: ["emails", "list", state],
    queryFn: () => api.listEmails({ q: state.q, page: state.page, pageSize: state.pageSize, status: state.filters.status as EmailStatus | undefined, emailType: state.filters.emailType }),
    placeholderData: (prev) => prev,
    refetchInterval: 15_000,
  });
  const templates = useQuery({ queryKey: ["email-templates"], queryFn: () => api.emailTemplates(), staleTime: 60_000 });
  const retry = useMutation({
    mutationFn: (id: string) => api.retryEmail(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["emails"] }),
    onError: (e) => setError(errorMessage(e)),
  });

  const columns: Column<EmailMessage>[] = [
    { key: "date", header: "Date", render: (m) => <span className="tabular-nums">{formatDateTime(m.sentAt ?? m.queuedAt)}</span> },
    { key: "to", header: "Recipient", render: (m) => <span className="line-clamp-1 max-w-[220px]">{m.to.join(", ")}</span> },
    { key: "subject", header: "Subject", render: (m) => <span className="line-clamp-1 max-w-[320px] font-medium">{m.subject}</span> },
    { key: "type", header: "Email type", render: (m) => templates.data?.find((t) => t.key === m.emailType)?.name ?? m.emailType },
    { key: "tenant", header: "Tenant", render: (m) => (m.tenantId ? <Link to={`/tenants/${m.tenantId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{m.tenantName}</Link> : "—") },
    { key: "by", header: "Sent by", render: (m) => m.sentByName ?? "—" },
    { key: "status", header: "Status", render: (m) => <EmailStatusBadge status={m.status} /> },
    {
      key: "actions",
      header: "",
      className: "text-right whitespace-nowrap",
      render: (m) =>
        m.status === "FAILED" && can("SEND_NOTICES") ? (
          <span onClick={(e) => e.stopPropagation()}>
            <Button size="xs" variant="outline" onClick={() => retry.mutate(m.id)}>
              <RefreshCw data-icon="inline-start" />
              Retry
            </Button>
          </span>
        ) : null,
    },
  ];

  return (
    <>
      <PageHeader
        title="Email Communication"
        description="Every email the system sent or is about to send: renewal notices, reminders, confirmations. Failed sends can be retried."
        actions={
          can("SEND_NOTICES") && (
            <Button onClick={() => setCompose(true)}>
              <Plus data-icon="inline-start" />
              Compose
            </Button>
          )
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-3">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search subject, tenant or recipient" />
        <select className={`${selectClass} w-auto`} value={state.filters.status ?? ""} onChange={(e) => update({ filters: { status: e.target.value } })} aria-label="Status">
          <option value="">Any status</option>
          {(["QUEUED", "SENT", "DELIVERED", "FAILED"] as EmailStatus[]).map((s) => (
            <option key={s} value={s}>
              {s.charAt(0) + s.slice(1).toLowerCase()}
            </option>
          ))}
        </select>
        <select className={`${selectClass} w-auto`} value={state.filters.emailType ?? ""} onChange={(e) => update({ filters: { emailType: e.target.value } })} aria-label="Email type">
          <option value="">Any type</option>
          {templates.data?.map((t) => (
            <option key={t.key} value={t.key}>
              {t.name}
            </option>
          ))}
          <option value="CUSTOM">Custom</option>
        </select>
      </div>
      <DataTable columns={columns} rows={query.data?.items} rowKey={(m) => m.id} loading={query.isPending} error={query.error ? "Could not load emails." : null} empty="No emails yet." onRowClick={(m) => setView(m)} />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}

      <ComposeDialog open={compose} onOpenChange={setCompose} />

      <Dialog open={view !== null} onOpenChange={(o) => !o && setView(null)}>
        <DialogContent className="sm:max-w-2xl">
          {view && (
            <>
              <DialogHeader>
                <DialogTitle>{view.subject}</DialogTitle>
                <DialogDescription>
                  To {view.to.join(", ")}
                  {view.cc.length > 0 && ` · cc ${view.cc.join(", ")}`} · {view.sentByName ?? "—"} · {formatDateTime(view.sentAt ?? view.queuedAt)}
                  {view.attachmentCount > 0 && ` · ${view.attachmentCount} attachment(s)`}
                </DialogDescription>
              </DialogHeader>
              <div className="flex items-center gap-2 text-[12.5px]">
                <EmailStatusBadge status={view.status} />
                {view.provider && <span className="text-muted-foreground">via {view.provider}</span>}
                {view.lastError && <span className="text-destructive">{view.lastError}</span>}
              </div>
              <pre className="max-h-[50vh] overflow-auto rounded-md border bg-muted/30 p-3 font-sans text-[13px] whitespace-pre-wrap">{view.bodyText}</pre>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}

export function ComposeDialog({
  open,
  onOpenChange,
  context,
}: {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  /** Pre-fills recipient and links the log entry to a tenant / contract / case. */
  context?: { tenantId?: string; contractId?: string; caseId?: string; to?: string };
}) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const templates = useQuery({ queryKey: ["email-templates"], queryFn: () => api.emailTemplates(), staleTime: 60_000 });
  const [form, setForm] = useState({ template: "", to: "", cc: "", subject: "", body: "" });

  useEffect(() => {
    if (open) setForm({ template: "", to: context?.to ?? "", cc: "", subject: "", body: "" });
  }, [open, context?.to]);

  async function applyTemplate(key: string) {
    setForm((f) => ({ ...f, template: key }));
    if (!key) return;
    try {
      const p = await api.previewTemplate(key, { caseId: context?.caseId ?? null, subject: null, bodyText: null });
      setForm((f) => ({ ...f, subject: p.subject, body: p.bodyText, to: f.to || p.recipient || "" }));
    } catch {
      // keep whatever the user typed
    }
  }

  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Compose email"
      description={context?.caseId ? "Placeholders are filled from this renewal case." : "Pick a template to start from, or write a custom message."}
      submitLabel="Queue for sending"
      wide
      onSubmit={async () => {
        await api.composeEmail({
          emailType: form.template || "CUSTOM",
          tenantId: context?.tenantId ?? null,
          contractId: context?.contractId ?? null,
          caseId: context?.caseId ?? null,
          to: form.to.split(/[,;]/).map((s) => s.trim()).filter(Boolean),
          cc: form.cc.split(/[,;]/).map((s) => s.trim()).filter(Boolean),
          subject: form.subject,
          bodyText: form.body,
          attachmentDocumentIds: [],
        });
        await queryClient.invalidateQueries({ queryKey: ["emails"] });
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField id="ce-template" label="Template" value={form.template} onChange={(v) => void applyTemplate(v)} options={(templates.data ?? []).filter((t) => t.active).map((t) => ({ value: t.key, label: t.name }))} placeholder="Custom message" className="sm:col-span-2" />
        <TextField id="ce-to" label="To" value={form.to} onChange={(v) => setForm({ ...form, to: v })} required placeholder="comma-separated" />
        <TextField id="ce-cc" label="CC" value={form.cc} onChange={(v) => setForm({ ...form, cc: v })} placeholder="comma-separated" />
        <TextField id="ce-subject" label="Subject" value={form.subject} onChange={(v) => setForm({ ...form, subject: v })} required className="sm:col-span-2" />
        <TextAreaField id="ce-body" label="Message" value={form.body} onChange={(v) => setForm({ ...form, body: v })} rows={10} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}
