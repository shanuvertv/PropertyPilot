import { describe, expect, it } from "vitest";

import { NAV, phoneTabs } from "./nav";

// Mirrors renewal_core::roles: the phone tabs must never show a module the role cannot open.
const CAN = {
  ADMIN: () => true,
  OPERATIONS: (cap: string) => ["VIEW_UNITS", "UPDATE_UNIT_STATUS", "VIEW_TENANTS", "VIEW_BUILDINGS"].includes(cap),
  MANAGEMENT: (cap: string) =>
    [
      "VIEW_DASHBOARD",
      "VIEW_BUILDINGS",
      "VIEW_UNITS",
      "VIEW_TENANTS",
      "VIEW_CONTRACTS",
      "VIEW_RENEWALS",
      "VIEW_FOLLOW_UPS",
      "VIEW_REPORTS",
      "VIEW_AUDIT_TRAIL",
    ].includes(cap),
};

function visibleFor(role: keyof typeof CAN) {
  return NAV.filter((i) => CAN[role](i.requires));
}

describe("phoneTabs", () => {
  it("gives Admin and Leasing the four daily modules", () => {
    expect(phoneTabs(visibleFor("ADMIN")).map((t) => t.to)).toEqual(["/", "/units", "/renewals", "/follow-ups"]);
  });

  it("never exceeds four tabs and only shows what the role may see", () => {
    for (const role of Object.keys(CAN) as (keyof typeof CAN)[]) {
      const tabs = phoneTabs(visibleFor(role));
      expect(tabs.length).toBeLessThanOrEqual(4);
      for (const t of tabs) expect(CAN[role](t.requires)).toBe(true);
    }
  });

  it("falls back to master data for Operations", () => {
    expect(phoneTabs(visibleFor("OPERATIONS")).map((t) => t.to)).toEqual(["/units", "/tenants", "/buildings"]);
  });

  it("keeps every module reachable through the full list", () => {
    expect(NAV.map((i) => i.to)).toContain("/settings");
    expect(NAV.filter((i) => i.short).map((i) => i.short)).toEqual(["Buildings", "Notices", "Email"]);
  });
});
