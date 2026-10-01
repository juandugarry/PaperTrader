import { MAX_MONEY, parseMoney } from "./money";
import type { Side } from "../api";
export function previewTrade(
  side: Side,
  quantityText: string,
  priceText: string,
  brokerageText: string,
) {
  if (!/^[1-9]\d*$/.test(quantityText.trim()))
    throw new Error("Enter a positive whole-share quantity.");
  const quantity = Number(quantityText);
  if (!Number.isSafeInteger(quantity) || quantity > 1_000_000_000)
    throw new Error("Quantity cannot exceed 1,000,000,000 shares.");
  const priceMicros = parseMoney(priceText, 6);
  if (priceMicros <= 0) throw new Error("Price must be greater than zero.");
  const brokerageMicros = parseMoney(brokerageText);
  const notional =
    ((BigInt(quantity) * BigInt(priceMicros) + 5_000n) / 10_000n) * 10_000n;
  if (notional < 10_000n || notional > BigInt(MAX_MONEY))
    throw new Error(
      "Share value must be at least one cent and at most A$1 billion.",
    );
  const delta =
    side === "BUY"
      ? -notional - BigInt(brokerageMicros)
      : notional - BigInt(brokerageMicros);
  if (delta < -BigInt(MAX_MONEY) || delta > BigInt(MAX_MONEY))
    throw new Error("Cash movement exceeds the supported limit.");
  return {
    quantity,
    priceMicros,
    brokerageMicros,
    notionalMicros: Number(notional),
    cashDeltaMicros: Number(delta),
  };
}
