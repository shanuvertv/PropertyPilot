import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { selectClass } from "@/components/forms";
import { cn } from "@/lib/utils";

export interface Column<T> {
  key: string;
  header: ReactNode;
  /** Sort key sent to the server; omit for unsortable columns. */
  sort?: string;
  className?: string;
  render: (row: T) => ReactNode;
  /** Phone card layout: `title` is the card heading, `hidden` drops the value (e.g. an action column). */
  card?: "title" | "hidden";
}

interface Props<T> {
  columns: Column<T>[];
  rows: T[] | undefined;
  rowKey: (row: T) => string;
  loading?: boolean;
  error?: string | null;
  empty?: ReactNode;
  sort?: string;
  dir?: "asc" | "desc";
  onSort?: (key: string) => void;
  onRowClick?: (row: T) => void;
}

function StatusRow({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("rounded-md border bg-card py-8 text-center text-[13.5px]", className)}>{children}</div>;
}

/**
 * Sortable server-driven table. On phones (< md) the same columns render as a card
 * per row — the first column (or the one marked `card: "title"`) becomes the heading
 * and the rest become label/value pairs; sorting moves into a select.
 */
export function DataTable<T>({ columns, rows, rowKey, loading, error, empty, sort, dir, onSort, onRowClick }: Props<T>) {
  const titleCol = columns.find((c) => c.card === "title") ?? columns[0];
  const shown = columns.filter((c) => c !== titleCol && c.card !== "hidden");
  // Unlabelled columns are row actions: they go in a footer row instead of the label/value list.
  const detailCols = shown.filter((c) => c.header);
  const actionCols = shown.filter((c) => !c.header);
  const sortable = columns.filter((c) => c.sort);

  return (
    <>
      {/* ---- phone: cards */}
      <div className="flex flex-col gap-2 md:hidden">
        {onSort && sortable.length > 0 && (
          <div className="flex items-center gap-2">
            <label htmlFor="dt-sort" className="shrink-0 text-[12.5px] text-muted-foreground">
              Sort by
            </label>
            <select
              id="dt-sort"
              className={cn(selectClass, "h-9 flex-1")}
              value={sort ?? ""}
              onChange={(e) => e.target.value && onSort(e.target.value)}
            >
              <option value="">Default</option>
              {sortable.map((c) => (
                <option key={c.key} value={c.sort}>
                  {typeof c.header === "string" ? c.header : c.key}
                </option>
              ))}
            </select>
            {sort && (
              <Button variant="outline" size="icon" onClick={() => onSort(sort)} aria-label="Reverse sort order">
                {dir === "desc" ? <ArrowDown /> : <ArrowUp />}
              </Button>
            )}
          </div>
        )}
        {error && <StatusRow className="text-destructive">{error}</StatusRow>}
        {!error && loading && !rows && <StatusRow className="text-muted-foreground">Loading…</StatusRow>}
        {!error && rows && rows.length === 0 && <StatusRow className="text-muted-foreground">{empty ?? "Nothing here yet."}</StatusRow>}
        {rows?.map((row) => {
          const body = (
            <>
              <div className="text-[14.5px] font-medium">{titleCol.render(row)}</div>
              {detailCols.length > 0 && (
                <dl className="mt-1.5 grid grid-cols-[minmax(96px,auto)_1fr] gap-x-3 gap-y-1 text-[13px]">
                  {detailCols.map((c) => (
                    <div key={c.key} className="contents">
                      <dt className="truncate text-muted-foreground">{c.header}</dt>
                      <dd className="min-w-0 break-words">{c.render(row)}</dd>
                    </div>
                  ))}
                </dl>
              )}
              {actionCols.length > 0 && (
                <div className="mt-2 flex flex-wrap items-center justify-end gap-2 border-t pt-2">
                  {actionCols.map((c) => (
                    <div key={c.key}>{c.render(row)}</div>
                  ))}
                </div>
              )}
            </>
          );
          // A div, not a <button>: cards contain their own links and action buttons.
          return onRowClick ? (
            <div
              key={rowKey(row)}
              role="button"
              tabIndex={0}
              className="w-full rounded-lg border bg-card px-3.5 py-3 text-left active:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50 outline-none"
              onClick={() => onRowClick(row)}
              onKeyDown={(e) => {
                if (e.target === e.currentTarget && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  onRowClick(row);
                }
              }}
            >
              {body}
            </div>
          ) : (
            <div key={rowKey(row)} className="rounded-lg border bg-card px-3.5 py-3">
              {body}
            </div>
          );
        })}
      </div>

      {/* ---- desktop: table */}
      <div className="hidden overflow-x-auto rounded-md border bg-card md:block">
        <Table>
          <TableHeader>
            <TableRow>
              {columns.map((c) => (
                <TableHead key={c.key} className={cn("whitespace-nowrap", c.className)}>
                  {c.sort && onSort ? (
                    <button
                      type="button"
                      className="inline-flex items-center gap-1 hover:text-foreground"
                      onClick={() => onSort(c.sort!)}
                    >
                      {c.header}
                      {sort === c.sort ? (
                        dir === "desc" ? (
                          <ArrowDown className="size-3.5" />
                        ) : (
                          <ArrowUp className="size-3.5" />
                        )
                      ) : (
                        <ArrowUpDown className="size-3.5 opacity-40" />
                      )}
                    </button>
                  ) : (
                    c.header
                  )}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {error && (
              <TableRow>
                <TableCell colSpan={columns.length} className="py-6 text-center text-destructive">
                  {error}
                </TableCell>
              </TableRow>
            )}
            {!error && loading && !rows && (
              <TableRow>
                <TableCell colSpan={columns.length} className="py-6 text-center text-muted-foreground">
                  Loading…
                </TableCell>
              </TableRow>
            )}
            {!error && rows && rows.length === 0 && (
              <TableRow>
                <TableCell colSpan={columns.length} className="py-8 text-center text-muted-foreground">
                  {empty ?? "Nothing here yet."}
                </TableCell>
              </TableRow>
            )}
            {rows?.map((row) => (
              <TableRow
                key={rowKey(row)}
                className={cn(onRowClick && "cursor-pointer")}
                onClick={onRowClick ? () => onRowClick(row) : undefined}
              >
                {columns.map((c) => (
                  <TableCell key={c.key} className={c.className}>
                    {c.render(row)}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </>
  );
}

export function Paginator({
  page,
  pageSize,
  total,
  onPage,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPage: (p: number) => void;
}) {
  const pages = Math.max(1, Math.ceil(total / pageSize));
  const from = total === 0 ? 0 : (page - 1) * pageSize + 1;
  const to = Math.min(total, page * pageSize);
  return (
    <div className="mt-3 flex items-center justify-between text-[12.5px] text-muted-foreground">
      <span className="tabular-nums">
        {from}–{to} of {total}
      </span>
      <div className="flex items-center gap-1">
        <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => onPage(page - 1)}>
          Previous
        </Button>
        <span className="px-2 tabular-nums">
          {page} / {pages}
        </span>
        <Button variant="outline" size="sm" disabled={page >= pages} onClick={() => onPage(page + 1)}>
          Next
        </Button>
      </div>
    </div>
  );
}
