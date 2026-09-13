import type { ReactNode } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import type { ExpenseCategory } from "@/api/types-domain";
import { EXPENSE_CATEGORY_LABEL, formatMoney } from "@/lib/format";

/**
 * Chart conventions (kept deliberately plain): one hue for magnitude, thin marks with a
 * rounded data end, recessive grid, a hover tooltip on every mark, and text in the
 * theme's text tokens — never in the series colour.
 */
const INK = "var(--muted-foreground)";
const GRID = "var(--border)";
const SERIES = "var(--primary)";

/** Fixed, colour-blind-safe categorical order (validated palette; light / dark steps). */
const CATEGORY_COLOR: Record<ExpenseCategory, { light: string; dark: string }> = {
  ELECTRICITY: { light: "#2a78d6", dark: "#3987e5" },
  WATER: { light: "#eb6834", dark: "#d95926" },
  GAS: { light: "#1baf7a", dark: "#199e70" },
  INTERNET: { light: "#eda100", dark: "#c98500" },
  MAINTENANCE: { light: "#e87ba4", dark: "#d55181" },
  CLEANING: { light: "#008300", dark: "#008300" },
  MUNICIPALITY: { light: "#4a3aa7", dark: "#9085e9" },
  OTHER: { light: "#e34948", dark: "#e66767" },
};

function isDark(): boolean {
  if (typeof document === "undefined") return false;
  const explicit = document.documentElement.dataset.theme ?? document.documentElement.classList.contains("dark");
  if (explicit === "dark" || explicit === true) return true;
  if (explicit === "light") return false;
  return typeof window !== "undefined" && window.matchMedia?.("(prefers-color-scheme: dark)").matches;
}

export function categoryColor(c: ExpenseCategory): string {
  const pair = CATEGORY_COLOR[c] ?? CATEGORY_COLOR.OTHER;
  return isDark() ? pair.dark : pair.light;
}

function TipBox({ title, rows }: { title: ReactNode; rows: [string, string][] }) {
  return (
    <div className="rounded-md border bg-popover px-2.5 py-2 text-[12px] text-popover-foreground shadow-md">
      <div className="font-medium">{title}</div>
      {rows.map(([k, v]) => (
        <div key={k} className="flex justify-between gap-4 text-muted-foreground">
          <span>{k}</span>
          <span className="tabular-nums text-foreground">{v}</span>
        </div>
      ))}
    </div>
  );
}

export function EmptyChart({ children }: { children: ReactNode }) {
  return <div className="flex h-[220px] items-center justify-center text-[13px] text-muted-foreground">{children}</div>;
}

const compact = (v: number) => (Math.abs(v) >= 1000 ? new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(v) : String(v));

/** Monthly totals as a line with an emphasised latest point. */
export function MonthlyTrend({ data }: { data: { label: string; amount: number; count: number }[] }) {
  if (data.length === 0) return <EmptyChart>No expenses in this period.</EmptyChart>;
  const last = data.length - 1;
  return (
    <ResponsiveContainer width="100%" height={220}>
      <LineChart data={data} margin={{ top: 12, right: 16, bottom: 0, left: 0 }}>
        <CartesianGrid stroke={GRID} strokeDasharray="0" vertical={false} />
        <XAxis dataKey="label" tick={{ fill: INK, fontSize: 11 }} axisLine={{ stroke: GRID }} tickLine={false} interval="preserveStartEnd" />
        <YAxis tick={{ fill: INK, fontSize: 11 }} axisLine={false} tickLine={false} width={44} tickFormatter={compact} />
        <Tooltip
          cursor={{ stroke: GRID }}
          content={({ active, payload }) =>
            active && payload?.[0] ? (
              <TipBox
                title={payload[0].payload.label}
                rows={[
                  ["Total", formatMoney(payload[0].payload.amount)],
                  ["Expenses", String(payload[0].payload.count)],
                ]}
              />
            ) : null
          }
        />
        <Line
          type="monotone"
          dataKey="amount"
          stroke={SERIES}
          strokeWidth={2}
          dot={(p) => {
            const { cx, cy, index } = p as { cx: number; cy: number; index: number };
            return <circle key={index} cx={cx} cy={cy} r={index === last ? 5 : 3.5} fill={index === last ? SERIES : "var(--card)"} stroke={SERIES} strokeWidth={2} />;
          }}
          activeDot={{ r: 6, strokeWidth: 2, stroke: "var(--card)" }}
          isAnimationActive={false}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}

export interface BarDatum {
  id: string;
  label: string;
  sublabel?: string | null;
  amount: number;
  count: number;
  color?: string;
}

/** Horizontal bars, largest first; one hue unless each bar carries its own (categories). */
export function HorizontalBars({ data, onClick, height }: { data: BarDatum[]; onClick?: (d: BarDatum) => void; height?: number }) {
  if (data.length === 0) return <EmptyChart>Nothing to show for this period.</EmptyChart>;
  const h = height ?? Math.max(120, 28 * data.length + 24);
  return (
    <ResponsiveContainer width="100%" height={h}>
      <BarChart data={data} layout="vertical" margin={{ top: 4, right: 56, bottom: 0, left: 4 }} barCategoryGap={6}>
        <CartesianGrid stroke={GRID} horizontal={false} />
        <XAxis type="number" tick={{ fill: INK, fontSize: 11 }} axisLine={false} tickLine={false} tickFormatter={compact} />
        <YAxis type="category" dataKey="label" width={110} tick={{ fill: INK, fontSize: 11.5 }} axisLine={false} tickLine={false} />
        <Tooltip
          cursor={{ fill: "var(--accent)" }}
          content={({ active, payload }) =>
            active && payload?.[0] ? (
              <TipBox
                title={`${payload[0].payload.label}${payload[0].payload.sublabel ? ` · ${payload[0].payload.sublabel}` : ""}`}
                rows={[
                  ["Total", formatMoney(payload[0].payload.amount)],
                  ["Expenses", String(payload[0].payload.count)],
                ]}
              />
            ) : null
          }
        />
        <Bar
          dataKey="amount"
          radius={[0, 4, 4, 0]}
          barSize={16}
          isAnimationActive={false}
          onClick={onClick ? (d) => onClick(d as unknown as BarDatum) : undefined}
          className={onClick ? "cursor-pointer" : undefined}
          label={{ position: "right", fill: INK, fontSize: 11, formatter: (v: unknown) => compact(Number(v ?? 0)) }}
        >
          {data.map((d) => (
            <Cell key={d.id} fill={d.color ?? SERIES} stroke="var(--card)" strokeWidth={1} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}

export function categoryLabel(c: ExpenseCategory): string {
  return EXPENSE_CATEGORY_LABEL[c] ?? c;
}
