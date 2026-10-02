import { describe, it, expect } from "vitest";
import {
  parseDecimal,
  decimalText,
  cryptoNotional,
  quantityText,
} from "./crypto";
describe("fractional crypto precision", () => {
  it("keeps tiny coin prices and 8-decimal quantities exact", () => {
    expect(parseDecimal("0.00000001", 8)).toBe(1n);
    expect(parseDecimal("0.000000000001", 12)).toBe(1n);
    expect(quantityText("123456789")).toBe("1.23456789");
    expect(decimalText("100000000000000000", 12)).toBe("100000");
    expect(cryptoNotional("100000", "100000000000000000")).toBe(100000000);
  });
  it("rounds cents half up and rejects empty, exponential, negative or oversized input", () => {
    expect(cryptoNotional("100000000", "1000500000000")).toBe(1000000);
    expect(cryptoNotional("100000000", "1005000000000")).toBe(1010000);
    for (const text of ["", "0", "-1", "1e4", "0.000000001"]) {
      expect(() => parseDecimal(text, 8)).toThrow();
    }
    expect(() => cryptoNotional("1", "1")).toThrow();
  });
});
