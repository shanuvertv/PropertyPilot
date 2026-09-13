import type {
  Band,
  ChequeStatus,
  ContractStatus,
  ExpenseCategory,
  FollowUpStatus,
  FollowUpType,
  NoticeStatus,
  RenewalStatus,
  SplitMethod,
  TenantResponse,
  UnitStatus,
} from "@/api/types-domain";

export const APP_NAME = "PropertyPilot";

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso.length === 10 ? `${iso}T00:00:00` : iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString(undefined, { day: "2-digit", month: "short", year: "numeric" });
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, { day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function todayIso(): string {
  const d = new Date();
  const m = `${d.getMonth() + 1}`.padStart(2, "0");
  const day = `${d.getDate()}`.padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

export function addDaysIso(iso: string, days: number): string {
  const d = new Date(`${iso}T00:00:00`);
  d.setDate(d.getDate() + days);
  const m = `${d.getMonth() + 1}`.padStart(2, "0");
  const day = `${d.getDate()}`.padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/** "in 12 days" / "today" / "5 days ago" for remaining-day counters. */
export function remainingLabel(days: number | null | undefined): string {
  if (days === null || days === undefined) return "—";
  if (days === 0) return "today";
  if (days < 0) return `${-days} day${days === -1 ? "" : "s"} ago`;
  return `${days} day${days === 1 ? "" : "s"}`;
}

export const UNIT_STATUS_LABEL: Record<UnitStatus, string> = {
  VACANT: "Vacant",
  OCCUPIED: "Occupied",
  RESERVED: "Reserved",
  MAINTENANCE: "Under Maintenance",
};

export const CONTRACT_STATUS_LABEL: Record<ContractStatus, string> = {
  DRAFT: "Draft",
  ACTIVE: "Active",
  RENEWED: "Renewed",
  EXPIRED: "Expired",
  TERMINATED: "Terminated",
};

export const RENEWAL_STATUS_LABEL: Record<RenewalStatus, string> = {
  NOT_STARTED: "Not Started",
  NOTICE_PENDING: "Notice Pending",
  NOTICE_SENT: "Notice Sent",
  WAITING_FOR_TENANT_RESPONSE: "Waiting for Tenant Response",
  TENANT_INTERESTED: "Tenant Interested",
  UNDER_NEGOTIATION: "Under Negotiation",
  RENEWAL_CONFIRMED: "Renewal Confirmed",
  RENEWAL_COMPLETED: "Renewal Completed",
  TENANT_NOT_RENEWING: "Tenant Not Renewing",
  VACATING: "Vacating",
  CLOSED: "Closed",
};

export const NOTICE_STATUS_LABEL: Record<NoticeStatus, string> = {
  NOT_REQUIRED: "Not Required",
  PENDING: "Pending",
  DRAFT: "Draft",
  SENT: "Sent",
  DELIVERED: "Delivered",
  FAILED: "Failed",
};

export const TENANT_RESPONSE_LABEL: Record<TenantResponse, string> = {
  NO_RESPONSE: "No Response",
  INTERESTED_IN_RENEWAL: "Interested in Renewal",
  RENEWAL_CONFIRMED: "Renewal Confirmed",
  UNDER_DISCUSSION: "Under Discussion",
  NOT_INTERESTED: "Not Interested",
  WILL_VACATE: "Will Vacate",
};

export const FOLLOW_UP_TYPE_LABEL: Record<FollowUpType, string> = {
  PHONE_CALL: "Phone Call",
  EMAIL: "Email",
  MEETING: "Meeting",
  WHATS_APP: "WhatsApp",
  INTERNAL_DISCUSSION: "Internal Discussion",
};

export const FOLLOW_UP_STATUS_LABEL: Record<FollowUpStatus, string> = {
  OPEN: "Open",
  DONE: "Done",
  CANCELLED: "Cancelled",
};

export const BAND_ORDER: Band[] = ["EXPIRED", "DAYS_0_TO_30", "DAYS_31_TO_60", "DAYS_61_TO_90", "DAYS_91_TO_120", "BEYOND_120"];

export function keys<T extends string>(rec: Record<T, string>): T[] {
  return Object.keys(rec) as T[];
}

export const EXPENSE_CATEGORY_LABEL: Record<ExpenseCategory, string> = {
  ELECTRICITY: "Electricity",
  WATER: "Water",
  GAS: "Gas",
  INTERNET: "Internet",
  MAINTENANCE: "Maintenance",
  CLEANING: "Cleaning",
  MUNICIPALITY: "Municipality",
  OTHER: "Other",
};

export const SPLIT_METHOD_LABEL: Record<SplitMethod, string> = {
  NONE: "Unit cost (not split)",
  EQUAL: "Split equally between the occupants",
};

export const CURRENCY = "AED";

/** `1234.5` → "AED 1,234.50" (the app's single currency). */
export function formatMoney(amount: number | null | undefined, opts?: { compact?: boolean }): string {
  if (amount === null || amount === undefined || Number.isNaN(amount)) return "—";
  if (opts?.compact && Math.abs(amount) >= 10_000) {
    return `${CURRENCY} ${new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(amount)}`;
  }
  return `${CURRENCY} ${new Intl.NumberFormat(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(amount)}`;
}

/** "2026-09-01" → "Sep 2026" for month axes. */
export function formatMonth(iso: string): string {
  const d = new Date(iso + "T00:00:00");
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(undefined, { month: "short", year: "numeric" });
}

export const CHEQUE_STATUS_LABEL: Record<ChequeStatus, string> = {
  PENDING: "Pending",
  DEPOSITED: "Deposited",
  CLEARED: "Cleared",
  BOUNCED: "Bounced",
  CANCELLED: "Cancelled",
};
