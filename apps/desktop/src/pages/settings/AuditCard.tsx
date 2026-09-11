import { useQuery } from "@tanstack/react-query";

import { AuditList } from "@/components/HistoryPanel";
import { Paginator } from "@/components/DataTable";
import { selectClass } from "@/components/forms";
import { SearchBox } from "@/components/SearchBox";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";
import { useListParams } from "@/lib/list-params";
import { useEmployees } from "@/lib/queries";

const ENTITIES = ["building", "unit", "tenant", "contract", "renewal_case", "user", "email_template", "settings"];

/** Spec §19: the complete audit trail, filterable (Admin / Management; Leasing sees own entries). */
export function AuditCard() {
  const { api } = useApp();
  const employees = useEmployees();
  const { state, update } = useListParams({ pageSize: 25 });
  const rows = useQuery({
    queryKey: ["audit", "list", state],
    queryFn: () => api.auditList({ q: state.q, page: state.page, pageSize: state.pageSize, entityType: state.filters.entityType, actorId: state.filters.actorId }),
    placeholderData: (prev) => prev,
  });
  return (
    <Card>
      <CardHeader>
        <CardTitle>Audit trail</CardTitle>
        <CardDescription>Every change with who made it, when, and the previous and new values. Click an entry to see the diff.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <SearchBox value={state.q} onChange={(q) => update({ q })} placeholder="Search action, user or record type" />
          <select className={`${selectClass} w-auto`} value={state.filters.entityType ?? ""} onChange={(e) => update({ filters: { entityType: e.target.value } })} aria-label="Record type">
            <option value="">All record types</option>
            {ENTITIES.map((e) => (
              <option key={e} value={e}>
                {e.replace("_", " ")}
              </option>
            ))}
          </select>
          <select className={`${selectClass} w-auto`} value={state.filters.actorId ?? ""} onChange={(e) => update({ filters: { actorId: e.target.value } })} aria-label="User">
            <option value="">Any user</option>
            {employees.data?.map((e) => (
              <option key={e.id} value={e.id}>
                {e.name}
              </option>
            ))}
          </select>
        </div>
        <AuditList rows={rows.data?.items} loading={rows.isPending} />
        {rows.data && <Paginator page={state.page} pageSize={state.pageSize} total={rows.data.total} onPage={(p) => update({ page: p })} />}
      </CardContent>
    </Card>
  );
}
