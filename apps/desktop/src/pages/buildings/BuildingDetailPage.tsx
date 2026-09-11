import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, Pencil, Plus } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router";

import type { UnitSummary } from "@/api/types-domain";
import { ExpiryChip, RenewalStatusBadge, UnitStatusBadge } from "@/components/badges";
import { DataTable, type Column } from "@/components/DataTable";
import { DocumentsPanel } from "@/components/DocumentsPanel";
import { HistoryPanel } from "@/components/HistoryPanel";
import { errorMessage } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useApp } from "@/lib/app-state";
import { formatDate } from "@/lib/format";
import { UnitDialog } from "@/pages/units/UnitDialog";
import { BuildingDialog } from "./BuildingDialog";

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <Card className="gap-1 py-4">
      <CardContent className="px-4">
        <div className="text-[12.5px] text-muted-foreground">{label}</div>
        <div className="mt-1 text-[24px] font-semibold tracking-tight tabular-nums">{value}</div>
      </CardContent>
    </Card>
  );
}

export function BuildingDetailPage() {
  const { id = "" } = useParams();
  const { api, can } = useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [edit, setEdit] = useState(false);
  const [unitDialog, setUnitDialog] = useState<{ open: boolean; unit?: UnitSummary | null }>({ open: false });
  const [error, setError] = useState<string | null>(null);

  const detail = useQuery({ queryKey: ["buildings", id], queryFn: () => api.getBuilding(id) });
  const b = detail.data?.building;
  const s = detail.data?.summary;

  async function archive() {
    if (!b || !window.confirm(`Archive ${b.name} and all its units? This cannot be undone from the app.`)) return;
    try {
      await api.archiveBuilding(b.id);
      await queryClient.invalidateQueries({ queryKey: ["buildings"] });
      navigate("/buildings");
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  const unitColumns: Column<UnitSummary>[] = [
    { key: "unit", header: "Unit", render: (u) => <span className="font-medium">{u.unitNumber}</span> },
    { key: "floor", header: "Floor", render: (u) => u.floor ?? "—" },
    { key: "type", header: "Type", render: (u) => u.unitType ?? "—" },
    { key: "status", header: "Status", render: (u) => <UnitStatusBadge status={u.status} /> },
    { key: "tenant", header: "Tenant", render: (u) => (u.tenantId ? <Link to={`/tenants/${u.tenantId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{u.tenantName}</Link> : "—") },
    { key: "end", header: "Contract end", render: (u) => formatDate(u.endDate) },
    { key: "remaining", header: "Remaining", render: (u) => <ExpiryChip band={u.band} days={u.remainingDays} /> },
    { key: "renewal", header: "Renewal", render: (u) => <RenewalStatusBadge status={u.renewalStatus} /> },
  ];

  if (detail.isError) {
    return (
      <Alert variant="destructive">
        <AlertDescription>{errorMessage(detail.error, "Could not load this building.")}</AlertDescription>
      </Alert>
    );
  }
  if (!b || !s) return <p className="text-muted-foreground">Loading…</p>;

  return (
    <>
      <PageHeader
        title={b.name}
        description={[b.buildingType, b.location].filter(Boolean).join(" · ") || undefined}
        actions={
          <>
            <Badge variant="outline" className="font-mono">
              {b.code}
            </Badge>
            {can("MANAGE_BUILDINGS") && (
              <>
                <Button variant="outline" onClick={() => setEdit(true)}>
                  <Pencil data-icon="inline-start" />
                  Edit
                </Button>
                <Button variant="ghost" onClick={() => void archive()} title="Archive building">
                  <Archive data-icon="inline-start" />
                  Archive
                </Button>
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

      <div className="mb-6 grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-6">
        <Stat label="Total units" value={s.totalUnits} />
        <Stat label="Occupied" value={s.occupiedUnits} />
        <Stat label="Vacant" value={s.vacantUnits} />
        <Stat label="Reserved / maintenance" value={s.reservedUnits + s.maintenanceUnits} />
        <Stat label="Expiring soon" value={s.expiringSoon} />
        <Stat label="Renewals pending" value={s.renewalsPending} />
      </div>

      <Tabs defaultValue="units">
        <TabsList>
          <TabsTrigger value="units">Units ({detail.data?.units.length ?? 0})</TabsTrigger>
          <TabsTrigger value="documents">Documents ({detail.data?.documents.length ?? 0})</TabsTrigger>
          <TabsTrigger value="notes">Notes</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
        <TabsContent value="units" className="pt-4">
          {can("MANAGE_UNITS") && (
            <div className="mb-3 flex justify-end">
              <Button size="sm" onClick={() => setUnitDialog({ open: true, unit: null })}>
                <Plus data-icon="inline-start" />
                Add unit
              </Button>
            </div>
          )}
          <DataTable
            columns={unitColumns}
            rows={detail.data?.units}
            rowKey={(u) => u.id}
            empty="No units in this building yet."
            onRowClick={can("MANAGE_UNITS") ? (u) => setUnitDialog({ open: true, unit: u }) : undefined}
          />
        </TabsContent>
        <TabsContent value="documents" className="pt-4">
          <DocumentsPanel entityType="building" entityId={b.id} canManage={can("MANAGE_BUILDINGS")} />
        </TabsContent>
        <TabsContent value="notes" className="pt-4">
          <p className="max-w-[68ch] whitespace-pre-wrap text-[13.5px]">{b.notes ?? <span className="text-muted-foreground">No notes.</span>}</p>
        </TabsContent>
        <TabsContent value="history" className="pt-4">
          <HistoryPanel entityType="building" entityId={b.id} />
        </TabsContent>
      </Tabs>

      <BuildingDialog open={edit} onOpenChange={setEdit} building={b} onSaved={() => void queryClient.invalidateQueries({ queryKey: ["buildings", id] })} />
      <UnitDialog
        open={unitDialog.open}
        onOpenChange={(o) => setUnitDialog((d) => ({ ...d, open: o }))}
        unit={unitDialog.unit}
        buildingId={b.id}
        onSaved={() => void queryClient.invalidateQueries({ queryKey: ["buildings", id] })}
      />
    </>
  );
}
