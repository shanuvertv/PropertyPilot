import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ClipboardCheck, MessageSquare, Plus } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router";

import type { FollowUp, FollowUpType, RenewalStatus, TenantResponse } from "@/api/types-domain";
import { ExpiryChip, FollowUpStatusBadge, NoticeStatusBadge, RenewalStatusBadge } from "@/components/badges";
import { FormDialog, SelectField, TextAreaField, TextField, errorMessage, opt, selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Progress } from "@/components/ui/progress";
import { useApp } from "@/lib/app-state";
import {
  FOLLOW_UP_TYPE_LABEL,
  RENEWAL_STATUS_LABEL,
  TENANT_RESPONSE_LABEL,
  addDaysIso,
  formatDate,
  formatDateTime,
  keys,
  todayIso,
} from "@/lib/format";
import { useEmployees } from "@/lib/queries";
import { NoticePanel } from "@/pages/renewals/NoticePanel";
import { CaseEmails } from "@/pages/renewals/CaseEmails";
import { HistoryPanel } from "@/components/HistoryPanel";
import { cn } from "@/lib/utils";

const MAIN_LINE: RenewalStatus[] = [
  "NOT_STARTED",
  "NOTICE_PENDING",
  "NOTICE_SENT",
  "WAITING_FOR_TENANT_RESPONSE",
  "TENANT_INTERESTED",
  "UNDER_NEGOTIATION",
  "RENEWAL_CONFIRMED",
  "RENEWAL_COMPLETED",
];

