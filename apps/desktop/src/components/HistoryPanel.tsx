import { useState } from "react";
import { useQuery } from "@tanstack/react-query";

import type { AuditEntry } from "@/api/types-domain";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

function pretty(v: unknown): string {
  if (v === null || v === undefined) return "";
  if (typeof v === "string") return v;
  return JSON.stringify(v, null, 1).replace(/[{}"]/g, "").replace(/,\n/g, "\n").trim();
}

export function actionLabel(a: string): string {
  return a.replace(/_/g, " ").toLowerCase().replace(/^\w/, (c) => c.toUpperCase());
}

/** Spec §19 on a single record: who did what, when, previous value → new value. */
export function HistoryPanel({ entityType, entityId }: { entityType: string; entityId: string }) {
  const { api } = useApp();
  const rows = useQuery({ queryKey: ["audit", entityType, entityId], queryFn: () => api.auditFor(entityType, entityId) });
  return <AuditList rows={rows.data} loading={rows.isPending} />;
}

export function AuditList({ rows, loading }: { rows: AuditEntry[] | undefined; loading?: boolean }) {
  const [open, setOpen] = useState<number | null>(null);
  if (loading && !rows) return <p className="text-[13px] text-muted-foreground">Loading…</p>;
  if (!rows || rows.length === 0) return <p className="text-[13px] text-muted-foreground">No history recorded yet.</p>;
  return (
    <ol className="divide-y rounded-md border bg-card">
      {rows.map((a) => {
        const expanded = open === a.id;
        const hasDiff = a.before !== null || a.after !== null;
        return (
          <li key={a.id}>
            <button
              type="button"
              className={cn("flex w-full items-center gap-3 px-4 py-2.5 text-left text-[13px]", hasDiff && "hover:bg-muted/50")}
              onClick={() => hasDiff && setOpen(expanded ? null : a.id)}
            >
              <span className="w-36 shrink-0 tabular-nums text-muted-foreground">{formatDateTime(a.createdAt)}</span>
              <span className="w-36 shrink-0 truncate">{a.actorName ?? "System"}</span>
              <span className="flex-1 font-medium">{actionLabel(a.action)}</span>
              <span className="text-[11.5px] text-muted-foreground">{a.entityType.replace("_", " ")}</span>
            </button>
            {expanded && (
              <div className="grid gap-3 border-t bg-muted/30 px-4 py-3 text-[12px] md:grid-cols-2">
                <div>
                  <div className="mb-1 text-[11px] font-medium tracking-[0.06em] text-muted-foreground uppercase">Previous value</div>
                  <pre className="max-h-64 overflow-auto font-mono whitespace-pre-wrap text-muted-foreground">{pretty(a.before) || "—"}</pre>
                </div>
                <div>
                  <div className="mb-1 text-[11px] font-medium tracking-[0.06em] text-muted-foreground uppercase">New value</div>
                  <pre className="max-h-64 overflow-auto font-mono whitespace-pre-wrap">{pretty(a.after) || "—"}</pre>
                </div>
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
