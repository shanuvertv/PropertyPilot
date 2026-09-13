import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";

import type { EmailMessage } from "@/api/types-domain";
import { DataTable, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { ComposeDialog, EmailStatusBadge } from "@/pages/EmailsPage";

export function TenantEmails({ tenantId, tenantEmail }: { tenantId: string; tenantEmail: string | null }) {
  const { api, can } = useApp();
  const [compose, setCompose] = useState(false);
  const emails = useQuery({ queryKey: ["emails", "tenant", tenantId], queryFn: () => api.listEmails({ tenantId, pageSize: 100 }) });
  const columns: Column<EmailMessage>[] = [
    { key: "date", header: "Date", card: "metric", render: (m) => <span className="tabular-nums">{formatDateTime(m.sentAt ?? m.queuedAt)}</span> },
    { key: "to", header: "Recipient", card: "subtitle", render: (m) => m.to.join(", ") },
    { key: "subject", header: "Subject", card: "title", render: (m) => <span className="font-medium">{m.subject}</span> },
    { key: "type", header: "Email type", render: (m) => m.emailType.replace(/_/g, " ").toLowerCase() },
    { key: "by", header: "Sent by", render: (m) => m.sentByName ?? "—" },
    { key: "status", header: "Status", card: "badge", render: (m) => <EmailStatusBadge status={m.status} /> },
  ];
  const [view, setView] = useViewMode("tenant-emails");
  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-end gap-2">
        <ViewToggle value={view} onChange={setView} />
        {can("SEND_NOTICES") && (
          <Button size="sm" variant="outline" onClick={() => setCompose(true)}>
            <Plus data-icon="inline-start" />
            Send email
          </Button>
        )}
      </div>
      <DataTable view={view} columns={columns} rows={emails.data?.items} rowKey={(m) => m.id} loading={emails.isPending} empty="No emails to this tenant yet." />
      <ComposeDialog open={compose} onOpenChange={setCompose} context={{ tenantId, to: tenantEmail ?? undefined }} />
    </div>
  );
}
