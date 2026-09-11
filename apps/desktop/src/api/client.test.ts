import { describe, expect, it } from "vitest";

import { normalizeBaseUrl } from "./client";

describe("normalizeBaseUrl", () => {
  it("adds a scheme and strips trailing slashes", () => {
    expect(normalizeBaseUrl("server.local:8787/")).toBe("http://server.local:8787");
    expect(normalizeBaseUrl("  https://renewals.example.com//  ")).toBe("https://renewals.example.com");
    expect(normalizeBaseUrl("")).toBe("");
  });
});
