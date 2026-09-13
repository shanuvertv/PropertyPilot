import type {
  BootstrapRequest,
  CreateUserRequest,
  ErrorBody,
  ErrorCode,
  HealthResponse,
  LoginRequest,
  LoginResponse,
  SessionInfo,
  SystemStatus,
  UserSummary,
} from "./types";
import type {
  Building,
  BuildingDetail,
  BuildingInput,
  CaseListParams,
  ChecklistItem,
  ChecklistTemplateItem,
  CompleteRenewalRequest,
  CompletionResult,
  Contract,
  ContractDetail,
  ContractInput,
  ContractListParams,
  Dashboard,
  DocumentEntity,
  DocumentInfo,
  EmployeeOption,
  FollowUp,
  FollowUpCounts,
  FollowUpInput,
  FollowUpStatus,
  ListParams,
  Page,
  RecordResponseRequest,
  RenewalCase,
  RenewalCaseDetail,
  RenewalStatus,
  SearchHit,
  Tenant,
  TenantDetail,
  TenantInput,
  UnitInput,
  UnitListParams,
  UnitStatus,
  UnitSummary,
  ComposeEmailRequest,
  EmailListParams,
  EmailMessage,
  EmailTemplate,
  EmailTemplateInput,
  MailStatus,
  Notice,
  NoticeDraftInput,
  NoticeWorkspace,
  Placeholder,
  Preview,
  PreviewRequest,
  SendNoticeRequest,
  Notification,
  OrgSettings,
  ReminderRule,
  SweepSummary,
  AuditEntry,
  AuditParams,
  ReportKind,
  ReportParams,
  ReportTable,
  ImportPreview,
  ImportResult,
  MailConfigInput,
  MailConfigView,
  MailTestResult,
  Occupant,
  OccupantInput,
  Expense,
  ExpenseInput,
  ExpenseDetail,
  ExpenseListParams,
  ExpenseSummary,
  ExpenseSummaryParams,
} from "./types-domain";

/** Thrown for any non-2xx response, or when the server cannot be reached at all. */
export class ApiRequestError extends Error {
  constructor(
    public readonly code: ErrorCode | "NETWORK",
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }
}

export interface ApiClientOptions {
  baseUrl: string;
  getToken: () => string | null;
  onUnauthorized?: () => void;
}

