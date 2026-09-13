// Hand-mirrored from crates/api/src/master.rs and crates/api/src/contracts.rs.
// Keep in sync until the TypeScript export is wired (Phase 1 follow-up in PLAN.md).

import type { Role } from "./types";

// ---------------------------------------------------------------- list envelope

export interface Page<T> {
  items: T[];
  page: number;
  pageSize: number;
  total: number;
}

export interface ListParams {
  q?: string;
  page?: number;
  pageSize?: number;
  sort?: string;
  dir?: "asc" | "desc";
}

// ---------------------------------------------------------------- enums (crates/core)

export type UnitStatus = "VACANT" | "OCCUPIED" | "RESERVED" | "MAINTENANCE";
export type ContractStatus = "DRAFT" | "ACTIVE" | "RENEWED" | "EXPIRED" | "TERMINATED";
export type RenewalStatus =
  | "NOT_STARTED"
  | "NOTICE_PENDING"
  | "NOTICE_SENT"
  | "WAITING_FOR_TENANT_RESPONSE"
  | "TENANT_INTERESTED"
  | "UNDER_NEGOTIATION"
  | "RENEWAL_CONFIRMED"
  | "RENEWAL_COMPLETED"
  | "TENANT_NOT_RENEWING"
  | "VACATING"
  | "CLOSED";
export type NoticeStatus = "NOT_REQUIRED" | "PENDING" | "DRAFT" | "SENT" | "DELIVERED" | "FAILED";
export type TenantResponse =
  | "NO_RESPONSE"
  | "INTERESTED_IN_RENEWAL"
  | "RENEWAL_CONFIRMED"
  | "UNDER_DISCUSSION"
  | "NOT_INTERESTED"
  | "WILL_VACATE";
export type FollowUpType = "PHONE_CALL" | "EMAIL" | "MEETING" | "WHATS_APP" | "INTERNAL_DISCUSSION";
export type FollowUpStatus = "OPEN" | "DONE" | "CANCELLED";
export type Band = "EXPIRED" | "DAYS_0_TO_30" | "DAYS_31_TO_60" | "DAYS_61_TO_90" | "DAYS_91_TO_120" | "BEYOND_120";
export type DocumentEntity = "building" | "tenant" | "contract" | "notice" | "expense";

// ---------------------------------------------------------------- buildings

export interface Building {
  id: string;
  name: string;
  code: string;
  location: string | null;
  buildingType: string | null;
  notes: string | null;
  totalUnits: number;
  occupiedUnits: number;
  vacantUnits: number;
  reservedUnits: number;
  maintenanceUnits: number;
  createdAt: string;
  updatedAt: string;
}

export interface BuildingInput {
  name: string;
  code: string;
  location: string | null;
  buildingType: string | null;
  notes: string | null;
}

export interface BuildingSummary {
  totalUnits: number;
  occupiedUnits: number;
  vacantUnits: number;
  reservedUnits: number;
  maintenanceUnits: number;
  activeContracts: number;
  expiringSoon: number;
  renewalsPending: number;
}

export interface BuildingDetail {
  building: Building;
  summary: BuildingSummary;
  units: UnitSummary[];
  documents: DocumentInfo[];
}

// ---------------------------------------------------------------- units

