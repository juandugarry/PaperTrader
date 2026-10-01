import { describe, it, expect } from "vitest";
import { previewTrade } from "./trade";
describe("manual fill preview", () => {
  it("matches BEN share cost, brokerage, and cent rounding exactly", () => {
    const buy = previewTrade("BUY", "24", "10.115", "3");
    expect(buy.notionalMicros).toBe(242_760_000);
    expect(buy.cashDeltaMicros).toBe(-245_760_000);
    expect(previewTrade("BUY", "1", "10.115", "0").notionalMicros).toBe(
      10_120_000,
    );
    expect(previewTrade("SELL", "24", "10.60", "3").cashDeltaMicros).toBe(
      251_400_000,
    );
  });
  it("rejects fractional quantities, zero prices and oversized values", () => {
    for (const [q, p, b] of [
      ["0", "10", "3"],
      ["1.5", "10", "3"],
      ["1", "0", "3"],
      ["1", "10.1234567", "3"],
      ["1000000001", "10", "3"],
      ["1000000000", "1000000000", "3"],
      ["1", "10", "-1"],
      ["1", "0.000001", "0"],
    ])
      expect(() => previewTrade("BUY", q, p, b)).toThrow();
  });
});
