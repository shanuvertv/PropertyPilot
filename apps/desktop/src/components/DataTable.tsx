import { ArrowDown, ArrowUp, ArrowUpDown, LayoutGrid, Rows3 } from "lucide-react";
import { useCallback, useState, type ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { selectClass } from "@/components/forms";
import { cn } from "@/lib/utils";

export type ViewMode = "table" | "cards";

/**
 * Where a column goes on a card: `title` heads the card (default: first column),
 * `subtitle` sits under it, `badge` goes top-right (statuses, chips), `metric` becomes a
 * stat tile (the numbers to see at a glance), `hidden` is dropped. Columns with no role
 * become label/value rows; unlabelled columns are row actions and go in the footer.
 */
export type CardRole = "title" | "subtitle" | "badge" | "metric" | "hidden";

export interface Column<T> {
  key: string;
  header: ReactNode;
  /** Sort key sent to the server; omit for unsortable columns. */
  sort?: string;
  className?: string;
  render: (row: T) => ReactNode;
  card?: CardRole;
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
  /** Desktop layout; phones always show cards. Pair with `useViewMode` + `ViewToggle`. */
  view?: ViewMode;
}

/** Remembers the table / cards choice for one list (per browser, like a folder view). */
export function useViewMode(key: string): [ViewMode, (v: ViewMode) => void] {
  const storageKey = `renewal.view.${key}`;
  const [view, setView] = useState<ViewMode>(() => {
    try {
      return localStorage.getItem(storageKey) === "cards" ? "cards" : "table";
    } catch {
      return "table";
    }
  });
  const set = useCallback(
    (v: ViewMode) => {
      setView(v);
      try {
        localStorage.setItem(storageKey, v);
      } catch {
        // private mode / storage disabled: the choice just lasts for this page
      }
    },
    [storageKey],
  );
  return [view, set];
}

/** Table ⇄ cards switch for a list's toolbar; hidden on phones, which are always cards. */
export function ViewToggle({ value, onChange, className }: { value: ViewMode; onChange: (v: ViewMode) => void; className?: string }) {
  const btn = "inline-flex size-7 items-center justify-center rounded-[6px] text-muted-foreground transition-colors hover:text-foreground aria-pressed:bg-accent aria-pressed:text-foreground [&_svg]:size-4";
  return (
    <div role="group" aria-label="Layout" className={cn("hidden shrink-0 rounded-md border bg-card p-0.5 md:inline-flex", className)}>
      <button type="button" className={btn} aria-pressed={value === "table"} title="Table" aria-label="Table layout" onClick={() => onChange("table")}>
        <Rows3 />
      </button>
      <button type="button" className={btn} aria-pressed={value === "cards"} title="Cards" aria-label="Card layout" onClick={() => onChange("cards")}>
        <LayoutGrid />
      </button>
    </div>
  );
}

function StatusRow({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("rounded-md border bg-card py-8 text-center text-[13.5px]", className)}>{children}</div>;
}

/** Card per row: heading + badges, stat tiles for the metrics, label/value rows, actions. */
function Cards<T>({ columns, rows, rowKey, loading, error, empty, sort, dir, onSort, onRowClick, grid }: Props<T> & { grid: boolean }) {
  const titleCol = columns.find((c) => c.card === "title") ?? columns[0];
  const rest = columns.filter((c) => c !== titleCol && c.card !== "hidden");
  const subtitleCols = rest.filter((c) => c.card === "subtitle");
  const badgeCols = rest.filter((c) => c.card === "badge");
  const metricCols = rest.filter((c) => c.card === "metric");
  const detailCols = rest.filter((c) => !c.card && c.header);
  const actionCols = rest.filter((c) => !c.card && !c.header);
  const sortable = columns.filter((c) => c.sort);
  const metricGrid = metricCols.length === 1 ? "grid-cols-1" : metricCols.length === 2 || metricCols.length === 4 ? "grid-cols-2" : "grid-cols-3";

  return (
    <div className="flex flex-col gap-2">
      {onSort && sortable.length > 0 && (
        <div className="flex items-center gap-2">
          <label htmlFor="dt-sort" className="shrink-0 text-[12.5px] text-muted-foreground">
            Sort by
          </label>
          <select id="dt-sort" className={cn(selectClass, "h-9 flex-1 md:h-8 md:w-auto md:flex-none")} value={sort ?? ""} onChange={(e) => e.target.value && onSort(e.target.value)}>
            <option value="">Default</option>
            {sortable.map((c) => (
              <option key={c.key} value={c.sort}>
                {typeof c.header === "string" ? c.header : c.key}
              </option>
            ))}
          </select>
          {sort && (
            <Button variant="outline" size="icon" className="md:size-8" onClick={() => onSort(sort)} aria-label="Reverse sort order">
              {dir === "desc" ? <ArrowDown /> : <ArrowUp />}
            </Button>
          )}
        </div>
      )}
      {error && <StatusRow className="text-destructive">{error}</StatusRow>}
      {!error && loading && !rows && <StatusRow className="text-muted-foreground">Loading…</StatusRow>}
      {!error && rows && rows.length === 0 && <StatusRow className="text-muted-foreground">{empty ?? "Nothing here yet."}</StatusRow>}
      {rows && rows.length > 0 && (
        <div className={grid ? "grid gap-3 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4" : "flex flex-col gap-2"}>
          {rows.map((row) => {
            // A badge with nothing to say (the "—" placeholder) is noise on a card.
            const badges = badgeCols.map((c) => (
              <div key={c.key} className="has-[[data-empty]]:hidden">
                {c.render(row)}
              </div>
            ));
            const body = (
              <>
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="text-[14.5px] leading-snug font-medium">{titleCol.render(row)}</div>
                    {subtitleCols.map((c) => (
                      <div key={c.key} className="truncate text-[12.5px] text-muted-foreground">
                        {c.render(row)}
                      </div>
                    ))}
                  </div>
                  {badges.length > 0 && <div className="flex shrink-0 flex-wrap items-center justify-end gap-1">{badges}</div>}
                </div>
                {metricCols.length > 0 && (
                  <div className={cn("mt-3 grid gap-2", metricGrid)}>
                    {metricCols.map((c) => (
                      <div key={c.key} className="min-w-0 rounded-md bg-muted/60 px-2.5 py-1.5">
                        <div className="truncate text-[10.5px] font-medium tracking-[0.06em] text-muted-foreground uppercase">{c.header}</div>
                        <div className="mt-0.5 text-[17px] leading-tight font-semibold tabular-nums">{c.render(row)}</div>
                      </div>
                    ))}
                  </div>
                )}
                {detailCols.length > 0 && (
                  <dl className="mt-2.5 grid grid-cols-[minmax(88px,auto)_1fr] gap-x-3 gap-y-1 text-[13px]">
                    {detailCols.map((c) => (
                      <div key={c.key} className="contents">
                        <dt className="truncate text-muted-foreground">{c.header}</dt>
                        <dd className="min-w-0 break-words">{c.render(row)}</dd>
                      </div>
                    ))}
                  </dl>
                )}
                {actionCols.length > 0 && (
                  <div className="mt-auto flex flex-wrap items-center justify-end gap-2 border-t pt-2">
                    {actionCols.map((c) => (
                      <div key={c.key}>{c.render(row)}</div>
                    ))}
                  </div>
                )}
              </>
            );
            const cardClass = "flex flex-col rounded-lg border bg-card px-3.5 py-3 text-left";
            // A div, not a <button>: cards contain their own links and action buttons.
            return onRowClick ? (
              <div
                key={rowKey(row)}
                role="button"
                tabIndex={0}
                className={cn(cardClass, "cursor-pointer outline-none transition-colors hover:border-foreground/25 active:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50")}
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
              <div key={rowKey(row)} className={cardClass}>
                {body}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/**
 * Sortable server-driven list. Desktop shows a table, or a grid of cards when `view`
 * is `"cards"`; phones (< md) always show cards. Column `card` roles decide what a
 * card shows: heading, badges, stat tiles for the key numbers, then label/value rows.
 */
export function DataTable<T>(props: Props<T>) {
  const { columns, rows, rowKey, loading, error, empty, sort, dir, onSort, onRowClick, view = "table" } = props;

  if (view === "cards") return <Cards {...props} grid />;

  return (
    <>
      {/* ---- phone: cards */}
      <div className="md:hidden">
        <Cards {...props} grid={false} />
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