export function RenewalCasePage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const employees = useEmployees();
  const [error, setError] = useState<string | null>(null);
  const [responseDlg, setResponseDlg] = useState(false);
  const [followUpDlg, setFollowUpDlg] = useState<{ open: boolean; edit?: FollowUp | null }>({ open: false });
  const [completeDlg, setCompleteDlg] = useState(false);
  const [statusDlg, setStatusDlg] = useState<RenewalStatus | null>(null);

  const detail = useQuery({ queryKey: ["renewals", id], queryFn: () => api.getCase(id) });
  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["renewals"] }),
      queryClient.invalidateQueries({ queryKey: ["contracts"] }),
      queryClient.invalidateQueries({ queryKey: ["follow-ups"] }),
      queryClient.invalidateQueries({ queryKey: ["units"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard"] }),
    ]);
  };

  const assign = useMutation({
    mutationFn: (emp: string) => api.assignCase(id, emp || null),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e)),
  });
  const tick = useMutation({
    mutationFn: ({ item, done }: { item: string; done: boolean }) => api.setChecklistItem(id, item, done),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e)),
  });
  const followUpStatus = useMutation({
    mutationFn: ({ fid, status }: { fid: string; status: "DONE" | "CANCELLED" | "OPEN" }) => api.setFollowUpStatus(fid, status),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e)),
  });

  if (detail.isError) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Could not load this renewal case.")}</AlertDescription>
      </Alert>
    );
  }
  if (!detail.data) return <p className="text-muted-foreground">Loading…</p>;
  const { case: rc, contract, responses, followUps, checklist, allowedTransitions } = detail.data;
  const open = rc.status !== "RENEWAL_COMPLETED" && rc.status !== "CLOSED";
  const manage = can("MANAGE_RENEWALS") && open;
  const stepIndex = MAIN_LINE.indexOf(rc.status);
  const branch = rc.status === "TENANT_NOT_RENEWING" || rc.status === "VACATING" || rc.status === "CLOSED";

  return (
    <>
      <PageHeader
        title={`Renewal · ${rc.contractNumber}`}
        description={`${rc.tenantName} · ${rc.buildingName} · ${rc.unitNumbers || "no units"} · contract ends ${formatDate(rc.endDate)}`}
        actions={
          <>
            {rc.isUrgent && <Badge variant="destructive">Urgent</Badge>}
            <RenewalStatusBadge status={rc.status} />
            {manage && rc.status === "RENEWAL_CONFIRMED" && (
              <Button onClick={() => setCompleteDlg(true)}>
                <Check data-icon="inline-start" />
                Complete renewal
              </Button>
            )}
            {rc.outcomeContractId && (
              <Button variant="outline" onClick={() => navigate(`/contracts/${rc.outcomeContractId}`)}>
                New contract {rc.outcomeContractNumber}
              </Button>
            )}
          </>
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Workflow stepper (spec §7 step 2) */}
      <ol className="mb-5 flex flex-wrap gap-1.5" aria-label="Renewal workflow">
        {MAIN_LINE.map((s, i) => (
          <li
            key={s}
            className={cn(
              "rounded-full border px-2.5 py-1 text-[12px]",
              i < stepIndex && "border-primary/30 bg-primary/10 text-primary",
              i === stepIndex && "border-primary bg-primary text-primary-foreground",
              (i > stepIndex || branch) && "text-muted-foreground",
            )}
          >
            {RENEWAL_STATUS_LABEL[s]}
          </li>
        ))}
        {branch && (
          <li className="rounded-full border border-destructive bg-destructive/10 px-2.5 py-1 text-[12px] text-destructive">{RENEWAL_STATUS_LABEL[rc.status]}</li>
        )}
      </ol>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="flex flex-col gap-4">
          {can("VIEW_RENEWALS") && <NoticePanel caseId={id} open={open} onChanged={invalidate} />}

          {/* Tenant response */}
          <Card className="py-4">
            <CardHeader className="flex flex-row items-center justify-between px-5">
              <CardTitle className="text-[13px]">Tenant response</CardTitle>
              {manage && (
                <Button size="sm" variant="outline" onClick={() => setResponseDlg(true)}>
                  <MessageSquare data-icon="inline-start" />
                  Record response
                </Button>
              )}
            </CardHeader>
            <CardContent className="px-5">
              {responses.length === 0 ? (
                <p className="text-[13px] text-muted-foreground">No response recorded yet. Recording one moves the case automatically.</p>
              ) : (
                <ul className="divide-y">
                  {responses.map((r) => (
                    <li key={r.id} className="flex items-start gap-3 py-2 text-[13.5px]">
                      <span className="w-32 shrink-0 tabular-nums text-muted-foreground">{formatDate(r.responseDate)}</span>
                      <div className="min-w-0 flex-1">
                        <div className="font-medium">{TENANT_RESPONSE_LABEL[r.response]}</div>
                        {r.notes && <div className="whitespace-pre-wrap text-muted-foreground">{r.notes}</div>}
                        {r.followUpDate && <div className="text-[12px] text-muted-foreground">Follow-up on {formatDate(r.followUpDate)}</div>}
                      </div>
                      <span className="text-[12px] text-muted-foreground">{r.recordedByName}</span>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>

          {/* Follow-ups */}
          <Card className="py-4">
            <CardHeader className="flex flex-row items-center justify-between px-5">
              <CardTitle className="text-[13px]">Follow-ups</CardTitle>
              {can("MANAGE_FOLLOW_UPS") && open && (
                <Button size="sm" variant="outline" onClick={() => setFollowUpDlg({ open: true, edit: null })}>
                  <Plus data-icon="inline-start" />
                  Add follow-up
                </Button>
              )}
            </CardHeader>
            <CardContent className="px-5">
              {followUps.length === 0 ? (
                <p className="text-[13px] text-muted-foreground">No follow-ups yet.</p>
              ) : (
                <ul className="divide-y">
                  {followUps.map((f) => (
                    <li key={f.id} className="flex items-center gap-3 py-2 text-[13.5px]">
                      <span className={cn("w-32 shrink-0 tabular-nums", f.status === "OPEN" && f.daysUntilDue < 0 ? "text-destructive" : "text-muted-foreground")}>
                        {formatDate(f.dueDate)}
                        {f.status === "OPEN" && f.daysUntilDue < 0 && " · overdue"}
                        {f.status === "OPEN" && f.daysUntilDue === 0 && " · today"}
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="font-medium">{FOLLOW_UP_TYPE_LABEL[f.followUpType]}</div>
                        {f.notes && <div className="truncate text-muted-foreground">{f.notes}</div>}
                      </div>
                      <span className="text-[12px] text-muted-foreground">{f.assignedEmployeeName ?? "—"}</span>
                      <FollowUpStatusBadge status={f.status} />
                      {can("MANAGE_FOLLOW_UPS") && f.status === "OPEN" && (
                        <span className="flex gap-1">
                          <Button size="xs" variant="ghost" onClick={() => setFollowUpDlg({ open: true, edit: f })}>
                            Edit
                          </Button>
                          <Button size="xs" variant="outline" onClick={() => followUpStatus.mutate({ fid: f.id, status: "DONE" })}>
                            Done
                          </Button>
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>

          <CaseEmails caseId={id} tenantId={rc.tenantId} contractId={rc.contractId} tenantEmail={rc.tenantEmail} />

          <Card className="py-4">
            <CardHeader className="px-5">
              <CardTitle className="text-[13px]">History</CardTitle>
            </CardHeader>
            <CardContent className="px-5">
              <HistoryPanel entityType="renewal_case" entityId={id} />
            </CardContent>
          </Card>

          {/* Status change */}
          {manage && (
            <Card className="py-4">
              <CardHeader className="px-5">
                <CardTitle className="text-[13px]">Move the case</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-wrap gap-2 px-5">
                {allowedTransitions.filter((s) => s !== "RENEWAL_COMPLETED").map((s) => (
                  <Button
                    key={s}
                    size="sm"
                    variant={s === "CLOSED" || s === "TENANT_NOT_RENEWING" || s === "VACATING" ? "destructive" : "outline"}
                    onClick={() => setStatusDlg(s)}
                  >
                    {RENEWAL_STATUS_LABEL[s]}
                  </Button>
                ))}
                <span className="basis-full text-[12px] text-muted-foreground">Sending the notice (Phase 5) and recording responses move the case automatically; use these for manual steps.</span>
              </CardContent>
            </Card>
          )}
        </div>

        <div className="flex flex-col gap-4">
          {/* Checklist */}
          <Card className="py-4">
            <CardHeader className="px-5">
              <CardTitle className="flex items-center justify-between text-[13px]">
                <span className="flex items-center gap-1.5">
                  <ClipboardCheck className="size-4" /> Checklist
                </span>
                <span className="tabular-nums text-muted-foreground">{rc.progressPercent}%</span>
              </CardTitle>
              <Progress value={rc.progressPercent} className="mt-2" />
            </CardHeader>
            <CardContent className="px-5">
              <ul className="flex flex-col gap-2">
                {checklist.map((i) => (
                  <li key={i.id} className="flex items-start gap-2 text-[13.5px]">
                    <Checkbox
                      checked={i.done}
                      disabled={!manage || tick.isPending}
                      onCheckedChange={(c) => tick.mutate({ item: i.id, done: c === true })}
                      aria-label={i.label}
                    />
                    <div className="min-w-0 flex-1">
                      <div className={cn(i.done && "text-muted-foreground line-through")}>
                        {i.label}
                        {i.required && !i.done && <span className="ml-1 text-[11px] text-destructive">required</span>}
                      </div>
                      {i.done && (
                        <div className="text-[11.5px] text-muted-foreground">
                          {i.doneByName ?? "system"} · {formatDateTime(i.doneAt)}
                        </div>
                      )}
                    </div>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>

          {/* Case facts */}
          <Card className="py-4">
            <CardHeader className="px-5">
              <CardTitle className="text-[13px]">Case</CardTitle>
            </CardHeader>
            <CardContent className="px-5">
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 text-[13px]">
                <dt className="text-muted-foreground">Contract</dt>
                <dd>
                  <Link to={`/contracts/${rc.contractId}`} className="hover:underline">
                    {rc.contractNumber}
                  </Link>
                </dd>
                <dt className="text-muted-foreground">Tenant</dt>
                <dd>
                  <Link to={`/tenants/${rc.tenantId}`} className="hover:underline">
                    {rc.tenantName}
                  </Link>
                  <div className="text-[12px] text-muted-foreground">{[rc.tenantContact, rc.tenantMobile, rc.tenantEmail].filter(Boolean).join(" · ")}</div>
                </dd>
                <dt className="text-muted-foreground">Remaining</dt>
                <dd>
                  <ExpiryChip band={rc.band} days={rc.remainingDays} />
                </dd>
                <dt className="text-muted-foreground">Notice</dt>
                <dd>
                  <NoticeStatusBadge status={rc.noticeStatus} />
                </dd>
                <dt className="text-muted-foreground">Assigned</dt>
                <dd>
                  {manage ? (
                    <select className={`${selectClass} h-7`} value={rc.assignedEmployeeId ?? ""} onChange={(e) => assign.mutate(e.target.value)} aria-label="Assigned employee">
                      <option value="">Unassigned</option>
                      {employees.data?.map((e) => (
                        <option key={e.id} value={e.id}>
                          {e.name}
                        </option>
                      ))}
                    </select>
                  ) : (
                    (rc.assignedEmployeeName ?? "—")
                  )}
                </dd>
                <dt className="text-muted-foreground">Opened</dt>
                <dd>{formatDateTime(rc.openedAt)}</dd>
                {rc.closedAt && (
                  <>
                    <dt className="text-muted-foreground">Closed</dt>
                    <dd>{formatDateTime(rc.closedAt)}</dd>
                  </>
                )}
                <dt className="text-muted-foreground">Terms</dt>
                <dd>{contract.rentTerms ?? "—"}</dd>
              </dl>
            </CardContent>
          </Card>
        </div>
      </div>

      <ResponseDialog open={responseDlg} onOpenChange={setResponseDlg} caseId={id} onSaved={invalidate} />
      <FollowUpDialog open={followUpDlg.open} onOpenChange={(o) => setFollowUpDlg((d) => ({ ...d, open: o }))} caseId={id} edit={followUpDlg.edit} onSaved={invalidate} />
      <CompleteDialog open={completeDlg} onOpenChange={setCompleteDlg} caseId={id} contractEnd={rc.endDate} contractNumber={rc.contractNumber} rentTerms={contract.rentTerms} onDone={(newId) => navigate(`/contracts/${newId}`)} />
      <FormDialog
        open={statusDlg !== null}
        onOpenChange={(o) => !o && setStatusDlg(null)}
        title={statusDlg ? `Move to “${RENEWAL_STATUS_LABEL[statusDlg]}”?` : ""}
        submitLabel="Confirm"
        destructive={statusDlg === "CLOSED" || statusDlg === "TENANT_NOT_RENEWING" || statusDlg === "VACATING"}
        onSubmit={async () => {
          if (!statusDlg) return;
          await api.setCaseStatus(id, statusDlg);
          setStatusDlg(null);
          await invalidate();
        }}
      >
        <p className="text-[13.5px] text-muted-foreground">The change is recorded in the audit trail.</p>
      </FormDialog>
    </>
  );
}

function ResponseDialog({ open, onOpenChange, caseId, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; caseId: string; onSaved: () => Promise<void> }) {
  const { api } = useApp();
  const [form, setForm] = useState({ response: "INTERESTED_IN_RENEWAL" as TenantResponse, responseDate: todayIso(), notes: "", followUpDate: "", followUpType: "PHONE_CALL" as FollowUpType });
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Record tenant response"
      description="The renewal status follows the response: interested → Tenant Interested, confirmed → Renewal Confirmed, not interested → Tenant Not Renewing."
      submitLabel="Record response"
      onSubmit={async () => {
        await api.recordResponse(caseId, {
          response: form.response,
          responseDate: form.responseDate,
          notes: opt(form.notes),
          followUpDate: form.followUpDate || null,
          followUpType: form.followUpDate ? form.followUpType : null,
        });
        setForm((f) => ({ ...f, notes: "", followUpDate: "" }));
        await onSaved();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <SelectField id="r-response" label="Response" value={form.response} onChange={(v) => setForm({ ...form, response: (v || "NO_RESPONSE") as TenantResponse })} options={keys(TENANT_RESPONSE_LABEL).map((r) => ({ value: r, label: TENANT_RESPONSE_LABEL[r] }))} />
        <TextField id="r-date" label="Response date" type="date" value={form.responseDate} onChange={(v) => setForm({ ...form, responseDate: v })} required />
        <TextAreaField id="r-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
        <TextField id="r-fu-date" label="Follow-up date (optional)" type="date" value={form.followUpDate} onChange={(v) => setForm({ ...form, followUpDate: v })} />
        <SelectField id="r-fu-type" label="Follow-up type" value={form.followUpType} onChange={(v) => setForm({ ...form, followUpType: (v || "PHONE_CALL") as FollowUpType })} options={keys(FOLLOW_UP_TYPE_LABEL).map((t) => ({ value: t, label: FOLLOW_UP_TYPE_LABEL[t] }))} disabled={!form.followUpDate} />
      </div>
    </FormDialog>
  );
}

export function FollowUpDialog({ open, onOpenChange, caseId, edit, onSaved }: { open: boolean; onOpenChange: (o: boolean) => void; caseId: string; edit?: FollowUp | null; onSaved: () => Promise<void> }) {
  const { api } = useApp();
  const employees = useEmployees();
  const [form, setForm] = useState({ dueDate: addDaysIso(todayIso(), 3), followUpType: "PHONE_CALL" as FollowUpType, assignedEmployeeId: "", notes: "" });
  const [lastEdit, setLastEdit] = useState<string | null>(null);
  if (open && (edit?.id ?? "new") !== lastEdit) {
    setLastEdit(edit?.id ?? "new");
    setForm({
      dueDate: edit?.dueDate ?? addDaysIso(todayIso(), 3),
      followUpType: edit?.followUpType ?? "PHONE_CALL",
      assignedEmployeeId: edit?.assignedEmployeeId ?? "",
      notes: edit?.notes ?? "",
    });
  }
  if (!open && lastEdit !== null) setLastEdit(null);
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={edit ? "Edit follow-up" : "Add follow-up"}
      submitLabel={edit ? "Save" : "Add follow-up"}
      onSubmit={async () => {
        const input = { dueDate: form.dueDate, followUpType: form.followUpType, assignedEmployeeId: form.assignedEmployeeId || null, notes: opt(form.notes) };
        if (edit) await api.updateFollowUp(edit.id, input);
        else await api.createFollowUp(caseId, input);
        await onSaved();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="f-date" label="Due date" type="date" value={form.dueDate} onChange={(v) => setForm({ ...form, dueDate: v })} required />
        <SelectField id="f-type" label="Type" value={form.followUpType} onChange={(v) => setForm({ ...form, followUpType: (v || "PHONE_CALL") as FollowUpType })} options={keys(FOLLOW_UP_TYPE_LABEL).map((t) => ({ value: t, label: FOLLOW_UP_TYPE_LABEL[t] }))} />
        <SelectField id="f-assigned" label="Assigned employee" value={form.assignedEmployeeId} onChange={(v) => setForm({ ...form, assignedEmployeeId: v })} options={(employees.data ?? []).map((e) => ({ value: e.id, label: e.name }))} placeholder="Case owner" className="sm:col-span-2" />
        <TextAreaField id="f-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}

function CompleteDialog({ open, onOpenChange, caseId, contractEnd, contractNumber, rentTerms, onDone }: { open: boolean; onOpenChange: (o: boolean) => void; caseId: string; contractEnd: string; contractNumber: string; rentTerms: string | null; onDone: (newContractId: string) => void }) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const start = addDaysIso(contractEnd, 1);
  const [form, setForm] = useState({ contractNumber: "", startDate: start, endDate: addDaysIso(start, 364), rentTerms: rentTerms ?? "", notes: "" });
  return (
    <FormDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Complete renewal"
      description={`Closes ${contractNumber} as Renewed and creates the linked new contract, Active from its start date. Tenant, building and units are copied.`}
      submitLabel="Create new contract"
      onSubmit={async () => {
        const res = await api.completeRenewal(caseId, {
          contractNumber: opt(form.contractNumber),
          startDate: form.startDate,
          endDate: form.endDate,
          rentTerms: opt(form.rentTerms),
          notes: opt(form.notes),
        });
        await queryClient.invalidateQueries();
        onDone(res.newContract.id);
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <TextField id="cr-number" label="New contract number" value={form.contractNumber} onChange={(v) => setForm({ ...form, contractNumber: v })} placeholder={`${contractNumber}-R…`} hint="Leave blank to number it automatically." className="sm:col-span-2" />
        <TextField id="cr-start" label="New start date" type="date" value={form.startDate} onChange={(v) => setForm({ ...form, startDate: v })} required />
        <TextField id="cr-end" label="New end date" type="date" value={form.endDate} onChange={(v) => setForm({ ...form, endDate: v })} required />
        <TextField id="cr-rent" label="Rental terms" value={form.rentTerms} onChange={(v) => setForm({ ...form, rentTerms: v })} className="sm:col-span-2" />
        <TextAreaField id="cr-notes" label="Notes" value={form.notes} onChange={(v) => setForm({ ...form, notes: v })} className="sm:col-span-2" />
      </div>
    </FormDialog>
  );
}
