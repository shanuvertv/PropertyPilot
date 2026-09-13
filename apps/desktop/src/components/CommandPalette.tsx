import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Building2, DoorOpen, FileText, Receipt, Search, Users } from "lucide-react";
import { useNavigate } from "react-router";

import type { SearchHit, SearchKind } from "@/api/types-domain";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { useApp } from "@/lib/app-state";
import { cn } from "@/lib/utils";

const ICON: Record<SearchKind, typeof Building2> = { building: Building2, unit: DoorOpen, tenant: Users, contract: FileText, expense: Receipt };
const LABEL: Record<SearchKind, string> = { building: "Building", unit: "Unit", tenant: "Tenant", contract: "Contract", expense: "Expense" };

function target(hit: SearchHit): string {
  switch (hit.kind) {
    case "building":
      return `/buildings/${hit.id}`;
    case "unit":
      return `/units/${hit.id}`;
    case "expense":
      return `/expenses/${hit.id}`;
    case "tenant":
      return `/tenants/${hit.id}`;
    case "contract":
      return `/contracts/${hit.id}`;
  }
}

/** Ctrl+K global search across buildings, units, tenants, contracts and expenses. */
export function CommandPalette() {
  const { api } = useApp();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((o) => !o);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (!open) {
      setQ("");
      setActive(0);
    }
  }, [open]);

  const hits = useQuery({ queryKey: ["search", q], queryFn: () => api.search(q), enabled: open && q.trim().length >= 2, staleTime: 5_000 });
  const list = hits.data ?? [];

  function go(hit: SearchHit) {
    setOpen(false);
    navigate(target(hit));
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent showCloseButton={false} className="top-[15%] translate-y-0 p-0 sm:max-w-lg">
        <DialogTitle className="sr-only">Search</DialogTitle>
        <div className="flex items-center gap-2 border-b px-3">
          <Search className="size-4 text-muted-foreground" aria-hidden="true" />
          <input
            autoFocus
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((a) => Math.min(a + 1, list.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((a) => Math.max(a - 1, 0));
              } else if (e.key === "Enter" && list[active]) {
                go(list[active]);
              }
            }}
            placeholder="Search tenants, units, buildings, contracts…"
            className="h-11 w-full bg-transparent text-[14px] outline-none placeholder:text-muted-foreground"
            aria-label="Search"
          />
          <kbd className="rounded border px-1.5 text-[10px] text-muted-foreground">Esc</kbd>
        </div>
        <ul className="max-h-80 overflow-y-auto p-1.5" role="listbox">
          {q.trim().length < 2 && <li className="px-3 py-6 text-center text-[13px] text-muted-foreground">Type at least two characters.</li>}
          {q.trim().length >= 2 && hits.isSuccess && list.length === 0 && <li className="px-3 py-6 text-center text-[13px] text-muted-foreground">No matches.</li>}
          {list.map((hit, i) => {
            const Icon = ICON[hit.kind];
            return (
              <li
                key={`${hit.kind}-${hit.id}`}
                role="option"
                aria-selected={i === active}
                className={cn("flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 text-[13.5px]", i === active && "bg-muted")}
                onMouseEnter={() => setActive(i)}
                onClick={() => go(hit)}
              >
                <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{hit.title}</span>
                  <span className="block truncate text-[12px] text-muted-foreground">{hit.subtitle}</span>
                </span>
                <span className="text-[11px] text-muted-foreground uppercase">{LABEL[hit.kind]}</span>
              </li>
            );
          })}
        </ul>
      </DialogContent>
    </Dialog>
  );
}
