export function parseDecimal(text: string, places: number): bigint {
  const value = text.trim();
  const match = /^(\d+)(?:\.(\d+))?$/.exec(value);
  if (!match || (match[2]?.length ?? 0) > places)
    throw Error(`Enter a positive number with up to ${places} decimal places.`);
  const result =
    BigInt(match[1]) * 10n ** BigInt(places) +
    BigInt((match[2] ?? "").padEnd(places, "0") || "0");
  if (result <= 0n) throw Error("The amount must be greater than zero.");
  return result;
}
export function decimalText(value: string, places: number): string {
  const n = BigInt(value),
    scale = 10n ** BigInt(places);
  return (
    `${n / scale}.${(n % scale).toString().padStart(places, "0")}`.replace(
      /\.?0+$/,
      "",
    ) || "0"
  );
}
export const quantityText = (value: string) => decimalText(value, 8);
export function cryptoPriceText(value: string, currency = "AUD"): string {
  const price = Number(value) / 1e12;
  const maximumFractionDigits =
    price >= 1 ? 2 : price >= 0.01 ? 4 : price >= 0.0001 ? 6 : 12;
  return `${currency} ${new Intl.NumberFormat("en-AU", { maximumFractionDigits, minimumFractionDigits: 2 }).format(price)}`;
}
export function cryptoNotional(atoms: string, price: string): number {
  const value =
    ((BigInt(atoms) * BigInt(price) + 500_000_000_000_000_000n) /
      1_000_000_000_000_000_000n) *
    10000n;
  if (value < 10000n || value > 1000000000000000n)
    throw Error(
      "Trade value must be at least one cent and within the A$1 billion limit.",
    );
  return Number(value);
}
