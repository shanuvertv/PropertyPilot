import type { Band, ContractStatus, FollowUpStatus, NoticeStatus, RenewalStatus, UnitStatus } from "@/api/types-domain";
import { Badge } from "@/components/ui/badge";
import { BAND_DOT, BAND_LABEL } from "@/lib/bands";
import {
  CONTRACT_STATUS_LABEL,
  FOLLOW_UP_STATUS_LABEL,
  NOTICE_STATUS_LABEL,
  RENEWAL_STATUS_LABEL,
  UNIT_STATUS_LABEL,
  remainingLabel,
} from "@/lib/format";
import { cn } from "@/lib/utils";

export function UnitStatusBadge({ status }: { status: UnitStatus }) {
  const variant = status === "OCCUPIED" ? "default" : status === "VACANT" ? "secondary" : "outline";
  return <Badge variant={variant}>{UNIT_STATUS_LABEL[status]}</Badge>;
}

export function ContractStatusBadge({ status }: { status: ContractStatus }) {
  const variant =
    status === "ACTIVE" ? "default" : status === "DRAFT" ? "outline" : status === "EXPIRED" ? "destructive" : "secondary";
  return <Badge variant={variant}>{CONTRACT_STATUS_LABEL[status]}</Badge>;
}

export function RenewalStatusBadge({ status }: { status: RenewalStatus | null | undefined }) {
  if (!status) return <span className="text-muted-foreground" data-empty="">—</span>;
  const variant =
    status === "RENEWAL_COMPLETED"
      ? "default"
      : status === "TENANT_NOT_RENEWING" || status === "VACATING"
        ? "destructive"
        : status === "CLOSED"
          ? "outline"
          : "secondary";
  return <Badge variant={variant}>{RENEWAL_STATUS_LABEL[status]}</Badge>;
}

export function NoticeStatusBadge({ status }: { status: NoticeStatus | null | undefined }) {
  if (!status) return <span className="text-muted-foreground" data-empty="">—</span>;
  const variant = status === "SENT" || status === "DELIVERED" ? "default" : status === "FAILED" ? "destructive" : "outline";
  return <Badge variant={variant}>{NOTICE_STATUS_LABEL[status]}</Badge>;
}

export function FollowUpStatusBadge({ status }: { status: FollowUpStatus }) {
  return <Badge variant={status === "OPEN" ? "secondary" : "outline"}>{FOLLOW_UP_STATUS_LABEL[status]}</Badge>;
}

/** Coloured dot + remaining-days text — the one expiry scale used everywhere (PLAN.md §3.1). */
export function ExpiryChip({ band, days }: { band: Band | null | undefined; days: number | null | undefined }) {
  if (!band || days === null || days === undefined) return <span className="text-muted-foreground">—</span>;
  return (
    <span className="inline-flex items-center gap-1.5 whitespace-nowrap" title={BAND_LABEL[band]}>
      <span className={cn("size-2 shrink-0 rounded-full", BAND_DOT[band])} aria-hidden="true" />
      <span className={cn("tabular-nums", days < 0 && "text-destructive")}>{days < 0 ? "expired" : remainingLabel(days)}</span>
    </span>
  );
}

export function BandDot({ band, className }: { band: Band; className?: string }) {
  return <span className={cn("inline-block size-2.5 rounded-full", BAND_DOT[band], className)} aria-hidden="true" />;
}
