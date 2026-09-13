import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Pencil } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router";

import type { Contract, RenewalCase } from "@/api/types-domain";
import { ContractStatusBadge, ExpiryChip, NoticeStatusBadge, RenewalStatusBadge } from "@/components/badges";
import { DataTable, type Column } from "@/components/DataTable";
import { DocumentsPanel } from "@/components/DocumentsPanel";
import { HistoryPanel } from "@/components/HistoryPanel";
import { errorMessage } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { useLocalFilter } from "@/lib/local-filter";
import { SearchBox } from "@/components/SearchBox";
import { TENANT_RESPONSE_LABEL, formatDate, formatDateTime } from "@/lib/format";
import { TenantDialog } from "./TenantDialog";
import { TenantEmails } from "./TenantEmails";

export function TenantDetailPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [edit, setEdit] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const detail = useQuery({ queryKey: ["tenants", id], queryFn: () => api.getTenant(id) });
  const contractFilter = useLocalFilter(detail.data?.contracts, (c) => [c.contractNumber, c.buildingName, c.unitNumbers, c.status, c.startDate, c.endDate]);
  const t = detail.data?.tenant;

  async function archive() {
    if (!t || !window.confirm(`Archive ${t.name}?`)) return;
    try {
      await api.archiveTenant(t.id);
      await queryClient.invalidateQueries({ queryKey: ["tenants"] });
      navigate("/tenants");
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  const contractCols: Column<Contract>[] = [
    { key: "no", header: "Contract", render: (c) => <Link to={`/contracts/${c.id}`} className="font-medium hover:underline">{c.contractNumber}</Link> },
    { key: "building", header: "Building", render: (c) => c.buildingName },
    { key: "units", header: "Units", render: (c) => c.unitNumbers },
    { key: "start", header: "Start", render: (c) => formatDate(c.startDate) },
    { key: "end", header: "End", render: (c) => formatDate(c.endDate) },
    { key: "remaining", header: "Remaining", render: (c) => (c.status === "ACTIVE" ? <ExpiryChip band={c.band} days={c.remainingDays} /> : "—") },
    { key: "status", header: "Status", render: (c) => <ContractStatusBadge status={c.status} /> },
    { key: "renewal", header: "Renewal", render: (c) => <RenewalStatusBadge status={c.renewalStatus} /> },
  ];

  const caseCols: Column<RenewalCase>[] = [
    { key: "opened", header: "Opened", render: (c) => formatDate(c.openedAt) },
    { key: "contract", header: "Contract", render: (c) => <Link to={`/contracts/${c.contractId}`} className="hover:underline">{c.contractNumber}</Link> },
    { key: "status", header: "Status", render: (c) => <RenewalStatusBadge status={c.status} /> },
    { key: "notice", header: "Notice", render: (c) => <NoticeStatusBadge status={c.noticeStatus} /> },
    { key: "response", header: "Tenant response", render: (c) => (c.latestResponse ? `${TENANT_RESPONSE_LABEL[c.latestResponse]} · ${formatDateTime(c.latestResponseAt)}` : "—") },
    { key: "outcome", header: "Outcome", render: (c) => (c.outcomeContractId ? <Link to={`/contracts/${c.outcomeContractId}`} className="hover:underline">{c.outcomeContractNumber}</Link> : "—") },
  ];

  if (detail.isError) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Could not load this tenant.")}</AlertDescription>
      </Alert>
    );
  }
  if (!t || !detail.data) return <p className="text-muted-foreground">Loading…</p>;

  const contracts = contractFilter.filtered ?? [];
  const current = contracts.filter((c) => c.status === "ACTIVE" || c.status === "DRAFT");
  const history = contracts.filter((c) => c.status !== "ACTIVE" && c.status !== "DRAFT");

  return (
    <>
      <PageHeader
        title={t.name}
        description={[t.contactPerson, t.mobile, t.email].filter(Boolean).join(" · ") || undefined}
        actions={
          can("MANAGE_TENANTS") && (
            <>
              <Button variant="outline" onClick={() => setEdit(true)}>
                <Pencil data-icon="inline-start" />
                Edit
              </Button>
              <Button variant="ghost" onClick={() => void archive()}>
                <Archive data-icon="inline-start" />
                Archive
              </Button>
            </>
          )
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="current">
        <TabsList>
          <TabsTrigger value="current">Current ({current.length})</TabsTrigger>
          <TabsTrigger value="past">Past contracts ({history.length + detail.data.cases.length})</TabsTrigger>
          <TabsTrigger value="communications">Communications</TabsTrigger>
          <TabsTrigger value="documents">Documents ({detail.data.documents.length})</TabsTrigger>
          <TabsTrigger value="details">Details</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
        <TabsContent value="current" className="pt-4">
          <div className="mb-3"><SearchBox value={contractFilter.q} onChange={contractFilter.setQ} placeholder="Search contract, building, unit or status" /></div>
          <DataTable columns={contractCols} rows={current} rowKey={(c) => c.id} empty="No active contracts. Create one from Contracts." />
        </TabsContent>
        <TabsContent value="past" className="flex flex-col gap-6 pt-4">
          <SearchBox value={contractFilter.q} onChange={contractFilter.setQ} placeholder="Search contract, building, unit or status" />
          <section>
            <h3 className="mb-2 text-[11px] font-medium tracking-[0.08em] text-muted-foreground uppercase">Previous contracts</h3>
            <DataTable columns={contractCols} rows={history} rowKey={(c) => c.id} empty="No previous contracts." />
          </section>
          <section>
            <h3 className="mb-2 text-[11px] font-medium tracking-[0.08em] text-muted-foreground uppercase">Renewals, notices and responses</h3>
            <DataTable columns={caseCols} rows={detail.data.cases} rowKey={(c) => c.id} empty="No renewal cases yet." onRowClick={can("VIEW_RENEWALS") ? (c) => navigate(`/renewals/${c.id}`) : undefined} />
          </section>
        </TabsContent>
        <TabsContent value="communications" className="pt-4">
          <TenantEmails tenantId={t.id} tenantEmail={t.email} />
        </TabsContent>
        <TabsContent value="documents" className="pt-4">
          <DocumentsPanel entityType="tenant" entityId={t.id} canManage={can("MANAGE_TENANTS")} />
        </TabsContent>
        <TabsContent value="details" className="pt-4">
          <dl className="grid max-w-[640px] grid-cols-[160px_1fr] gap-x-4 gap-y-2 text-[13.5px]">
            <dt className="text-muted-foreground">Contact person</dt>
            <dd>{t.contactPerson ?? "—"}</dd>
            <dt className="text-muted-foreground">Mobile</dt>
            <dd>{t.mobile ?? "—"}</dd>
            <dt className="text-muted-foreground">Email</dt>
            <dd>{t.email ?? "—"}</dd>
            <dt className="text-muted-foreground">Alternative contact</dt>
            <dd>{t.altContact ?? "—"}</dd>
            <dt className="text-muted-foreground">Address</dt>
            <dd className="whitespace-pre-wrap">{t.address ?? "—"}</dd>
            <dt className="text-muted-foreground">Notes</dt>
            <dd className="whitespace-pre-wrap">{t.notes ?? "—"}</dd>
          </dl>
        </TabsContent>
        <TabsContent value="history" className="pt-4">
          <HistoryPanel entityType="tenant" entityId={t.id} />
        </TabsContent>
      </Tabs>

      <TenantDialog open={edit} onOpenChange={setEdit} tenant={t} onSaved={() => void queryClient.invalidateQueries({ queryKey: ["tenants", id] })} />
    </>
  );
}
