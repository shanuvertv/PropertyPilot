import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FileDown, Mail, Save } from "lucide-react";

import type { Notice } from "@/api/types-domain";
import { NoticeStatusBadge } from "@/components/badges";
import { saveBlob } from "@/components/DocumentsPanel";
import { FormDialog, TextAreaField, TextField, errorMessage, opt } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";

/** Spec §7 steps 3–4: prepare → edit → save draft → preview → PDF → send by email. */
export function NoticePanel({ caseId, open, onChanged }: { caseId: string; open: boolean; onChanged: () => Promise<void> }) {
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const ws = useQuery({ queryKey: ["notice", caseId], queryFn: () => api.noticeWorkspace(caseId) });
  const [form, setForm] = useState({ subject: "", bodyText: "", proposedPeriod: "", otherTerms: "", recipient: "", cc: "" });
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sendDlg, setSendDlg] = useState(false);
  const [sendForm, setSendForm] = useState({ to: "", cc: "", cover: "" });
  const canSend = can("SEND_NOTICES") && open;

  // Load the draft, or the freshly prepared letter when there is none.
  useEffect(() => {
    const d = ws.data;
    if (!d || dirty) return;
    setForm({
      subject: d.draft?.subject ?? d.prepared.subject,
      bodyText: d.draft?.bodyText ?? d.prepared.bodyText,
      proposedPeriod: d.draft?.proposedPeriod ?? d.proposedPeriod,
      otherTerms: d.draft?.otherTerms ?? d.otherTerms,
      recipient: d.draft?.recipient ?? d.prepared.recipient ?? "",
      cc: (d.draft?.cc ?? []).join(", "),
    });
  }, [ws.data, dirty]);

  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["notice", caseId] });
    await onChanged();
  };

  const save = useMutation({
    mutationFn: () =>
      api.saveNoticeDraft(caseId, {
        subject: form.subject,
        bodyText: form.bodyText,
        proposedPeriod: opt(form.proposedPeriod),
        otherTerms: opt(form.otherTerms),
        recipient: opt(form.recipient),
        cc: form.cc.split(/[,;]/).map((s) => s.trim()).filter(Boolean),
      }),
    onSuccess: async () => {
      setDirty(false);
      setError(null);
      await refresh();
    },
    onError: (e) => setError(errorMessage(e)),
  });

  const regenerate = useMutation({
    mutationFn: () => api.noticeWorkspace(caseId),
    onSuccess: (d) => {
      setForm((f) => ({ ...f, subject: d.prepared.subject, bodyText: d.prepared.bodyText }));
      setDirty(true);
    },
  });

  const pdf = useMutation({
    mutationFn: async () => {
      if (dirty) await save.mutateAsync();
      const n = await api.generateNoticePdf(caseId);
      if (n.pdfDocumentId) {
        const blob = await api.downloadDocument(n.pdfDocumentId);
        await saveBlob(blob, `Renewal Notice ${n.subject.slice(0, 40)}.pdf`);
      }
      return n;
    },
    onSuccess: () => void refresh(),
    onError: (e) => setError(errorMessage(e)),
  });

  const history: Notice[] = ws.data?.history ?? [];
  const patch = (p: Partial<typeof form>) => {
    setForm((f) => ({ ...f, ...p }));
    setDirty(true);
  };

  return (
    <Card className="py-4">
      <CardHeader className="flex flex-row items-center justify-between px-5">
        <CardTitle className="flex items-center gap-2 text-[13px]">
          <Mail className="size-4" /> Renewal notice
          {ws.data?.draft && <NoticeStatusBadge status="DRAFT" />}
        </CardTitle>
        {canSend && (
          <div className="flex gap-1.5">
            <Button size="sm" variant="ghost" onClick={() => regenerate.mutate()} title="Re-populate the letter from the template and current contract data">
              Re-populate
            </Button>
            <Button size="sm" variant="outline" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
              <Save data-icon="inline-start" />
              {save.isPending ? "Saving…" : "Save draft"}
            </Button>
            <Button size="sm" variant="outline" onClick={() => pdf.mutate()} disabled={pdf.isPending}>
              <FileDown data-icon="inline-start" />
              {pdf.isPending ? "Generating…" : "PDF"}
            </Button>
            <Button
              size="sm"
              onClick={() => {
                setSendForm({ to: form.recipient, cc: form.cc, cover: "" });
                setSendDlg(true);
              }}
            >
              Send by email…
            </Button>
          </div>
        )}
      </CardHeader>
      <CardContent className="px-5">
        {error && (
          <Alert variant="destructive" className="mb-3">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        {ws.isPending ? (
          <p className="text-[13px] text-muted-foreground">Preparing the notice…</p>
        ) : (
          <div className="grid gap-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <TextField id="n-period" label="Proposed renewal period" value={form.proposedPeriod} onChange={(v) => patch({ proposedPeriod: v })} hint="Used by the {{ProposedRenewalPeriod}} placeholder; press Re-populate to apply." />
              <TextField id="n-terms" label="Other renewal terms" value={form.otherTerms} onChange={(v) => patch({ otherTerms: v })} placeholder="e.g. rent unchanged; two cheques" />
            </div>
            <TextField id="n-subject" label="Subject" value={form.subject} onChange={(v) => patch({ subject: v })} />
            <div className="flex flex-col gap-1.5">
              <label htmlFor="n-body" className="text-sm font-medium">
                Letter
              </label>
              <Textarea id="n-body" rows={12} value={form.bodyText} onChange={(e) => patch({ bodyText: e.target.value })} className="font-mono text-[12.5px]" disabled={!canSend} />
              <span className="text-[12px] text-muted-foreground">Blank lines separate paragraphs. The PDF adds the letterhead, date, recipient block and signature automatically.</span>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <TextField id="n-to" label="Recipient" value={form.recipient} onChange={(v) => patch({ recipient: v })} placeholder="tenant@example.com" />
              <TextField id="n-cc" label="CC" value={form.cc} onChange={(v) => patch({ cc: v })} placeholder="comma-separated" />
            </div>
          </div>
        )}

        {history.length > 0 && (
          <div className="mt-5">
            <h4 className="mb-2 text-[11px] font-medium tracking-[0.08em] text-muted-foreground uppercase">Sent notices</h4>
            <ul className="divide-y rounded-md border">
              {history.map((n) => (
                <li key={n.id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                  <NoticeStatusBadge status={n.status} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate font-medium">{n.subject}</div>
                    <div className="text-[12px] text-muted-foreground">
                      to {n.recipient ?? "—"}
                      {n.cc.length > 0 && ` · cc ${n.cc.join(", ")}`} · {n.sentByName ?? "—"} · {formatDateTime(n.sentAt)}
                      {n.emailStatus && ` · email ${n.emailStatus.toLowerCase()}`}
                    </div>
                  </div>
                  {n.pdfDocumentId && (
                    <Button
                      size="xs"
                      variant="ghost"
                      onClick={async () => {
                        const blob = await api.downloadDocument(n.pdfDocumentId!);
                        await saveBlob(blob, `Renewal Notice ${formatDateTime(n.sentAt).replace(/[/:,]/g, "-")}.pdf`);
                      }}
                    >
                      PDF
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}
      </CardContent>

      <FormDialog
        open={sendDlg}
        onOpenChange={setSendDlg}
        title="Send renewal notice"
        description="The letter is generated as a PDF and attached. The email body below is the cover message; leave it empty to use the letter text."
        submitLabel="Send"
        onSubmit={async () => {
          if (dirty) await save.mutateAsync();
          await api.sendNotice(caseId, {
            to: sendForm.to.split(/[,;]/).map((s) => s.trim()).filter(Boolean),
            cc: sendForm.cc.split(/[,;]/).map((s) => s.trim()).filter(Boolean),
            emailBodyText: opt(sendForm.cover),
          });
          setDirty(false);
          await refresh();
        }}
      >
        <TextField id="s-to" label="To" value={sendForm.to} onChange={(v) => setSendForm({ ...sendForm, to: v })} required />
        <TextField id="s-cc" label="CC" value={sendForm.cc} onChange={(v) => setSendForm({ ...sendForm, cc: v })} placeholder="comma-separated" />
        <TextAreaField id="s-cover" label="Cover message (optional)" value={sendForm.cover} onChange={(v) => setSendForm({ ...sendForm, cover: v })} rows={4} />
        <div className="rounded-md border bg-muted/40 p-3 text-[12.5px]">
          <div className="font-medium">{form.subject}</div>
          <div className="mt-1 whitespace-pre-wrap text-muted-foreground">{form.bodyText.slice(0, 400)}{form.bodyText.length > 400 ? "…" : ""}</div>
        </div>
      </FormDialog>
    </Card>
  );
}
