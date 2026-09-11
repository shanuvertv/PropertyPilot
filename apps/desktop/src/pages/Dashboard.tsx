import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "react-router";

import type { Contract, RenewalCase } from "@/api/types-domain";
import { BandDot, ExpiryChip, RenewalStatusBadge } from "@/components/badges";
import { DataTable, type Column } from "@/components/DataTable";
import { errorMessage } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";
import { BAND_LABEL } from "@/lib/bands";
import { BAND_ORDER, formatDate, formatDateTime } from "@/lib/format";

function Stat({ label, value, to }: { label: string; value: number | undefined; to?: string }) {
  const body = (
    <Card className="h-full gap-1 py-4 transition-colors hover:bg-muted/40">
      <CardContent className="px-4">
        <div className="text-[12.5px] text-muted-foreground">{label}</div>
        <div className="mt-1 text-[26px] font-semibold tracking-tight tabular-nums">{value ?? "—"}</div>
      </CardContent>
    </Card>
  );
  return to ? <Link to={to}>{body}</Link> : body;
}

export function DashboardPage() {
  const { api, session, can } = useApp();
  const navigate = useNavigate();
  const firstName = session?.name.split(" ")[0] ?? "";
  const dash = useQuery({ queryKey: ["dashboard"], queryFn: () => api.dashboard(), refetchInterval: 60_000 });
  const c = dash.data?.counts;
  const bandCount = (b: string) => dash.data?.bands.find((x) => x.band === b)?.count ?? 0;

  const contractCols = (action: (c: Contract) => React.ReactNode): Column<Contract>[] => [
    { key: "tenant", header: "Tenant", render: (x) => <span className="font-medium">{x.tenantName}</span> },
    { key: "building", header: "Building", render: (x) => x.buildingName },
    { key: "unit", header: "Unit", render: (x) => x.unitNumbers },
    { key: "end", header: "Contract end", render: (x) => formatDate(x.endDate) },
    { key: "remaining", header: "Remaining", render: (x) => <ExpiryChip band={x.band} days={x.remainingDays} /> },
    { key: "status", header: "Renewal status", render: (x) => (x.caseId ? <RenewalStatusBadge status={x.renewalStatus} /> : <span className="text-muted-foreground">Not started</span>) },
    { key: "action", header: "Action", className: "text-right whitespace-nowrap", render: (x) => <span onClick={(e) => e.stopPropagation()}>{action(x)}</span> },
  ];
  const openOrStart = (x: Contract) =>
    x.caseId ? (
      <Button size="xs" variant="ghost" onClick={() => navigate(`/renewals/${x.caseId}`)}>
        Open case
      </Button>
    ) : can("MANAGE_RENEWALS") ? (
      <Button size="xs" variant="outline" onClick={() => navigate(`/contracts/${x.id}?start=renewal`)}>
        Start renewal
      </Button>
    ) : (
      <Button size="xs" variant="ghost" onClick={() => navigate(`/contracts/${x.id}`)}>
        View
      </Button>
    );

  const completedCols: Column<RenewalCase>[] = [
    { key: "tenant", header: "Tenant", render: (x) => <span className="font-medium">{x.tenantName}</span> },
    { key: "building", header: "Building", render: (x) => x.buildingName },
    { key: "unit", header: "Unit", render: (x) => x.unitNumbers },
    { key: "old", header: "Previous contract", render: (x) => x.contractNumber },
    { key: "new", header: "New contract", render: (x) => (x.outcomeContractId ? <Link to={`/contracts/${x.outcomeContractId}`} className="hover:underline" onClick={(e) => e.stopPropagation()}>{x.outcomeContractNumber}</Link> : "—") },
    { key: "when", header: "Completed", render: (x) => formatDateTime(x.closedAt) },
    { key: "by", header: "Assigned", render: (x) => x.assignedEmployeeName ?? "—" },
  ];

  const section = (title: string, node: React.ReactNode, id: string) => (
    <section aria-labelledby={id} className="mb-8">
      <h2 id={id} className="mb-3 text-[11px] font-medium tracking-[0.08em] text-muted-foreground uppercase">
        {title}
      </h2>
      {node}
    </section>
  );

  return (
    <>
      <PageHeader title={`Good day, ${firstName}`} description="Unit-wise status and upcoming contract expiries. Nothing here should be a surprise to the leasing team." />
      {dash.isError && (
        <Alert variant="destructive" className="mb-4">
          <AlertDescription>{errorMessage(dash.error, "Could not load the dashboard.")}</AlertDescription>
        </Alert>
      )}

      {section(
        "Summary",
        <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-5">
          <Stat label="Total Buildings" value={c?.totalBuildings} to="/buildings" />
          <Stat label="Total Units" value={c?.totalUnits} to="/units" />
          <Stat label="Occupied Units" value={c?.occupiedUnits} to="/units?status=OCCUPIED" />
          <Stat label="Vacant Units" value={c?.vacantUnits} to="/units?status=VACANT" />
          <Stat label="Active Tenants" value={c?.activeTenants} to="/tenants" />
          <Stat label="Active Contracts" value={c?.activeContracts} to="/contracts?status=ACTIVE" />
          <Stat label="Contracts Expiring Soon" value={c?.expiringSoon} to="/renewals" />
          <Stat label="Renewals Pending" value={c?.renewalsPending} to="/renewals?hasOpenCase=true" />
          <Stat label={`Renewals Completed (${dash.data?.completedWindowDays ?? 90}d)`} value={c?.renewalsCompleted} />
          <Stat label="Expired Contracts" value={c?.expiredContracts} to="/contracts?band=EXPIRED" />
        </div>,
        "summary-h",
      )}

      {section(
        "Expiry summary",
        <Card className="py-4">
          <CardContent className="grid grid-cols-2 gap-x-6 gap-y-3 px-5 md:grid-cols-3 xl:grid-cols-6">
            {BAND_ORDER.map((band) => (
              <Link key={band} to={`/renewals?band=${band}`} className="flex items-center gap-2.5 rounded-md p-1 hover:bg-muted">
                <BandDot band={band} />
                <div className="min-w-0">
                  <div className="truncate text-[13px]">{BAND_LABEL[band]}</div>
                  <div className="text-[18px] font-semibold tabular-nums">{dash.data ? bandCount(band) : "—"}</div>
                </div>
              </Link>
            ))}
            <div className="col-span-full flex gap-6 border-t pt-3 text-[12.5px] text-muted-foreground">
              <span>
                Follow-ups today <b className="text-foreground tabular-nums">{dash.data?.followUps.today ?? "—"}</b>
              </span>
              <span>
                Overdue <b className="text-destructive tabular-nums">{dash.data?.followUps.overdue ?? "—"}</b>
              </span>
              <Link to="/follow-ups" className="ml-auto text-primary hover:underline">
                Open follow-ups →
              </Link>
            </div>
          </CardContent>
        </Card>,
        "expiry-h",
      )}

      {section("Urgent renewals", <DataTable columns={contractCols(openOrStart)} rows={dash.data?.urgentRenewals} rowKey={(x) => x.id} empty="No urgent renewals." onRowClick={(x) => navigate(x.caseId ? `/renewals/${x.caseId}` : `/contracts/${x.id}`)} />, "urgent-h")}
      {section("Upcoming contract expiries", <DataTable columns={contractCols(openOrStart)} rows={dash.data?.upcomingExpiries} rowKey={(x) => x.id} empty="No other contracts expiring soon." onRowClick={(x) => navigate(x.caseId ? `/renewals/${x.caseId}` : `/contracts/${x.id}`)} />, "upcoming-h")}
      {can("VIEW_RENEWALS") && (
        <>
          {section("Pending tenant responses", <DataTable columns={contractCols(openOrStart)} rows={dash.data?.pendingTenantResponses} rowKey={(x) => x.id} empty="No responses pending." onRowClick={(x) => navigate(`/renewals/${x.caseId}`)} />, "responses-h")}
          {section("Renewal notices pending", <DataTable columns={contractCols(openOrStart)} rows={dash.data?.noticesPending} rowKey={(x) => x.id} empty="No notices pending." onRowClick={(x) => navigate(`/renewals/${x.caseId}`)} />, "notices-h")}
          {section("Recently completed renewals", <DataTable columns={completedCols} rows={dash.data?.recentlyCompleted} rowKey={(x) => x.id} empty="No renewals completed in this period." onRowClick={(x) => navigate(`/renewals/${x.id}`)} />, "completed-h")}
        </>
      )}
    </>
  );
}
