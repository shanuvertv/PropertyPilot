import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useNavigate } from "react-router";

import type { Building } from "@/api/types-domain";
import { DataTable, Paginator, useViewMode, ViewToggle, type Column } from "@/components/DataTable";
import { PageHeader } from "@/components/PageHeader";
import { SearchBox } from "@/components/SearchBox";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { useListParams } from "@/lib/list-params";
import { BuildingDialog } from "./BuildingDialog";

export function BuildingsPage() {
  const { api, can } = useApp();
  const navigate = useNavigate();
  const { state, update, toggleSort } = useListParams({ sort: "name" });
  const [dialog, setDialog] = useState(false);
  const [view, setView] = useViewMode("buildings");

  const query = useQuery({
    queryKey: ["buildings", "list", state],
    queryFn: () => api.listBuildings({ q: state.q, page: state.page, pageSize: state.pageSize, sort: state.sort, dir: state.dir }),
    placeholderData: (prev) => prev,
  });

  const columns: Column<Building>[] = [
    { key: "name", header: "Property", sort: "name", render: (b) => <span className="font-medium">{b.name}</span> },
    { key: "code", header: "Code", sort: "code", render: (b) => <span className="font-mono text-[12.5px]">{b.code}</span> },
    { key: "location", header: "Location", sort: "location", render: (b) => b.location ?? "—" },
    { key: "type", header: "Type", sort: "building_type", render: (b) => b.buildingType ?? "—" },
    { key: "total", header: "Units", sort: "total_units", className: "text-right tabular-nums", card: "metric", render: (b) => b.totalUnits },
    { key: "occupied", header: "Occupied", sort: "occupied_units", className: "text-right tabular-nums", card: "metric", render: (b) => b.occupiedUnits },
    { key: "vacant", header: "Vacant", sort: "vacant_units", className: "text-right tabular-nums", card: "metric", render: (b) => b.vacantUnits },
  ];

  return (
    <>
      <PageHeader
        title="Properties / Buildings"
        description="Every property with its unit counts. Open a building for its summary, units and documents."
        actions={
          can("MANAGE_BUILDINGS") && (
            <Button onClick={() => setDialog(true)}>
              <Plus data-icon="inline-start" />
              Add building
            </Button>
          )
        }
      />
      <div className="mb-3 flex items-center gap-2">
        <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search name, code or location" />
        <ViewToggle value={view} onChange={setView} className="ml-auto" />
      </div>
      <DataTable
        columns={columns}
        rows={query.data?.items}
        rowKey={(b) => b.id}
        loading={query.isPending}
        error={query.error ? "Could not load buildings." : null}
        empty="No buildings yet. Add the first property to get started."
        sort={state.sort}
        dir={state.dir}
        onSort={toggleSort}
        onRowClick={(b) => navigate(`/buildings/${b.id}`)}
        view={view}
      />
      {query.data && <Paginator page={state.page} pageSize={state.pageSize} total={query.data.total} onPage={(p) => update({ page: p })} />}
      <BuildingDialog open={dialog} onOpenChange={setDialog} onSaved={(b) => navigate(`/buildings/${b.id}`)} />
    </>
  );
}
