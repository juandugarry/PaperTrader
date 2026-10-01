import { describe, it, expect } from "vitest";
import { parseMoney, formatMoney } from "./money";
describe("exact monetary input", () => {
  it("preserves cents and sub-cent security prices", () => {
    expect(parseMoney("1000")).toBe(1_000_000_000);
    expect(parseMoney("0.29")).toBe(290_000);
    expect(parseMoney("10.115", 6)).toBe(10_115_000);
    expect(
      parseMoney("1000") - 24 * parseMoney("10.115", 6) - parseMoney("3"),
    ).toBe(754_240_000);
    expect(formatMoney(754_240_000)).toBe("$754.24");
  });
  it("rejects malformed, negative, overprecise and unsafe input", () => {
    for (const value of [
      "-1",
      "1e3",
      "NaN",
      "1.001",
      "",
      "1,000",
      "1000000001",
      "Infinity",
    ])
      expect(() => parseMoney(value)).toThrow();
  });
});
