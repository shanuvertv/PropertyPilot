import {
  Bell,
  Building2,
  CalendarClock,
  ClipboardCheck,
  DoorOpen,
  FileText,
  LayoutDashboard,
  Mail,
  MailCheck,
  Receipt,
  Settings,
  Users,
  type LucideIcon,
} from "lucide-react";

import type { Capability } from "@/api/types";

/** Sidebar per spec §20 — one entry per module. `requires` hides it for roles without the view right. */
export interface NavItem {
  to: string;
  label: string;
  /** Label for the phone's bottom tabs, where there is room for one word. */
  short?: string;
  icon: LucideIcon;
  requires: Capability;
  /** Phase in PLAN.md §8 that delivers this screen. */
  phase: number;
}

export const NAV: NavItem[] = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, requires: "VIEW_DASHBOARD", phase: 3 },
  { to: "/buildings", label: "Properties / Buildings", short: "Buildings", icon: Building2, requires: "VIEW_BUILDINGS", phase: 1 },
  { to: "/units", label: "Units", icon: DoorOpen, requires: "VIEW_UNITS", phase: 1 },
  { to: "/tenants", label: "Tenants", icon: Users, requires: "VIEW_TENANTS", phase: 1 },
  { to: "/contracts", label: "Contracts", icon: FileText, requires: "VIEW_CONTRACTS", phase: 2 },
  { to: "/renewals", label: "Renewals", icon: CalendarClock, requires: "VIEW_RENEWALS", phase: 4 },
  { to: "/notices", label: "Renewal Notices", short: "Notices", icon: MailCheck, requires: "VIEW_RENEWALS", phase: 5 },
  { to: "/follow-ups", label: "Follow-Ups", icon: ClipboardCheck, requires: "VIEW_FOLLOW_UPS", phase: 4 },
  { to: "/emails", label: "Email Communication", short: "Email", icon: Mail, requires: "VIEW_RENEWALS", phase: 5 },
  { to: "/expenses", label: "Expenses", icon: Receipt, requires: "VIEW_EXPENSES", phase: 10 },
  { to: "/notifications", label: "Notifications", icon: Bell, requires: "VIEW_DASHBOARD", phase: 6 },
  { to: "/reports", label: "Reports", icon: FileText, requires: "VIEW_REPORTS", phase: 7 },
  { to: "/settings", label: "Settings", icon: Settings, requires: "MANAGE_SETTINGS", phase: 0 },
];

/** Preferred order for the phone's bottom tabs; the first four the role can see are shown. */
const PHONE_TAB_ORDER = ["/", "/units", "/renewals", "/follow-ups", "/contracts", "/tenants", "/buildings", "/reports"];

/**
 * Where "/" should land for a role that cannot see the dashboard: the most-used
 * module it may open (Units for Operations), falling back to sidebar order.
 */
export function homeFor(can: (cap: Capability) => boolean): NavItem | undefined {
  const allowed = NAV.filter((n) => n.to !== "/" && can(n.requires));
  return PHONE_TAB_ORDER.map((to) => allowed.find((i) => i.to === to)).find((i) => i !== undefined) ?? allowed[0];
}

/** The bottom tabs for a role: up to four of the modules it may use, most-used first. */
export function phoneTabs(visible: NavItem[]): NavItem[] {
  return PHONE_TAB_ORDER.map((to) => visible.find((i) => i.to === to))
    .filter((i): i is NavItem => i !== undefined)
    .slice(0, 4);
}
