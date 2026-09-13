// Hand-mirrored from crates/api/src/lib.rs and crates/core (Role, Capability, Thresholds).
// Keep in sync until the TypeScript export is wired (Phase 1 follow-up).

export type Role = "ADMIN" | "LEASING" | "OPERATIONS" | "MANAGEMENT";

export type Capability =
  | "VIEW_DASHBOARD"
  | "VIEW_BUILDINGS"
  | "MANAGE_BUILDINGS"
  | "VIEW_UNITS"
  | "MANAGE_UNITS"
  | "UPDATE_UNIT_STATUS"
  | "VIEW_TENANTS"
  | "MANAGE_TENANTS"
  | "VIEW_CONTRACTS"
  | "MANAGE_CONTRACTS"
  | "VIEW_RENEWALS"
  | "MANAGE_RENEWALS"
  | "SEND_NOTICES"
  | "VIEW_FOLLOW_UPS"
  | "MANAGE_FOLLOW_UPS"
  | "VIEW_REPORTS"
  | "MANAGE_SETTINGS"
  | "MANAGE_USERS"
  | "VIEW_AUDIT_TRAIL"
  | "MANAGE_OCCUPANTS"
  | "VIEW_EXPENSES"
  | "MANAGE_EXPENSES";

export const ROLES: Role[] = ["ADMIN", "LEASING", "OPERATIONS", "MANAGEMENT"];

export const ROLE_LABEL: Record<Role, string> = {
  ADMIN: "Admin",
  LEASING: "Leasing Team",
  OPERATIONS: "Operations Team",
  MANAGEMENT: "Management",
};

export type ErrorCode =
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "VALIDATION"
  | "CONFLICT"
  | "INVALID_CREDENTIALS"
  | "RATE_LIMITED"
  | "INTERNAL"
  | "SERVICE_UNAVAILABLE";

export interface ApiError {
  code: ErrorCode;
  message: string;
}

export interface ErrorBody {
  error: ApiError;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface SessionInfo {
  userId: string;
  name: string;
  email: string;
  role: Role;
  capabilities: Capability[];
  expiresAt: string;
}

export interface LoginResponse {
  token: string;
  session: SessionInfo;
}

export interface BootstrapRequest {
  name: string;
  email: string;
  password: string;
}

export interface UserSummary {
  id: string;
  name: string;
  email: string;
  role: Role;
  active: boolean;
  createdAt: string;
  lastLoginAt: string | null;
}

export interface CreateUserRequest {
  name: string;
  email: string;
  role: Role;
  password: string;
}

export interface SetUserActiveRequest {
  active: boolean;
}

export interface HealthResponse {
  ok: boolean;
  version: string;
  database: boolean;
  usersExist: boolean;
}

export interface Thresholds {
  expiringSoonDays: number;
  urgentDays: number;
}

export interface SchedulerStatus {
  version: string;
  hostname: string;
  lastHeartbeatAt: string;
  lastSweepAt: string | null;
  stale: boolean;
}

export interface SystemStatus {
  orgName: string;
  timezone: string;
  thresholds: Thresholds;
  scheduler: SchedulerStatus | null;
}
