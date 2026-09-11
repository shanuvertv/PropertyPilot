/**
 * Expiry band scale — the TypeScript twin of `renewal_core::expiry::Band`
 * (PLAN.md §3.1). The server computes bands; the UI only needs them for colour.
 */

export type Band = "EXPIRED" | "DAYS_0_TO_30" | "DAYS_31_TO_60" | "DAYS_61_TO_90" | "DAYS_91_TO_120" | "BEYOND_120";

export function bandForDays(remaining: number): Band {
  if (remaining < 0) return "EXPIRED";
  if (remaining <= 30) return "DAYS_0_TO_30";
  if (remaining <= 60) return "DAYS_31_TO_60";
  if (remaining <= 90) return "DAYS_61_TO_90";
  if (remaining <= 120) return "DAYS_91_TO_120";
  return "BEYOND_120";
}

export const BAND_LABEL: Record<Band, string> = {
  EXPIRED: "Expired",
  DAYS_0_TO_30: "0–30 days",
  DAYS_31_TO_60: "31–60 days",
  DAYS_61_TO_90: "61–90 days",
  DAYS_91_TO_120: "91–120 days",
  BEYOND_120: "More than 120 days",
};

/** Tailwind classes for the coloured dot/chip. One scale everywhere (🔴 🟠 🟡 🟢 🔵 ⚪). */
export const BAND_DOT: Record<Band, string> = {
  EXPIRED: "bg-red-600",
  DAYS_0_TO_30: "bg-orange-500",
  DAYS_31_TO_60: "bg-yellow-500",
  DAYS_61_TO_90: "bg-green-600",
  DAYS_91_TO_120: "bg-blue-600",
  BEYOND_120: "bg-neutral-400",
};
