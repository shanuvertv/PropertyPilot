import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { FileDown, FileSpreadsheet } from "lucide-react";

import type { ReportKind, ReportParams } from "@/api/types-domain";
import { saveBlob } from "@/components/DocumentsPanel";
import { errorMessage, selectClass } from "@/components/forms";
import { PageHeader } from "@/components/PageHeader";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";
import { formatDateTime } from "@/lib/format";
import { useListParams } from "@/lib/list-params";
import { useBuildingOptions, useEmployees } from "@/lib/queries";
import { cn } from "@/lib/utils";

const REPORTS: { kind: ReportKind; label: string; hint: string }[] = [
  { kind: "contract-expiry", label: "Contract Expiry", hint: "All contracts grouped by expiry period." },
  { kind: "renewal-status", label: "Renewal Status", hint: "Pending · In progress · Completed · Not renewing." },
  { kind: "unit-summary", label: "Unit-Wise Summary", hint: "Every unit with its tenant and contract status." },
  { kind: "tenant-renewal", label: "Tenant Renewal", hint: "Complete renewal history per tenant." },
  { kind: "notice-tracking", label: "Notice Tracking", hint: "Notice pending / sent, tenant response, follow-ups." },
];

/** Spec §17: five reports, filterable, exportable to Excel and PDF. */
export function ReportsPage() {
  const { api } = useApp();
  const buildings = useBuildingOptions();
  const employees = useEmployees();
  const { state, update } = useListParams();
  const kind = (state.filters.kind as ReportKind | undefined) ?? "contract-expiry";
  const params: ReportParams = { buildingId: state.filters.buildingId, employeeId: state.filters.employeeId, from: state.filters.from, to: state.filters.to };
  const [error, setError] = useState<string | null>(null);

  const report = useQuery({ queryKey: ["report", kind, params], queryFn: () => api.report(kind, params), placeholderData: (prev) => prev });
  const exportFile = useMutation({
    mutationFn: async (format: "xlsx" | "pdf") => {
      const blob = await api.reportFile(kind, params, format);
      await saveBlob(blob, `${report.data?.title ?? kind} ${new Date().toISOString().slice(0, 10)}.${format}`);
    },
    onError: (e) => setError(errorMessage(e, "Export failed.")),
  });

  return (
    <>
      <PageHeader
        title="Reports"
        description="Operational reports only — no accounting. Export any of them to Excel or PDF."
        actions={
          <>
            <Button variant="outline" onClick={() => exportFile.mutate("xlsx")} disabled={exportFile.isPending || !report.data}>
              <FileSpreadsheet data-icon="inline-start" />
              Excel
            </Button>
            <Button variant="outline" onClick={() => exportFile.mutate("pdf")} disabled={exportFile.isPending || !report.data}>
              <FileDown data-icon="inline-start" />
              PDF
            </Button>
          </>
        }
      />
      {error && (
        <Alert variant="destructive" className="mb-3">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      <div className="mb-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
        {REPORTS.map((r) => (
          <button
            key={r.kind}
            type="button"
            className={cn("rounded-md border bg-card px-3 py-2.5 text-left transition-colors hover:bg-muted", kind === r.kind && "border-primary ring-2 ring-primary/20")}
            onClick={() => update({ filters: { kind: r.kind } })}
            aria-pressed={kind === r.kind}
          >
            <div className="text-[13.5px] font-medium">{r.label}</div>
            <div className="text-[12px] text-muted-foreground">{r.hint}</div>
          </button>
        ))}
      </div>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <select className={`${selectClass} w-auto`} value={state.filters.buildingId ?? ""} onChange={(e) => update({ filters: { buildingId: e.target.value } })} aria-label="Building">
          <option value="">All buildings</option>
          {buildings.data?.map((b) => (
            <option key={b.id} value={b.id}>
              {b.name}
            </option>
          ))}
        </select>
        {(kind === "renewal-status" || kind === "notice-tracking") && (
          <select className={`${selectClass} w-auto`} value={state.filters.employeeId ?? ""} onChange={(e) => update({ filters: { employeeId: e.target.value } })} aria-label="Employee">
            <option value="">Any employee</option>
            {employees.data?.map((e) => (
              <option key={e.id} value={e.id}>
                {e.name}
              </option>
            ))}
          </select>
        )}
        {(kind === "contract-expiry" || kind === "renewal-status") && (
          <>
            <label className="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
              {kind === "contract-expiry" ? "Ends from" : "Opened from"}
              <Input type="date" className="w-auto" value={state.filters.from ?? ""} onChange={(e) => update({ filters: { from: e.target.value } })} />
            </label>
            {kind === "contract-expiry" && (
              <label className="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
                to
                <Input type="date" className="w-auto" value={state.filters.to ?? ""} onChange={(e) => update({ filters: { to: e.target.value } })} />
              </label>
            )}
          </>
        )}
      </div>

      {report.isError && (
        <Alert variant="destructive">
          <AlertDescription>{errorMessage(report.error, "Could not build the report.")}</AlertDescription>
        </Alert>
      )}
      {report.data && (
        <div className="rounded-md border bg-card">
          <div className="border-b px-5 py-3">
            <div className="text-[15px] font-semibold">{report.data.title}</div>
            <div className="text-[12.5px] text-muted-foreground">
              {report.data.subtitle} · {report.data.totalRows} rows · generated {formatDateTime(report.data.generatedAt)}
            </div>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-[13px]">
              <thead className="bg-muted/60 text-[11px] tracking-[0.04em] text-muted-foreground uppercase">
                <tr>
                  {report.data.columns.map((c) => (
                    <th key={c} className="px-3 py-2 text-left font-medium whitespace-nowrap">
                      {c}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {report.data.sections.map((s) => (
                  <SectionRows key={s.heading} heading={s.heading} rows={s.rows} cols={report.data!.columns.length} />
                ))}
                {report.data.totalRows === 0 && (
                  <tr>
                    <td colSpan={report.data.columns.length} className="px-3 py-8 text-center text-muted-foreground">
                      No records for these filters.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </>
  );
}

function SectionRows({ heading, rows, cols }: { heading: string; rows: string[][]; cols: number }) {
  return (
    <>
      <tr className="border-t bg-primary/5">
        <td colSpan={cols} className="px-3 py-1.5 text-[12.5px] font-semibold text-primary">
          {heading} <span className="font-normal text-muted-foreground">({rows.length})</span>
        </td>
      </tr>
      {rows.map((r, i) => (
        <tr key={i} className="border-t">
          {r.map((v, j) => (
            <td key={j} className={cn("px-3 py-1.5 whitespace-nowrap", /^-?\d+$/.test(v) && "text-right tabular-nums")}>
              {v}
            </td>
          ))}
        </tr>
      ))}
      {rows.length === 0 && (
        <tr className="border-t">
          <td colSpan={cols} className="px-3 py-2 text-[12.5px] text-muted-foreground">
            None.
          </td>
        </tr>
      )}
    </>
  );
}
