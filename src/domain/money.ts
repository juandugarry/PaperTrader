export const MICROS_PER_AUD = 1_000_000;
export const MAX_MONEY = 1_000_000_000 * MICROS_PER_AUD;
// Parse decimal text as integers; never multiply a binary floating-point amount.
export function parseMoney(text: string, decimals = 2): number {
  const value = text.trim();
  const match = /^(\d+)(?:\.(\d+))?$/.exec(value);
  if (!match || (match[2]?.length ?? 0) > decimals)
    throw new Error(`Enter an amount with at most ${decimals} decimal places.`);
  const micros =
    BigInt(match[1]) * 1_000_000n + BigInt((match[2] ?? "").padEnd(6, "0"));
  if (micros > BigInt(MAX_MONEY))
    throw new Error("Amounts cannot exceed A$1 billion.");
  return Number(micros);
}
export function formatMoney(micros: number): string {
  return new Intl.NumberFormat("en-AU", {
    style: "currency",
    currency: "AUD",
    currencyDisplay: "symbol",
  }).format(micros / MICROS_PER_AUD);
}
export function formatPrice(micros: number | null): string {
  if (micros === null) return "Not priced";
  return new Intl.NumberFormat("en-AU", {
    style: "currency",
    currency: "AUD",
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  }).format(micros / MICROS_PER_AUD);
}
export const moneyText = (micros: number) =>
  (micros / MICROS_PER_AUD).toFixed(6).replace(/0+$/, "").replace(/\.$/, "");
export const percent = (value: number | null, basis: number) =>
  value === null ? "Unavailable" : `${((value / basis) * 100).toFixed(2)}%`;
