import { useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { FileSpreadsheet, Upload } from "lucide-react";

import { ApiRequestError } from "@/api/client";
import type { ImportPreview, ImportResult } from "@/api/types-domain";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useApp } from "@/lib/app-state";
import { formatDate, formatMoney } from "@/lib/format";

type Stage =
  | { kind: "idle" }
  | { kind: "previewing" }
  | { kind: "preview"; preview: ImportPreview }
  | { kind: "committing"; preview: ImportPreview }
  | { kind: "done"; result: ImportResult };

function errorText(e: unknown, fallback: string): string {
  return e instanceof ApiRequestError ? e.message : fallback;
}

/**
 * Excel import (spec §20.2): pick a tenant-list workbook, review what would be created,
 * then commit. Re-running the same file is safe — existing buildings, units, tenants
 * and contracts are matched, never duplicated.
 */
export function ImportCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const fileInput = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [stage, setStage] = useState<Stage>({ kind: "idle" });
  const [error, setError] = useState<string | null>(null);

  async function loadPreview(f: File) {
    setFile(f);
    setError(null);
    setStage({ kind: "previewing" });
    try {
      const p = await api.importPreview(f);
      setStage({ kind: "preview", preview: p });
    } catch (e) {
      setError(errorText(e, "The workbook could not be read."));
      setStage({ kind: "idle" });
    }
  }

  async function commit() {
    if (!file || stage.kind !== "preview") return;
    setError(null);
    setStage({ kind: "committing", preview: stage.preview });
    try {
      const result = await api.importCommit(file);
      setStage({ kind: "done", result });
      await queryClient.invalidateQueries();
    } catch (e) {
      setError(errorText(e, "The import failed; nothing was written."));
      setStage({ kind: "preview", preview: stage.preview });
    }
  }

  function reset() {
    setFile(null);
    setError(null);
    setStage({ kind: "idle" });
    if (fileInput.current) fileInput.current.value = "";
  }

  const busy = stage.kind === "previewing" || stage.kind === "committing";
  const preview = stage.kind === "preview" || stage.kind === "committing" ? stage.preview : null;
  const importable = preview ? preview.contracts.filter((c) => !c.skip).length : 0;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Import tenant list</CardTitle>
        <CardDescription>
          Load buildings, units, tenants and contracts from an Excel tenant list. Nothing is written until you confirm the
          preview, and re-importing the same file never creates duplicates.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center gap-3">
          <input
            ref={fileInput}
            type="file"
            accept=".xlsx,.xlsm,.xls"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void loadPreview(f);
            }}
          />
          <Button variant="outline" disabled={busy} onClick={() => fileInput.current?.click()}>
            <FileSpreadsheet />
            {file ? "Choose another file" : "Choose workbook…"}
          </Button>
          {file && (
            <span className="text-[13px] text-muted-foreground">
              {file.name} · {(file.size / 1024).toFixed(0)} KB
            </span>
          )}
          {stage.kind === "previewing" && <span className="text-[13px] text-muted-foreground">Reading workbook…</span>}
        </div>

        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {preview && (
          <>
            <dl className="grid grid-cols-2 gap-x-6 gap-y-1.5 text-[13px] sm:grid-cols-4">
              <dt className="text-muted-foreground">Rows read</dt>
              <dd className="tabular-nums">
                {preview.rowsRead}
                {preview.rowsSkipped > 0 && <span className="text-muted-foreground"> ({preview.rowsSkipped} skipped)</span>}
              </dd>
              <dt className="text-muted-foreground">Buildings</dt>
              <dd className="tabular-nums">
                {preview.buildingsNew.length} new · {preview.buildingsExisting.length} existing
              </dd>
              <dt className="text-muted-foreground">Units</dt>
              <dd className="tabular-nums">
                {preview.unitsNew} new · {preview.unitsExisting} existing
              </dd>
              <dt className="text-muted-foreground">Tenants</dt>
              <dd className="tabular-nums">
                {preview.tenantsNew.length} new · {preview.tenantsExisting.length} existing
              </dd>
            </dl>

            <div className="break-words text-[12px] text-muted-foreground">
              Columns recognised:{" "}
              {Object.entries(preview.columns)
                .map(([k, v]) => `${k} ← “${v}”`)
                .join(", ")}
            </div>

            {preview.warnings.length > 0 && (
              <Alert>
                <AlertDescription>
                  <ul className="list-disc pl-4">
                    {preview.warnings.map((w, i) => (
                      <li key={i}>{w}</li>
                    ))}
                  </ul>
                </AlertDescription>
              </Alert>
            )}

            <div className="overflow-x-auto rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Building</TableHead>
                    <TableHead>Tenant</TableHead>
                    <TableHead>Units</TableHead>
                    <TableHead>Start</TableHead>
                    <TableHead>End</TableHead>
                    <TableHead className="text-right">Occupants</TableHead>
                    <TableHead className="text-right">Rent / annum</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Notes</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {preview.contracts.map((c, i) => (
                    <TableRow key={i} className={c.skip ? "opacity-60" : undefined}>
                      <TableCell className="whitespace-nowrap">{c.building}</TableCell>
                      <TableCell className="font-medium">{c.tenant}</TableCell>
                      <TableCell className="min-w-[200px] max-w-[360px] whitespace-normal text-[12.5px]">
                        <span className="font-medium tabular-nums">{c.units.length}</span>
                        <span className="text-muted-foreground"> · {c.units.join(", ")}</span>
                      </TableCell>
                      <TableCell className="tabular-nums whitespace-nowrap">{formatDate(c.start)}</TableCell>
                      <TableCell className="tabular-nums whitespace-nowrap">{formatDate(c.end)}</TableCell>
                      <TableCell className="text-right tabular-nums">{c.tenants}</TableCell>
                      <TableCell className="text-right tabular-nums whitespace-nowrap">{c.rentAmount === null ? "—" : formatMoney(c.rentAmount)}</TableCell>
                      <TableCell>
                        <Badge variant={c.skip ? "outline" : c.status === "EXPIRED" ? "destructive" : "secondary"}>
                          {c.skip ? "Already imported" : c.status === "EXPIRED" ? "Expired" : "Active"}
                        </Badge>
                      </TableCell>
                      <TableCell className="min-w-[160px] whitespace-normal text-[12px] text-muted-foreground">{c.warnings.join("; ")}</TableCell>
                    </TableRow>
                  ))}
                  {preview.contracts.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={9} className="py-6 text-center text-muted-foreground">
                        No contracts found in this workbook.
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </div>

            <div className="flex items-center justify-between gap-3">
              <span className="text-[13px] text-muted-foreground">
                {importable} contract{importable === 1 ? "" : "s"} will be created
                {preview.contracts.length - importable > 0 && `; ${preview.contracts.length - importable} already exist`}.
              </span>
              <div className="flex gap-2">
                <Button variant="ghost" disabled={busy} onClick={reset}>
                  Cancel
                </Button>
                <Button disabled={busy || importable === 0} onClick={() => void commit()}>
                  <Upload />
                  {stage.kind === "committing" ? "Importing…" : "Import"}
                </Button>
              </div>
            </div>
          </>
        )}

        {stage.kind === "done" && (
          <>
            <Alert>
              <AlertDescription>
                Imported {stage.result.contractsCreated} contract{stage.result.contractsCreated === 1 ? "" : "s"},{" "}
                {stage.result.buildingsCreated} building{stage.result.buildingsCreated === 1 ? "" : "s"}, {stage.result.unitsCreated}{" "}
                unit{stage.result.unitsCreated === 1 ? "" : "s"} and {stage.result.tenantsCreated} tenant
                {stage.result.tenantsCreated === 1 ? "" : "s"}
                {stage.result.contractsSkipped > 0 && ` (${stage.result.contractsSkipped} already existed)`}.
              </AlertDescription>
            </Alert>
            {stage.result.warnings.length > 0 && (
              <ul className="list-disc pl-5 text-[13px] text-muted-foreground">
                {stage.result.warnings.map((w, i) => (
                  <li key={i}>{w}</li>
                ))}
              </ul>
            )}
            <div>
              <Button variant="outline" onClick={reset}>
                Import another file
              </Button>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
