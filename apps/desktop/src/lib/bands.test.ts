import { describe, expect, it } from "vitest";

import { bandForDays } from "./bands";

// Same boundary table as crates/core/src/expiry.rs so the two implementations cannot drift.
describe("bandForDays", () => {
  it.each([
    [-1, "EXPIRED"],
    [0, "DAYS_0_TO_30"],
    [30, "DAYS_0_TO_30"],
    [31, "DAYS_31_TO_60"],
    [60, "DAYS_31_TO_60"],
    [61, "DAYS_61_TO_90"],
    [90, "DAYS_61_TO_90"],
    [91, "DAYS_91_TO_120"],
    [120, "DAYS_91_TO_120"],
    [121, "BEYOND_120"],
    [-400, "EXPIRED"],
    [4000, "BEYOND_120"],
  ])("%i days → %s", (days, band) => {
    expect(bandForDays(days)).toBe(band);
  });
});