export function normalizeBaseUrl(input: string): string {
  let url = input.trim();
  if (!url) return "";
  if (!/^https?:\/\//i.test(url)) url = `http://${url}`;
  return url.replace(/\/+$/, "");
}

/** Builds `?a=1&b=2` from an object, skipping undefined/null/empty values. */
export function qs(params: object | undefined): string {
  if (!params) return "";
  const parts: string[] = [];
  for (const [k, v] of Object.entries(params as Record<string, unknown>)) {
    if (v === undefined || v === null || v === "") continue;
    parts.push(`${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
  }
  return parts.length ? `?${parts.join("&")}` : "";
}

export class ApiClient {
  constructor(private readonly opts: ApiClientOptions) {}

  get baseUrl() {
    return this.opts.baseUrl;
  }

  private headers(json: boolean): Record<string, string> {
    const headers: Record<string, string> = { Accept: "application/json" };
    if (json) headers["Content-Type"] = "application/json";
    const token = this.opts.getToken();
    if (token) headers.Authorization = `Bearer ${token}`;
    return headers;
  }

  private async handle<T>(resPromise: Promise<Response>, asBlob = false): Promise<T> {
    let res: Response;
    try {
      res = await resPromise;
    } catch {
      throw new ApiRequestError("NETWORK", `Cannot reach the server at ${this.opts.baseUrl}`, 0);
    }
    if (res.status === 204) return undefined as T;
    if (!res.ok) {
      let code: ErrorCode = "INTERNAL";
      let message = `${res.status} ${res.statusText}`;
      try {
        const parsed = (await res.json()) as ErrorBody;
        if (parsed?.error) {
          code = parsed.error.code;
          message = parsed.error.message;
        }
      } catch {
        // non-JSON error body; keep the status text
      }
      if (res.status === 401 && code === "UNAUTHORIZED") this.opts.onUnauthorized?.();
      throw new ApiRequestError(code, message, res.status);
    }
    if (asBlob) return (await res.blob()) as T;
    return (await res.json()) as T;
  }

  private request<T>(method: string, path: string, body?: unknown): Promise<T> {
    return this.handle<T>(
      fetch(`${this.opts.baseUrl}${path}`, {
        method,
        headers: this.headers(body !== undefined),
        body: body === undefined ? undefined : JSON.stringify(body),
      }),
    );
  }

  // ---- system
  health() {
    return this.request<HealthResponse>("GET", "/api/health");
  }
  systemStatus() {
    return this.request<SystemStatus>("GET", "/api/system/status");
  }
  employees() {
    return this.request<EmployeeOption[]>("GET", "/api/employees");
  }

  // ---- auth
  login(req: LoginRequest) {
    return this.request<LoginResponse>("POST", "/api/auth/login", req);
  }
  bootstrap(req: BootstrapRequest) {
    return this.request<LoginResponse>("POST", "/api/auth/bootstrap", req);
  }
  logout() {
    return this.request<void>("POST", "/api/auth/logout");
  }
  me() {
    return this.request<SessionInfo>("GET", "/api/auth/me");
  }

  // ---- users (Admin)
  listUsers() {
    return this.request<UserSummary[]>("GET", "/api/users");
  }
  createUser(req: CreateUserRequest) {
    return this.request<UserSummary>("POST", "/api/users", req);
  }
  setUserActive(id: string, active: boolean) {
    return this.request<UserSummary>("PUT", `/api/users/${encodeURIComponent(id)}/active`, { active });
  }

  // ---- buildings
  listBuildings(p: ListParams) {
    return this.request<Page<Building>>("GET", `/api/buildings${qs(p)}`);
  }
  buildingOptions() {
    return this.request<Building[]>("GET", "/api/buildings/options");
  }
  getBuilding(id: string) {
    return this.request<BuildingDetail>("GET", `/api/buildings/${id}`);
  }
  createBuilding(input: BuildingInput) {
    return this.request<Building>("POST", "/api/buildings", input);
  }
  updateBuilding(id: string, input: BuildingInput) {
    return this.request<Building>("PUT", `/api/buildings/${id}`, input);
  }
  archiveBuilding(id: string) {
    return this.request<void>("DELETE", `/api/buildings/${id}`);
  }

  // ---- units
  listUnits(p: UnitListParams) {
    return this.request<Page<UnitSummary>>("GET", `/api/units${qs(p)}`);
  }
  getUnit(id: string) {
    return this.request<UnitSummary>("GET", `/api/units/${id}`);
  }
  createUnit(input: UnitInput) {
    return this.request<UnitSummary>("POST", "/api/units", input);
  }
  updateUnit(id: string, input: UnitInput) {
    return this.request<UnitSummary>("PUT", `/api/units/${id}`, input);
  }
  setUnitStatus(id: string, status: UnitStatus) {
    return this.request<UnitSummary>("PUT", `/api/units/${id}/status`, { status });
  }
  archiveUnit(id: string) {
    return this.request<void>("DELETE", `/api/units/${id}`);
  }

  // ---- tenants
  listTenants(p: ListParams) {
    return this.request<Page<Tenant>>("GET", `/api/tenants${qs(p)}`);
  }
  tenantOptions() {
    return this.request<Tenant[]>("GET", "/api/tenants/options");
  }
  getTenant(id: string) {
    return this.request<TenantDetail>("GET", `/api/tenants/${id}`);
  }
  createTenant(input: TenantInput) {
    return this.request<Tenant>("POST", "/api/tenants", input);
  }
  updateTenant(id: string, input: TenantInput) {
    return this.request<Tenant>("PUT", `/api/tenants/${id}`, input);
  }
  archiveTenant(id: string) {
    return this.request<void>("DELETE", `/api/tenants/${id}`);
  }

  // ---- documents
  listDocuments(entityType: DocumentEntity, entityId: string) {
    return this.request<DocumentInfo[]>("GET", `/api/documents${qs({ entityType, entityId })}`);
  }
  uploadDocument(entityType: DocumentEntity, entityId: string, file: File) {
    const form = new FormData();
    form.append("entityType", entityType);
    form.append("entityId", entityId);
    form.append("file", file, file.name);
    return this.handle<DocumentInfo>(
      fetch(`${this.opts.baseUrl}/api/documents`, { method: "POST", headers: this.headers(false), body: form }),
    );
  }
  downloadDocument(id: string) {
    return this.handle<Blob>(
      fetch(`${this.opts.baseUrl}/api/documents/${id}/download`, { headers: this.headers(false) }),
      true,
    );
  }
  deleteDocument(id: string) {
    return this.request<void>("DELETE", `/api/documents/${id}`);
  }

  // ---- contracts
  listContracts(p: ContractListParams) {
    return this.request<Page<Contract>>("GET", `/api/contracts${qs(p)}`);
  }
  suggestContractNumber() {
    return this.request<{ contractNumber: string }>("GET", "/api/contracts/suggest-number");
  }
  getContract(id: string) {
    return this.request<ContractDetail>("GET", `/api/contracts/${id}`);
  }
  createContract(input: ContractInput) {
    return this.request<Contract>("POST", "/api/contracts", input);
  }
  updateContract(id: string, input: ContractInput) {
    return this.request<Contract>("PUT", `/api/contracts/${id}`, input);
  }
  activateContract(id: string) {
    return this.request<Contract>("POST", `/api/contracts/${id}/activate`);
  }
  terminateContract(id: string, reason: string | null) {
    return this.request<Contract>("POST", `/api/contracts/${id}/terminate`, { reason });
  }
  assignContract(id: string, assignedEmployeeId: string | null) {
    return this.request<Contract>("PUT", `/api/contracts/${id}/assign`, { assignedEmployeeId });
  }
  startRenewal(contractId: string, assignedEmployeeId: string | null) {
    return this.request<RenewalCase>("POST", `/api/contracts/${contractId}/renewal`, { assignedEmployeeId });
  }
  expireOverdue() {
    return this.request<string[]>("POST", "/api/contracts/expire-overdue");
  }

  // ---- dashboard / search
  dashboard() {
    return this.request<Dashboard>("GET", "/api/dashboard");
  }
  search(q: string) {
    return this.request<SearchHit[]>("GET", `/api/search${qs({ q })}`);
  }

  // ---- renewals
  listCases(p: CaseListParams) {
    return this.request<Page<RenewalCase>>("GET", `/api/renewals${qs(p)}`);
  }
  getCase(id: string) {
    return this.request<RenewalCaseDetail>("GET", `/api/renewals/${id}`);
  }
  setCaseStatus(id: string, status: RenewalStatus) {
    return this.request<RenewalCase>("PUT", `/api/renewals/${id}/status`, { status });
  }
  assignCase(id: string, assignedEmployeeId: string | null) {
    return this.request<RenewalCase>("PUT", `/api/renewals/${id}/assign`, { assignedEmployeeId });
  }
  setCaseNotes(id: string, notes: string | null) {
    return this.request<RenewalCase>("PUT", `/api/renewals/${id}/notes`, { notes });
  }
  recordResponse(id: string, req: RecordResponseRequest) {
    return this.request<RenewalCase>("POST", `/api/renewals/${id}/responses`, req);
  }
  setChecklistItem(caseId: string, itemId: string, done: boolean) {
    return this.request<ChecklistItem[]>("PUT", `/api/renewals/${caseId}/checklist/${itemId}`, { done });
  }
  completeRenewal(id: string, req: CompleteRenewalRequest) {
    return this.request<CompletionResult>("POST", `/api/renewals/${id}/complete`, req);
  }
  checklistTemplate() {
    return this.request<ChecklistTemplateItem[]>("GET", "/api/renewals/checklist-template");
  }
  saveChecklistTemplate(items: ChecklistTemplateItem[]) {
    return this.request<ChecklistTemplateItem[]>("PUT", "/api/renewals/checklist-template", items);
  }

  // ---- follow-ups
  listFollowUps(p: ListParams & { scope?: "today" | "overdue" | "upcoming" | "open" | "all"; mine?: boolean }) {
    return this.request<Page<FollowUp>>("GET", `/api/follow-ups${qs(p)}`);
  }
  followUpCounts(mine = false) {
    return this.request<FollowUpCounts>("GET", `/api/follow-ups/counts${qs({ mine })}`);
  }
  createFollowUp(caseId: string, input: FollowUpInput) {
    return this.request<FollowUp>("POST", `/api/renewals/${caseId}/follow-ups`, input);
  }
  updateFollowUp(id: string, input: FollowUpInput) {
    return this.request<FollowUp>("PUT", `/api/follow-ups/${id}`, input);
  }
  setFollowUpStatus(id: string, status: FollowUpStatus) {
    return this.request<FollowUp>("PUT", `/api/follow-ups/${id}/status`, { status });
  }

  // ---- email templates
  emailTemplates() {
    return this.request<EmailTemplate[]>("GET", "/api/email-templates");
  }
  placeholders() {
    return this.request<Placeholder[]>("GET", "/api/email-templates/placeholders");
  }
  updateEmailTemplate(key: string, input: EmailTemplateInput) {
    return this.request<EmailTemplate>("PUT", `/api/email-templates/${encodeURIComponent(key)}`, input);
  }
  previewTemplate(key: string, req: PreviewRequest) {
    return this.request<Preview>("POST", `/api/email-templates/${encodeURIComponent(key)}/preview`, req);
  }

  // ---- email log
  listEmails(p: EmailListParams) {
    return this.request<Page<EmailMessage>>("GET", `/api/emails${qs(p)}`);
  }
  getEmail(id: string) {
    return this.request<EmailMessage>("GET", `/api/emails/${id}`);
  }
  composeEmail(req: ComposeEmailRequest) {
    return this.request<EmailMessage>("POST", "/api/emails", req);
  }
  retryEmail(id: string) {
    return this.request<EmailMessage>("POST", `/api/emails/${id}/retry`);
  }
  mailStatus() {
    return this.request<MailStatus>("GET", "/api/system/mail");
  }

  // ---- notices
  noticeWorkspace(caseId: string) {
    return this.request<NoticeWorkspace>("GET", `/api/renewals/${caseId}/notice`);
  }
  saveNoticeDraft(caseId: string, input: NoticeDraftInput) {
    return this.request<Notice>("PUT", `/api/renewals/${caseId}/notice`, input);
  }
  generateNoticePdf(caseId: string) {
    return this.request<Notice>("POST", `/api/renewals/${caseId}/notice/pdf`);
  }
  sendNotice(caseId: string, req: SendNoticeRequest) {
    return this.request<Notice>("POST", `/api/renewals/${caseId}/notice/send`, req);
  }

  // ---- notifications / live events
  notifications(unreadOnly = false) {
    return this.request<Notification[]>("GET", `/api/notifications${qs({ unread: unreadOnly || undefined })}`);
  }
  unreadCount() {
    return this.request<{ unread: number }>("GET", "/api/notifications/count");
  }
  markRead(id: string) {
    return this.request<void>("PUT", `/api/notifications/${id}/read`);
  }
  markAllRead() {
    return this.request<void>("POST", "/api/notifications/read-all");
  }
  /** EventSource cannot send headers, so the token rides in the query string (HTTPS in production). */
  eventsUrl(): string | null {
    const token = this.opts.getToken();
    return token ? `${this.opts.baseUrl}/api/events?token=${encodeURIComponent(token)}` : null;
  }

  // ---- settings
  orgSettings() {
    return this.request<OrgSettings>("GET", "/api/settings/org");
  }
  saveOrgSettings(input: OrgSettings) {
    return this.request<OrgSettings>("PUT", "/api/settings/org", input);
  }
  reminderRules() {
    return this.request<ReminderRule[]>("GET", "/api/settings/reminder-rules");
  }
  saveReminderRules(rules: ReminderRule[]) {
    return this.request<ReminderRule[]>("PUT", "/api/settings/reminder-rules", rules);
  }
  runSweep() {
    return this.request<SweepSummary>("POST", "/api/system/sweep");
  }

  // ---- reports / audit
  report(kind: ReportKind, p: ReportParams) {
    return this.request<ReportTable>("GET", `/api/reports/${kind}${qs(p)}`);
  }
  reportFile(kind: ReportKind, p: ReportParams, format: "xlsx" | "pdf") {
    return this.handle<Blob>(fetch(`${this.opts.baseUrl}/api/reports/${kind}${qs({ ...p, format })}`, { headers: this.headers(false) }), true);
  }
  auditList(p: AuditParams) {
    return this.request<Page<AuditEntry>>("GET", `/api/audit${qs(p)}`);
  }
  auditFor(entityType: string, id: string) {
    return this.request<AuditEntry[]>("GET", `/api/audit/${entityType}/${id}`);
  }

  // ---- Settings → Email sending
  mailSettings() {
    return this.request<MailConfigView>("GET", "/api/settings/mail");
  }
  saveMailSettings(input: MailConfigInput) {
    return this.request<MailConfigView>("PUT", "/api/settings/mail", input);
  }
  sendTestMail(to: string) {
    return this.request<MailTestResult>("POST", "/api/settings/mail/test", { to });
  }

  deleteUser(id: string) {
    return this.request<void>("DELETE", `/api/users/${id}`);
  }

  // ---- occupants & expenses
  occupants(unitId: string, includePast = false) {
    return this.request<Occupant[]>("GET", `/api/units/${unitId}/occupants${qs({ includePast })}`);
  }
  createOccupant(unitId: string, input: OccupantInput) {
    return this.request<Occupant>("POST", `/api/units/${unitId}/occupants`, input);
  }
  updateOccupant(id: string, input: OccupantInput) {
    return this.request<Occupant>("PUT", `/api/occupants/${id}`, input);
  }
  moveOutOccupant(id: string, moveOut: string | null) {
    return this.request<Occupant>("POST", `/api/occupants/${id}/move-out`, { moveOut });
  }
  deleteOccupant(id: string) {
    return this.request<void>("DELETE", `/api/occupants/${id}`);
  }
  expenses(p: ExpenseListParams) {
    return this.request<Page<Expense>>("GET", `/api/expenses${qs(p)}`);
  }
  expense(id: string) {
    return this.request<ExpenseDetail>("GET", `/api/expenses/${id}`);
  }
  createExpense(input: ExpenseInput) {
    return this.request<Expense>("POST", "/api/expenses", input);
  }
  updateExpense(id: string, input: ExpenseInput) {
    return this.request<Expense>("PUT", `/api/expenses/${id}`, input);
  }
  deleteExpense(id: string) {
    return this.request<void>("DELETE", `/api/expenses/${id}`);
  }
  splitExpenseEqually(id: string) {
    return this.request<ExpenseDetail>("POST", `/api/expenses/${id}/split`);
  }
  setExpenseShares(id: string, shares: { occupantId: string; amount: number }[]) {
    return this.request<ExpenseDetail>("PUT", `/api/expenses/${id}/shares`, { shares });
  }
  settleShare(id: string, occupantId: string, settled: boolean) {
    return this.request<ExpenseDetail>("PUT", `/api/expenses/${id}/shares/${occupantId}/settled`, { settled });
  }
  expenseSummary(p: ExpenseSummaryParams) {
    return this.request<ExpenseSummary>("GET", `/api/expenses/summary${qs(p)}`);
  }

  // ---- passwords
  changePassword(currentPassword: string, newPassword: string) {
    return this.request<void>("PUT", "/api/auth/password", { currentPassword, newPassword });
  }
  resetPassword(userId: string, newPassword: string) {
    return this.request<void>("PUT", `/api/users/${userId}/password`, { newPassword });
  }

  // ---- Excel import
  private importUpload<T>(path: string, file: File) {
    const form = new FormData();
    form.append("file", file, file.name);
    return this.handle<T>(fetch(`${this.opts.baseUrl}${path}`, { method: "POST", headers: this.headers(false), body: form }));
  }
  importPreview(file: File) {
    return this.importUpload<ImportPreview>("/api/import/preview", file);
  }
  importCommit(file: File) {
    return this.importUpload<ImportResult>("/api/import/commit", file);
  }
}
