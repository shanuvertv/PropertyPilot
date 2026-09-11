import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router";

/**
 * List state (search, filters, sort, page) kept in the URL so it survives
 * navigating into a record and back, and can be shared as a link.
 */
export interface ListState {
  q: string;
  page: number;
  pageSize: number;
  sort: string | undefined;
  dir: "asc" | "desc";
  filters: Record<string, string>;
}

const RESERVED = new Set(["q", "page", "pageSize", "sort", "dir"]);

export function useListParams(defaults: { sort?: string; dir?: "asc" | "desc"; pageSize?: number } = {}) {
  const [params, setParams] = useSearchParams();

  const state: ListState = useMemo(() => {
    const filters: Record<string, string> = {};
    params.forEach((v, k) => {
      if (!RESERVED.has(k) && v !== "") filters[k] = v;
    });
    return {
      q: params.get("q") ?? "",
      page: Math.max(1, Number(params.get("page") ?? 1) || 1),
      pageSize: Number(params.get("pageSize") ?? defaults.pageSize ?? 25) || 25,
      sort: params.get("sort") ?? defaults.sort,
      dir: (params.get("dir") as "asc" | "desc" | null) ?? defaults.dir ?? "asc",
      filters,
    };
  }, [params, defaults.sort, defaults.dir, defaults.pageSize]);

  const update = useCallback(
    (patch: Partial<Pick<ListState, "q" | "page" | "pageSize" | "sort" | "dir">> & { filters?: Record<string, string | undefined> }) => {
      setParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          const set = (k: string, v: string | number | undefined) => {
            if (v === undefined || v === "" || v === null) next.delete(k);
            else next.set(k, String(v));
          };
          if (patch.q !== undefined) {
            set("q", patch.q);
            next.delete("page");
          }
          if (patch.page !== undefined) set("page", patch.page === 1 ? undefined : patch.page);
          if (patch.pageSize !== undefined) set("pageSize", patch.pageSize);
          if (patch.sort !== undefined) set("sort", patch.sort);
          if (patch.dir !== undefined) set("dir", patch.dir);
          if (patch.filters) {
            for (const [k, v] of Object.entries(patch.filters)) set(k, v);
            next.delete("page");
          }
          return next;
        },
        { replace: true },
      );
    },
    [setParams],
  );

  const toggleSort = useCallback(
    (key: string) => {
      if (state.sort === key) update({ dir: state.dir === "asc" ? "desc" : "asc" });
      else update({ sort: key, dir: "asc" });
    },
    [state.sort, state.dir, update],
  );

  return { state, update, toggleSort };
}
