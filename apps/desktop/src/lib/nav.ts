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
  Settings,
  Users,
  type LucideIcon,
} from "lucide-react";

import type { Capability } from "@/api/types";

/** Sidebar per spec §20 — one entry per module. `requires` hides it for roles without the view right. */
export interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  requires: Capability;
  /** Phase in PLAN.md §8 that delivers this screen. */
  phase: number;
}

export const NAV: NavItem[] = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, requires: "VIEW_DASHBOARD", phase: 3 },
  { to: "/buildings", label: "Properties / Buildings", icon: Building2, requires: "VIEW_BUILDINGS", phase: 1 },
  { to: "/units", label: "Units", icon: DoorOpen, requires: "VIEW_UNITS", phase: 1 },
  { to: "/tenants", label: "Tenants", icon: Users, requires: "VIEW_TENANTS", phase: 1 },
  { to: "/contracts", label: "Contracts", icon: FileText, requires: "VIEW_CONTRACTS", phase: 2 },
  { to: "/renewals", label: "Renewals", icon: CalendarClock, requires: "VIEW_RENEWALS", phase: 4 },
  { to: "/notices", label: "Renewal Notices", icon: MailCheck, requires: "VIEW_RENEWALS", phase: 5 },
  { to: "/follow-ups", label: "Follow-Ups", icon: ClipboardCheck, requires: "VIEW_FOLLOW_UPS", phase: 4 },
  { to: "/emails", label: "Email Communication", icon: Mail, requires: "VIEW_RENEWALS", phase: 5 },
  { to: "/notifications", label: "Notifications", icon: Bell, requires: "VIEW_DASHBOARD", phase: 6 },
  { to: "/reports", label: "Reports", icon: FileText, requires: "VIEW_REPORTS", phase: 7 },
  { to: "/settings", label: "Settings", icon: Settings, requires: "MANAGE_SETTINGS", phase: 0 },
];
