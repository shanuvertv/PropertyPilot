import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { ComposeDialog, EmailStatusBadge } from "@/pages/EmailsPage";

/** Communication history for one case, with compose-from-template (spec §10). */
export function CaseEmails({ caseId, tenantId, contractId, tenantEmail }: { caseId: string; tenantId: string; contractId: string; tenantEmail: string | null }) {
  const { api, can } = useApp();
  const [compose, setCompose] = useState(false);
  const emails = useQuery({ queryKey: ["emails", "case", caseId], queryFn: () => api.listEmails({ caseId, pageSize: 50 }), refetchInterval: 15_000 });
  return (
    <Card className="py-4">
      <CardHeader className="flex flex-row items-center justify-between px-5">
        <CardTitle className="text-[13px]">Email communication</CardTitle>
        {can("SEND_NOTICES") && (
          <Button size="sm" variant="outline" onClick={() => setCompose(true)}>
            <Plus data-icon="inline-start" />
            Send email
          </Button>
        )}
      </CardHeader>
      <CardContent className="px-5">
        {emails.data?.items.length === 0 && <p className="text-[13px] text-muted-foreground">No emails for this case yet.</p>}
        <ul className="divide-y">
          {emails.data?.items.map((m) => (
            <li key={m.id} className="flex items-center gap-3 py-2 text-[13.5px]">
              <span className="w-36 shrink-0 tabular-nums text-muted-foreground">{formatDateTime(m.sentAt ?? m.queuedAt)}</span>
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium">{m.subject}</div>
                <div className="truncate text-[12px] text-muted-foreground">
                  to {m.to.join(", ")} · {m.emailType.replace(/_/g, " ").toLowerCase()} · {m.sentByName ?? "—"}
                  {m.lastError && <span className="text-destructive"> · {m.lastError}</span>}
                </div>
              </div>
              <EmailStatusBadge status={m.status} />
            </li>
          ))}
        </ul>
      </CardContent>
      <ComposeDialog open={compose} onOpenChange={setCompose} context={{ caseId, tenantId, contractId, to: tenantEmail ?? undefined }} />
    </Card>
  );
}
