import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Bell, CalendarClock, CheckCheck, Clock, Mail, MessageSquare, UserCheck, XCircle } from "lucide-react";
import { useNavigate } from "react-router";

import type { Notification, NotificationKind } from "@/api/types-domain";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

/** Spec §15 colours: 🔴 expiring / overdue, 🟠 notice pending, 🟡 response pending, 🔵 follow-up today, ⚫ expired. */
const STYLE: Record<NotificationKind, { icon: typeof Bell; dot: string; label: string }> = {
  CONTRACT_EXPIRING_SOON: { icon: AlertTriangle, dot: "bg-red-600", label: "Contract expiring soon" },
  RENEWAL_NOTICE_PENDING: { icon: Mail, dot: "bg-orange-500", label: "Renewal notice pending" },
  TENANT_RESPONSE_PENDING: { icon: MessageSquare, dot: "bg-yellow-500", label: "Tenant response pending" },
  FOLLOW_UP_DUE_TODAY: { icon: Clock, dot: "bg-blue-600", label: "Follow-up due today" },
  OVERDUE_FOLLOW_UP: { icon: AlertTriangle, dot: "bg-red-600", label: "Overdue follow-up" },
  CONTRACT_EXPIRED: { icon: XCircle, dot: "bg-neutral-800", label: "Contract expired" },
  RENEWAL_REMINDER: { icon: CalendarClock, dot: "bg-green-600", label: "Renewal reminder" },
  RENEWAL_COMPLETED: { icon: CheckCheck, dot: "bg-green-600", label: "Renewal completed" },
  CASE_ASSIGNED: { icon: UserCheck, dot: "bg-blue-600", label: "Case assigned to you" },
};

export function targetOf(n: Notification): string {
  switch (n.entityType) {
    case "renewal_case":
      return `/renewals/${n.entityId}`;
    case "contract":
      return `/contracts/${n.entityId}`;
    case "tenant":
      return `/tenants/${n.entityId}`;
    case "unit":
      return `/units`;
  }
}

export function NotificationsPage() {
  const { api } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const list = useQuery({ queryKey: ["notifications", "list"], queryFn: () => api.notifications(false), refetchInterval: 30_000 });
  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["notifications"] });
  const markRead = useMutation({ mutationFn: (id: string) => api.markRead(id), onSuccess: () => void invalidate() });
  const markAll = useMutation({ mutationFn: () => api.markAllRead(), onSuccess: () => void invalidate() });

  const unread = list.data?.filter((n) => !n.readAt).length ?? 0;

  return (
    <>
      <PageHeader
        title="Notifications"
        description="Expiring contracts, pending notices and responses, follow-ups due, and cases assigned to you. Click one to open the record."
        actions={
          unread > 0 && (
            <Button variant="outline" onClick={() => markAll.mutate()} disabled={markAll.isPending}>
              <CheckCheck data-icon="inline-start" />
              Mark all read ({unread})
            </Button>
          )
        }
      />
      {list.data?.length === 0 && <p className="text-[13.5px] text-muted-foreground">Nothing yet. The daily sweep and your colleagues will fill this in.</p>}
      <ul className="flex flex-col gap-1.5">
        {list.data?.map((n) => {
          const s = STYLE[n.kind] ?? STYLE.RENEWAL_REMINDER;
          const Icon = s.icon;
          return (
            <li key={n.id}>
              <button
                type="button"
                className={cn(
                  "flex w-full items-start gap-3 rounded-md border bg-card px-4 py-3 text-left transition-colors hover:bg-muted/50",
                  !n.readAt && "border-primary/40",
                )}
                onClick={() => {
                  if (!n.readAt) markRead.mutate(n.id);
                  navigate(targetOf(n));
                }}
              >
                <span className={cn("mt-1 size-2.5 shrink-0 rounded-full", s.dot)} aria-hidden="true" />
                <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                <span className="min-w-0 flex-1">
                  <span className={cn("block text-[13.5px]", !n.readAt && "font-semibold")}>{n.title}</span>
                  {n.body && <span className="block text-[12.5px] text-muted-foreground">{n.body}</span>}
                  <span className="block text-[11.5px] text-muted-foreground">
                    {s.label} · {formatDateTime(n.createdAt)}
                  </span>
                </span>
                {!n.readAt && <span className="text-[11px] font-medium text-primary">New</span>}
              </button>
            </li>
          );
        })}
      </ul>
    </>
  );
}
