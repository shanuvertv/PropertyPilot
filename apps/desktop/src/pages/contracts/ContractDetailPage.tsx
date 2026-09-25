import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarClock, KeyRound, Pencil, Play, XCircle } from "lucide-react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router";

import { ContractStatusBadge, ExpiryChip, NoticeStatusBadge, RenewalStatusBadge, UnitStatusBadge } from "@/components/badges";
import { DocumentsPanel } from "@/components/DocumentsPanel";
import { HistoryPanel } from "@/components/HistoryPanel";
import { FormDialog, SelectField, TextAreaField, TextField, errorMessage } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { formatDate, formatDateTime, formatMoney, todayIso } from "@/lib/format";
import { useEmployees } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { ContractCheques } from "@/pages/cheques/ContractCheques";
import { ContractDialog } from "./ContractDialog";

export function ContractDetailPage() {
  const { id = "" } = useParams();
  const [search, setSearch] = useSearchParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const employees = useEmployees();
  const [edit, setEdit] = useState(false);
  const [terminate, setTerminate] = useState(false);
  const [reason, setReason] = useState("");
  const [endedOn, setEndedOn] = useState(todayIso());
  const [relet, setRelet] = useState(false);
  const [startRenewal, setStartRenewal] = useState(false);
  const [assignee, setAssignee] = useState("");
  const [error, setError] = useState<string | null>(null);

  const detail = useQuery({ queryKey: ["contracts", id], queryFn: () => api.getContract(id) });
  const c = detail.data?.contract;

  // Deep link from the unit list: /contracts/:id?start=renewal
  useEffect(() => {
    if (search.get("start") === "renewal" && c && !c.caseId && can("MANAGE_RENEWALS")) {
      setStartRenewal(true);
      setSearch({}, { replace: true });
    }
  }, [search, c, can, setSearch]);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["contracts"] }),
      queryClient.invalidateQueries({ queryKey: ["units"] }),
      queryClient.invalidateQueries({ queryKey: ["renewals"] }),
      queryClient.invalidateQueries({ queryKey: ["dashboard"] }),
    ]);
  };

  // After a contract expires the units stay Occupied until somebody records the handover,
  // so they cannot be let to another tenant by mistake.
  const unitsHeld = (detail.data?.units ?? []).some((u) => u.status === "OCCUPIED") && c?.status !== "ACTIVE" && c?.status !== "DRAFT";
  const release = useMutation({
    mutationFn: () => api.releaseContractUnits(id),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e, "Could not release the units.")),
  });

  const activate = useMutation({
    mutationFn: () => api.activateContract(id),
    onSuccess: () => void invalidate(),
    onError: (e) => setError(errorMessage(e)),
  });

  if (detail.isError) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Could not load this contract.")}</AlertDescription>
      </Alert>
    );
  }
  if (!c || !detail.data) return <p className="text-muted-foreground">Loading…</p>;
  const d = detail.data;

  return (
    <>
      <PageHeader
        title={`Contract ${c.contractNumber}`}
        description={`${c.tenantName} · ${c.buildingName} · ${c.unitNumbers || "no units"}`}
        actions={
          <>
            <ContractStatusBadge status={c.status} />
            {c.status === "DRAFT" && can("MANAGE_CONTRACTS") && (
              <Button onClick={() => activate.mutate()} disabled={activate.isPending}>
                <Play data-icon="inline-start" />
                Activate
              </Button>
            )}
            {(c.status === "ACTIVE" || (c.status === "EXPIRED" && unitsHeld)) && !c.caseId && can("MANAGE_RENEWALS") && (
              <Button onClick={() => setStartRenewal(true)}>
                <CalendarClock data-icon="inline-start" />
                Start renewal
              </Button>
            )}
            {c.caseId && can("VIEW_RENEWALS") && (
              <Button variant="outline" onClick={() => navigate(`/renewals/${c.caseId}`)}>
                Open renewal case
              </Button>
            )}
            {c.status !== "ACTIVE" && c.status !== "DRAFT" && can("MANAGE_CONTRACTS") && (
              <>
                {unitsHeld && !c.caseId && (
                  <Button variant="outline" onClick={() => release.mutate()} disabled={release.isPending}>
                    <KeyRound data-icon="inline-start" />
                    Release units
                  </Button>
                )}
                <Button variant="outline" onClick={() => setRelet(true)}>
                  <Play data-icon="inline-start" />
                  Re-let these units
                </Button>
              </>
            )}
            {(c.status === "DRAFT" || c.status === "ACTIVE") && can("MANAGE_CONTRACTS") && (
              <>
                <Button variant="outline" onClick={() => setEdit(true)}>
                  <Pencil data-icon="inline-start" />
                  Edit
                </Button>
                {c.status === "ACTIVE" && (
                  <Button variant="outline" className="text-destructive" onClick={() => setTerminate(true)}>
                    <XCircle data-icon="inline-start" />
                    Cancel contract
                  </Button>
                )}
              </>
            )}
          </>
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="mb-6 grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
        <Card className="py-4">
          <CardContent className="grid grid-cols-2 gap-x-6 gap-y-3 px-5 text-[13.5px] md:grid-cols-4">
            <div>
              <div className="text-[12px] text-muted-foreground">Start date</div>
              <div>{formatDate(c.startDate)}</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">End date</div>
              <div>{formatDate(c.endDate)}</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Duration</div>
              <div className="tabular-nums">{c.durationMonths} months</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Remaining</div>
              <div>{c.status === "ACTIVE" ? <ExpiryChip band={c.band} days={c.remainingDays} /> : "—"}</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Tenant</div>
              <Link to={`/tenants/${c.tenantId}`} className="hover:underline">
                {c.tenantName}
              </Link>
              {c.tenantContact && <div className="text-[12px] text-muted-foreground">{c.tenantContact}</div>}
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Building</div>
              <Link to={`/buildings/${c.buildingId}`} className="hover:underline">
                {c.buildingName}
              </Link>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Assigned employee</div>
              <div>{c.assignedEmployeeName ?? "—"}</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Rent amount</div>
              <div className="tabular-nums">
                {c.rentAmount === null ? "—" : formatMoney(c.rentAmount)}
                {c.unitIds.length > 1 && c.rentAmount !== null && <span className="text-[12px] text-muted-foreground"> · total of {c.unitIds.length} units</span>}
              </div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Payment terms</div>
              <div>{c.rentTerms ?? "—"}</div>
            </div>
            <div>
              <div className="text-[12px] text-muted-foreground">Number of occupants</div>
              <div className="tabular-nums">
                {c.occupantCount}
                {c.unitIds.length > 1 && <span className="text-[12px] text-muted-foreground"> · {c.unitTerms.map((t) => t.occupantCount).join(" + ")} by unit</span>}
              </div>
            </div>
            {c.notes && (
              <div className="col-span-full">
                <div className="text-[12px] text-muted-foreground">Notes</div>
                <div className="whitespace-pre-wrap">{c.notes}</div>
              </div>
            )}
          </CardContent>
        </Card>

        <Card className="py-4">
          <CardHeader className="px-5">
            <CardTitle className="text-[13px]">Renewal</CardTitle>
          </CardHeader>
          <CardContent className="px-5 text-[13.5px]">
            {c.caseId ? (
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5">
                <dt className="text-muted-foreground">Status</dt>
                <dd>
                  <RenewalStatusBadge status={c.renewalStatus} />
                </dd>
                <dt className="text-muted-foreground">Notice</dt>
                <dd>
                  <NoticeStatusBadge status={c.noticeStatus} />
                </dd>
                <dt className="text-muted-foreground">Assigned</dt>
                <dd>{c.caseAssignedEmployeeName ?? "—"}</dd>
                {d.openCase && (
                  <>
                    <dt className="text-muted-foreground">Progress</dt>
                    <dd className="tabular-nums">{d.openCase.progressPercent}%</dd>
                  </>
                )}
              </dl>
            ) : c.status === "ACTIVE" ? (
              <p className="text-muted-foreground">{c.expiringSoon ? "Expiring soon — no renewal case yet." : "No renewal case yet."}</p>
            ) : c.status === "RENEWED" ? (
              <p className="text-muted-foreground">Renewed — see the timeline below.</p>
            ) : (
              <p className="text-muted-foreground">Not applicable.</p>
            )}
          </CardContent>
        </Card>
      </div>

      <Tabs value={search.get("tab") ?? "timeline"} onValueChange={(v) => setSearch((prev) => { const n = new URLSearchParams(prev); if (v === "timeline") n.delete("tab"); else n.set("tab", String(v)); return n; })}>
        <TabsList>
          <TabsTrigger value="timeline">Timeline ({d.chain.length})</TabsTrigger>
          <TabsTrigger value="cheques">Cheques</TabsTrigger>
          <TabsTrigger value="units">Units ({d.units.length})</TabsTrigger>
          <TabsTrigger value="documents">Documents ({d.documents.length})</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
        <TabsContent value="timeline" className="pt-4">
          <ol className="flex flex-col gap-2">
            {d.chain.map((x) => (
              <li key={x.id} className={cn("flex items-center gap-4 rounded-md border bg-card px-4 py-3", x.id === c.id && "border-primary")}>
                <Badge variant="outline" className="w-24 justify-center font-mono">
                  {x.renewalSequence === 0 ? "Original" : `Renewal ${x.renewalSequence}`}
                </Badge>
                <div className="min-w-0 flex-1">
                  {x.id === c.id ? <span className="font-medium">{x.contractNumber}</span> : <Link to={`/contracts/${x.id}`} className="font-medium hover:underline">{x.contractNumber}</Link>}
                  <div className="text-[12.5px] text-muted-foreground">
                    {formatDate(x.startDate)} → {formatDate(x.endDate)} · {x.durationMonths} months{x.rentAmount !== null ? ` · ${formatMoney(x.rentAmount)}` : ""}{x.rentTerms ? ` · ${x.rentTerms}` : ""}
                  </div>
                </div>
                <ContractStatusBadge status={x.status} />
                <span className="w-28 text-right text-[12px] text-muted-foreground">{x.endedAt ? `ended ${formatDateTime(x.endedAt)}` : x.activatedAt ? `active since ${formatDate(x.activatedAt)}` : ""}</span>
              </li>
            ))}
          </ol>
        </TabsContent>
        <TabsContent value="cheques" className="pt-4">
          <ContractCheques contract={c} />
        </TabsContent>
        <TabsContent value="units" className="pt-4">
          <ul className="divide-y rounded-md border bg-card">
            {d.units.map((u) => (
              <li key={u.id} className="flex items-center gap-3 px-4 py-2.5 text-[13.5px]">
                <span className="w-24 font-medium">{u.unitNumber}</span>
                <span className="flex-1 text-muted-foreground">{[u.floor && `Floor ${u.floor}`, u.unitType].filter(Boolean).join(" · ")}</span>
                <span className="text-[12.5px] text-muted-foreground tabular-nums">{c.unitTerms.find((t) => t.unitId === u.id)?.occupantCount ?? 0} occupants</span>
                <span className="w-32 text-right text-[12.5px] tabular-nums">{(() => { const r = c.unitTerms.find((t) => t.unitId === u.id)?.rentAmount ?? null; return r === null ? "—" : formatMoney(r); })()}</span>
                <UnitStatusBadge status={u.status} />
              </li>
            ))}
          </ul>
        </TabsContent>
        <TabsContent value="documents" className="pt-4">
          <DocumentsPanel entityType="contract" entityId={c.id} canManage={can("MANAGE_CONTRACTS")} />
        </TabsContent>
        <TabsContent value="history" className="pt-4">
          <HistoryPanel entityType="contract" entityId={c.id} />
        </TabsContent>
      </Tabs>

      <ContractDialog open={edit} onOpenChange={setEdit} contract={c} onSaved={() => void invalidate()} />
      <ContractDialog
        open={relet}
        onOpenChange={setRelet}
        defaults={{
          tenantId: c.tenantId,
          buildingId: c.buildingId,
          unitIds: c.unitIds,
          unitTerms: c.unitTerms,
          rentTerms: c.rentTerms,
          replacesContractId: c.id,
        }}
        onSaved={(saved) => {
          void invalidate();
          navigate(`/contracts/${saved.id}`);
        }}
      />

      <FormDialog
        open={terminate}
        onOpenChange={setTerminate}
        title={`Cancel contract ${c.contractNumber}?`}
        description="The units become vacant and any open renewal case is closed. This cannot be undone."
        submitLabel="Cancel the contract"
        destructive
        onSubmit={async () => {
          await api.terminateContract(c.id, reason.trim() || null, endedOn || null);
          await invalidate();
        }}
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <TextField id="term-ended" label="Ended on" type="date" value={endedOn} onChange={setEndedOn} required hint="The day the tenancy actually ends." />
          <TextAreaField id="term-reason" label="Reason (optional)" value={reason} onChange={setReason} className="sm:col-span-2" />
        </div>
      </FormDialog>

      <FormDialog
        open={startRenewal}
        onOpenChange={setStartRenewal}
        title="Start renewal case"
        description={`Opens the renewal workflow for ${c.contractNumber} (${c.tenantName}). The checklist is created from the Admin template.`}
        submitLabel="Start renewal"
        onSubmit={async () => {
          const created = await api.startRenewal(c.id, assignee || null);
          await invalidate();
          navigate(`/renewals/${created.id}`);
        }}
      >
        <SelectField
          id="sr-assignee"
          label="Assigned employee"
          value={assignee}
          onChange={setAssignee}
          options={(employees.data ?? []).map((e) => ({ value: e.id, label: e.name }))}
          placeholder={c.assignedEmployeeName ? `Keep ${c.assignedEmployeeName}` : "Assign to me"}
        />
      </FormDialog>
    </>
  );
}