export interface UnitSummary {
  id: string;
  buildingId: string;
  buildingName: string;
  buildingCode: string;
  unitNumber: string;
  floor: string | null;
  unitType: string | null;
  status: UnitStatus;
  /** Number of tenants on the unit's active contract (what its bills are split by); 0 when vacant. */
  occupantCount: number;
  notes: string | null;
  contractId: string | null;
  contractNumber: string | null;
  /** Rent on the active contract (the contract may cover several units). */
  rentAmount: number | null;
  tenantId: string | null;
  tenantName: string | null;
  startDate: string | null;
  endDate: string | null;
  remainingDays: number | null;
  band: Band | null;
  expiringSoon: boolean;
  urgent: boolean;
  caseId: string | null;
  renewalStatus: RenewalStatus | null;
  assignedEmployeeId: string | null;
  assignedEmployeeName: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface UnitInput {
  buildingId: string;
  unitNumber: string;
  floor: string | null;
  unitType: string | null;
  status: UnitStatus;
  notes: string | null;
}

export interface UnitListParams extends ListParams {
  buildingId?: string;
  status?: UnitStatus;
  band?: Band;
  renewalStatus?: RenewalStatus;
  tenantId?: string;
  expiringSoon?: boolean;
}

// ---------------------------------------------------------------- tenants

export interface Tenant {
  id: string;
  name: string;
  contactPerson: string | null;
  mobile: string | null;
  email: string | null;
  altContact: string | null;
  address: string | null;
  notes: string | null;
  activeContracts: number;
  currentUnits: number;
  createdAt: string;
  updatedAt: string;
}

export interface TenantInput {
  name: string;
  contactPerson: string | null;
  mobile: string | null;
  email: string | null;
  altContact: string | null;
  address: string | null;
  notes: string | null;
}

export interface TenantDetail {
  tenant: Tenant;
  contracts: Contract[];
  cases: RenewalCase[];
  documents: DocumentInfo[];
}

// ---------------------------------------------------------------- documents

export interface DocumentInfo {
  id: string;
  entityType: DocumentEntity;
  entityId: string;
  fileName: string;
  contentType: string;
  sizeBytes: number;
  uploadedByName: string | null;
  createdAt: string;
}

// ---------------------------------------------------------------- contracts

export interface Contract {
  id: string;
  contractNumber: string;
  tenantId: string;
  tenantName: string;
  tenantContact: string | null;
  tenantEmail: string | null;
  buildingId: string;
  buildingName: string;
  buildingCode: string;
  unitIds: string[];
  unitNumbers: string;
  /** Number of tenants per unit on this contract. */
  unitTenants: ContractUnitTenants[];
  /** Number of tenants on the whole contract. */
  occupantCount: number;
  startDate: string;
  endDate: string;
  durationMonths: number;
  rentTerms: string | null;
  /** Rent for the contract (AED); optional. */
  rentAmount: number | null;
  status: ContractStatus;
  assignedEmployeeId: string | null;
  assignedEmployeeName: string | null;
  previousContractId: string | null;
  rootContractId: string | null;
  renewalSequence: number;
  notes: string | null;
  remainingDays: number;
  band: Band;
  expiringSoon: boolean;
  urgent: boolean;
  renewalInProgress: boolean;
  caseId: string | null;
  renewalStatus: RenewalStatus | null;
  noticeStatus: NoticeStatus | null;
  caseAssignedEmployeeId: string | null;
  caseAssignedEmployeeName: string | null;
  activatedAt: string | null;
  endedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

/** How many people live in one unit under a contract (what that unit's bills are split by). */
export interface ContractUnitTenants {
  unitId: string;
  occupantCount: number;
}

export interface ContractInput {
  contractNumber: string;
  tenantId: string;
  buildingId: string;
  unitIds: string[];
  /** Number of tenants per unit; units not listed get 0. */
  unitTenants: ContractUnitTenants[];
  startDate: string;
  endDate: string;
  rentTerms: string | null;
  rentAmount?: number | null;
  assignedEmployeeId: string | null;
  notes: string | null;
  activate?: boolean;
}

export interface ContractDetail {
  contract: Contract;
  units: UnitSummary[];
  chain: Contract[];
  documents: DocumentInfo[];
  openCase: RenewalCase | null;
}

export interface ContractListParams extends ListParams {
  status?: ContractStatus;
  band?: Band;
  tenantId?: string;
  buildingId?: string;
  expiringSoon?: boolean;
  urgent?: boolean;
  hasOpenCase?: boolean;
  renewalStatus?: RenewalStatus;
}

// ---------------------------------------------------------------- renewals

export interface RenewalCase {
  id: string;
  contractId: string;
  contractNumber: string;
  contractStatus: ContractStatus;
  status: RenewalStatus;
  noticeStatus: NoticeStatus;
  assignedEmployeeId: string | null;
  assignedEmployeeName: string | null;
  isUrgent: boolean;
  latestResponse: TenantResponse | null;
  latestResponseAt: string | null;
  nextFollowUpDate: string | null;
  outcomeContractId: string | null;
  outcomeContractNumber: string | null;
  notes: string | null;
  tenantId: string;
  tenantName: string;
  tenantContact: string | null;
  tenantEmail: string | null;
  tenantMobile: string | null;
  buildingId: string;
  buildingName: string;
  unitNumbers: string;
  startDate: string;
  endDate: string;
  remainingDays: number;
  band: Band;
  checklistTotal: number;
  checklistDone: number;
  progressPercent: number;
  openFollowUps: number;
  noticeSentAt: string | null;
  noticeRecipient: string | null;
  openedAt: string;
  closedAt: string | null;
  updatedAt: string;
}

export interface TenantResponseRecord {
  id: string;
  response: TenantResponse;
  responseDate: string;
  notes: string | null;
  followUpDate: string | null;
  recordedByName: string | null;
  createdAt: string;
}

export interface ChecklistItem {
  id: string;
  key: string | null;
  label: string;
  required: boolean;
  done: boolean;
  doneByName: string | null;
  doneAt: string | null;
}

export interface ChecklistTemplateItem {
  id: string | null;
  key: string | null;
  label: string;
  required: boolean;
  active: boolean;
}

export interface RenewalCaseDetail {
  case: RenewalCase;
  contract: Contract;
  responses: TenantResponseRecord[];
  followUps: FollowUp[];
  checklist: ChecklistItem[];
  allowedTransitions: RenewalStatus[];
}

export interface RecordResponseRequest {
  response: TenantResponse;
  responseDate: string;
  notes: string | null;
  followUpDate: string | null;
  followUpType: FollowUpType | null;
}

export interface CompleteRenewalRequest {
  contractNumber: string | null;
  startDate: string;
  endDate: string;
  rentTerms: string | null;
  /** New rent; null keeps the old contract's amount. */
  rentAmount?: number | null;
  notes: string | null;
}

export interface CompletionResult {
  case: RenewalCase;
  newContract: Contract;
}

export interface CaseListParams extends ListParams {
  status?: RenewalStatus;
  noticeStatus?: NoticeStatus;
  band?: Band;
  openOnly?: boolean;
  assignedEmployeeId?: string;
  buildingId?: string;
  latestResponse?: TenantResponse;
}

// ---------------------------------------------------------------- follow-ups

export interface FollowUp {
  id: string;
  caseId: string;
  contractId: string;
  contractNumber: string;
  tenantId: string;
  tenantName: string;
  buildingName: string;
  unitNumbers: string;
  dueDate: string;
  daysUntilDue: number;
  followUpType: FollowUpType;
  assignedEmployeeId: string | null;
  assignedEmployeeName: string | null;
  notes: string | null;
  status: FollowUpStatus;
  completedAt: string | null;
  createdAt: string;
}

export interface FollowUpInput {
  dueDate: string;
  followUpType: FollowUpType;
  assignedEmployeeId: string | null;
  notes: string | null;
}

export interface FollowUpCounts {
  today: number;
  overdue: number;
  upcoming: number;
}

// ---------------------------------------------------------------- dashboard / search

export interface DashboardCounts {
  totalBuildings: number;
  totalUnits: number;
  occupiedUnits: number;
  vacantUnits: number;
  activeTenants: number;
  activeContracts: number;
  expiringSoon: number;
  renewalsPending: number;
  renewalsCompleted: number;
  expiredContracts: number;
}

export interface BandCount {
  band: Band;
  count: number;
}

export interface Dashboard {
  counts: DashboardCounts;
  bands: BandCount[];
  urgentRenewals: Contract[];
  upcomingExpiries: Contract[];
  pendingTenantResponses: Contract[];
  noticesPending: Contract[];
  recentlyCompleted: RenewalCase[];
  followUps: FollowUpCounts;
  completedWindowDays: number;
}

export type SearchKind = "building" | "unit" | "tenant" | "contract" | "expense";

export interface SearchHit {
  kind: SearchKind;
  id: string;
  title: string;
  subtitle: string;
}

export interface EmployeeOption {
  id: string;
  name: string;
  role: Role;
}

// ---------------------------------------------------------------- email (crates/api/src/email.rs)

export type EmailStatus = "QUEUED" | "SENT" | "DELIVERED" | "FAILED";

export interface EmailTemplate {
  id: string;
  key: string;
  name: string;
  subject: string;
  bodyText: string;
  active: boolean;
  updatedAt: string;
}

export interface EmailTemplateInput {
  name: string;
  subject: string;
  bodyText: string;
  active: boolean;
}

export interface Placeholder {
  name: string;
  description: string;
}

export interface PreviewRequest {
  caseId: string | null;
  subject: string | null;
  bodyText: string | null;
}

export interface Preview {
  subject: string;
  bodyText: string;
  bodyHtml: string;
  recipient: string | null;
}

export interface EmailMessage {
  id: string;
  emailType: string;
  tenantId: string | null;
  tenantName: string | null;
  contractId: string | null;
  contractNumber: string | null;
  caseId: string | null;
  to: string[];
  cc: string[];
  subject: string;
  bodyText: string;
  status: EmailStatus;
  attempts: number;
  lastError: string | null;
  provider: string | null;
  sentByName: string | null;
  attachmentCount: number;
  queuedAt: string;
  sentAt: string | null;
}

export interface ComposeEmailRequest {
  emailType: string;
  tenantId: string | null;
  contractId: string | null;
  caseId: string | null;
  to: string[];
  cc: string[];
  subject: string;
  bodyText: string;
  attachmentDocumentIds: string[];
}

export interface EmailListParams extends ListParams {
  tenantId?: string;
  contractId?: string;
  caseId?: string;
  status?: EmailStatus;
  emailType?: string;
}

export interface Notice {
  id: string;
  caseId: string;
  status: NoticeStatus;
  proposedPeriod: string | null;
  otherTerms: string | null;
  subject: string;
  bodyText: string;
  recipient: string | null;
  cc: string[];
  pdfDocumentId: string | null;
  emailMessageId: string | null;
  emailStatus: EmailStatus | null;
  sentByName: string | null;
  sentAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface NoticeWorkspace {
  draft: Notice | null;
  prepared: Preview;
  proposedPeriod: string;
  otherTerms: string;
  history: Notice[];
}

export interface NoticeDraftInput {
  subject: string;
  bodyText: string;
  proposedPeriod: string | null;
  otherTerms: string | null;
  recipient: string | null;
  cc: string[];
}

export interface SendNoticeRequest {
  to: string[];
  cc: string[];
  emailBodyText: string | null;
}

export interface MailStatus {
  provider: string;
  sender: string;
  fromSettings: boolean;
  queued: number;
  failed: number;
}

// ---------------------------------------------------------------- Settings → Email sending

export type MailProviderKind = "log" | "smtp";
export type SmtpSecurity = "starttls" | "ssl";

export interface MailConfigView {
  configured: boolean;
  provider: MailProviderKind;
  fromName: string;
  fromAddress: string;
  smtpHost: string;
  smtpPort: number;
  smtpSecurity: SmtpSecurity;
  smtpUsername: string;
  hasPassword: boolean;
  imapEnabled: boolean;
  imapHost: string;
  imapPort: number;
  imapSentFolder: string;
}

export interface MailConfigInput {
  provider: MailProviderKind;
  fromName: string;
  fromAddress: string;
  smtpHost: string;
  smtpPort: number;
  smtpSecurity: SmtpSecurity;
  smtpUsername: string;
  /** Omit or leave empty to keep the stored password. */
  smtpPassword?: string | null;
  imapEnabled: boolean;
  imapHost: string;
  imapPort: number;
  imapSentFolder: string;
}

export interface MailTestResult {
  provider: string;
  providerMessageId: string | null;
}

// ---------------------------------------------------------------- automation (crates/api/src/automation.rs)

export type NotificationKind =
  | "CONTRACT_EXPIRING_SOON"
  | "RENEWAL_NOTICE_PENDING"
  | "TENANT_RESPONSE_PENDING"
  | "FOLLOW_UP_DUE_TODAY"
  | "OVERDUE_FOLLOW_UP"
  | "CONTRACT_EXPIRED"
  | "RENEWAL_REMINDER"
  | "RENEWAL_COMPLETED"
  | "CASE_ASSIGNED";

export interface Notification {
  id: string;
  kind: NotificationKind;
  title: string;
  body: string | null;
  entityType: "tenant" | "unit" | "contract" | "renewal_case";
  entityId: string;
  createdAt: string;
  readAt: string | null;
}

export interface ReminderRule {
  id: string | null;
  daysBefore: number;
  label: string;
  notifyInApp: boolean;
  emailAssignedEmployee: boolean;
  markUrgent: boolean;
  active: boolean;
}

export interface Letterhead {
  companyName: string;
  addressLines: string[];
  phone: string;
  email: string;
  footer: string;
}

export interface OrgSettings {
  orgName: string;
  timezone: string;
  thresholds: { expiringSoonDays: number; urgentDays: number };
  autoOpenCase: boolean;
  completedWindowDays: number;
  letterhead: Letterhead;
}

export interface SweepSummary {
  contractsChecked: number;
  remindersFired: number;
  remindersSkipped: number;
  casesOpened: number;
  contractsExpired: number;
  notificationsCreated: number;
  emailsQueued: number;
  followUpAlerts: number;
  noticeAlerts: number;
  responseAlerts: number;
}

// ---------------------------------------------------------------- reports / audit (phase 7)

export type ReportKind = "contract-expiry" | "renewal-status" | "unit-summary" | "tenant-renewal" | "notice-tracking";

export interface ReportSection {
  heading: string;
  rows: string[][];
}

export interface ReportTable {
  kind: ReportKind;
  title: string;
  subtitle: string;
  generatedAt: string;
  columns: string[];
  sections: ReportSection[];
  totalRows: number;
}

export interface ReportParams {
  buildingId?: string;
  employeeId?: string;
  from?: string;
  to?: string;
}

export interface AuditEntry {
  id: number;
  actorId: string | null;
  actorName: string | null;
  entityType: string;
  entityId: string | null;
  action: string;
  before: unknown;
  after: unknown;
  createdAt: string;
}

export interface AuditParams extends ListParams {
  entityType?: string;
  entityId?: string;
  actorId?: string;
  action?: string;
}

// ---------------------------------------------------------------- import (Phase 8)

export interface PlannedContract {
  building: string;
  tenant: string;
  units: string[];
  start: string;
  end: string;
  status: string;
  rentTerms: string | null;
  rentAmount: number | null;
  tenants: number;
  warnings: string[];
  skip: boolean;
}

export interface ImportPreview {
  rowsRead: number;
  rowsSkipped: number;
  columns: Record<string, string>;
  buildingsNew: string[];
  buildingsExisting: string[];
  tenantsNew: string[];
  tenantsExisting: string[];
  unitsNew: number;
  unitsExisting: number;
  contracts: PlannedContract[];
  warnings: string[];
}

export interface ImportResult {
  buildingsCreated: number;
  unitsCreated: number;
  tenantsCreated: number;
  contractsCreated: number;
  contractsSkipped: number;
  warnings: string[];
}

// ---------------------------------------------------------------- expenses

export type ExpenseCategory = "ELECTRICITY" | "WATER" | "GAS" | "INTERNET" | "MAINTENANCE" | "CLEANING" | "MUNICIPALITY" | "OTHER";
export type SplitMethod = "NONE" | "EQUAL";

export interface Expense {
  id: string;
  unitId: string;
  unitNumber: string;
  buildingId: string;
  buildingName: string;
  category: ExpenseCategory;
  description: string;
  amount: number;
  expenseDate: string;
  periodStart: string | null;
  periodEnd: string | null;
  vendor: string | null;
  reference: string | null;
  splitMethod: SplitMethod;
  notes: string | null;
  /** People the bill is split between (0 when not split). */
  splitCount: number;
  /** How many of them have paid. */
  settledCount: number;
  createdByName: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ExpenseInput {
  unitId: string;
  category: ExpenseCategory;
  description: string;
  amount: number;
  expenseDate: string;
  periodStart: string | null;
  periodEnd: string | null;
  vendor: string | null;
  reference: string | null;
  splitMethod: SplitMethod;
  /** People to split between; 0 / omitted = the unit's number of tenants. */
  splitCount?: number | null;
  notes: string | null;
}

/** One person's equal share; the first shares carry the rounding remainder. */
export interface ExpenseShare {
  index: number;
  amount: number;
  settled: boolean;
}

export interface ExpenseDetail {
  expense: Expense;
  shares: ExpenseShare[];
}

export interface ExpenseListParams extends ListParams {
  unitId?: string;
  buildingId?: string;
  category?: ExpenseCategory | "";
  from?: string;
  to?: string;
  outstanding?: boolean;
}

export interface MonthPoint {
  month: string;
  amount: number;
  expenseCount: number;
}

export interface GroupPoint {
  id: string;
  label: string;
  sublabel: string | null;
  amount: number;
  expenseCount: number;
}

export interface CategoryPoint {
  category: ExpenseCategory;
  amount: number;
  expenseCount: number;
}

export interface ExpenseSummary {
  from: string;
  to: string;
  total: number;
  expenseCount: number;
  thisMonth: number;
  lastMonth: number;
  outstanding: number;
  outstandingShares: number;
  monthly: MonthPoint[];
  byBuilding: GroupPoint[];
  byUnit: GroupPoint[];
  byCategory: CategoryPoint[];
}

export interface ExpenseSummaryParams {
  buildingId?: string;
  unitId?: string;
  from?: string;
  to?: string;
}
