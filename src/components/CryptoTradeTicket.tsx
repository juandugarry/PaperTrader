import { useEffect, useState, type FormEvent } from "react";
import {
  executeCryptoTrade,
  type CryptoAsset,
  type CryptoQuote,
  type CryptoTradeInput,
  type Snapshot,
  type Side,
} from "../api";
import {
  parseDecimal,
  decimalText,
  cryptoPriceText,
  quantityText,
  cryptoNotional,
} from "../domain/crypto";
import { formatMoney, parseMoney } from "../domain/money";
export default function CryptoTradeTicket({
  asset,
  quote,
  snapshot,
  onChanged,
}: {
  asset: CryptoAsset;
  quote?: CryptoQuote;
  snapshot: Snapshot;
  onChanged: (s: Snapshot) => void;
}) {
  const [side, setSide] = useState<Side>("BUY"),
    [quantity, setQuantity] = useState(""),
    [price, setPrice] = useState(
      quote ? decimalText(quote.pricePicos, 12) : "",
    ),
    [manual, setManual] = useState(false),
    [fee, setFee] = useState("0.00"),
    [notes, setNotes] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [review, setReview] = useState<CryptoTradeInput | null>(null);
  useEffect(() => {
    if (!manual && !review && quote)
      setPrice(decimalText(quote.pricePicos, 12));
  }, [quote?.pricePicos, manual, review]);
  const wallet = snapshot.crypto?.wallet,
    held = snapshot.crypto?.positions.find((p) => p.pair === asset.pair);
  function prepare(event: FormEvent) {
    event.preventDefault();
    setError("");
    try {
      const atoms = parseDecimal(quantity, 8),
        picos = parseDecimal(price, 12);
      if (atoms > 9000000000000000000n || picos > 9000000000000000000n)
        throw Error("Amount or price exceeds the supported limit.");
      const feeMicros = parseMoney(fee);
      if (feeMicros < 0 || feeMicros % 10000)
        throw Error("Enter a non-negative fee in whole cents.");
      const notional = cryptoNotional(atoms.toString(), picos.toString());
      const cashDelta =
        side === "BUY" ? -notional - feeMicros : notional - feeMicros;
      if (!wallet) throw Error("Set up your crypto wallet first.");
      if (wallet.cashMicros + cashDelta < 0)
        throw Error("Insufficient virtual funds, including the fee.");
      if (side === "SELL" && atoms > BigInt(held?.quantityAtoms ?? "0"))
        throw Error("You cannot sell more crypto than you own.");
      setReview({
        requestId: crypto.randomUUID(),
        pair: asset.pair,
        side,
        quantityAtoms: atoms.toString(),
        pricePicos: picos.toString(),
        feeMicros,
        notes,
      });
    } catch (e) {
      setError(String(e));
    }
  }
  async function confirm() {
    if (!review) return;
    setBusy(true);
    setError("");
    try {
      onChanged(await executeCryptoTrade(review));
      setReview(null);
      setQuantity("");
      setNotes("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  if (!wallet)
    return (
      <p className="notice">
        Choose your virtual wallet funding above before trading.
      </p>
    );
  return (
    <section className="crypto-ticket">
      <h3>Simulated {asset.symbol} trade</h3>
      <p className="muted">
        Available {formatMoney(wallet.cashMicros)} · Held{" "}
        {quantityText(held?.quantityAtoms ?? "0")} {asset.symbol}. Fictional
        fills only; no exchange order is sent.
      </p>
      {review ? (
        <section className="review">
          <h4>Review your fictional {review.side.toLowerCase()}</h4>
          <p>
            {quantityText(review.quantityAtoms)} {asset.symbol} at{" "}
            {cryptoPriceText(review.pricePicos)} per coin
          </p>
          <p>
            Value{" "}
            {formatMoney(
              cryptoNotional(review.quantityAtoms, review.pricePicos),
            )}{" "}
            · Fee {formatMoney(review.feeMicros)}
          </p>
          <p>
            Cash movement{" "}
            {formatMoney(
              (review.side === "BUY" ? -1 : 1) *
                cryptoNotional(review.quantityAtoms, review.pricePicos) -
                review.feeMicros,
            )}
          </p>
          <p className="muted">
            This fill price is frozen while you review; live reference prices
            may change. The backend rechecks funds and holdings.
          </p>
          {review.notes && <p>{review.notes}</p>}
          <button disabled={busy} onClick={() => void confirm()}>
            {busy ? "Recording…" : "Confirm simulated crypto trade"}
          </button>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => setReview(null)}
          >
            Edit trade
          </button>
        </section>
      ) : (
        <form onSubmit={prepare}>
          <fieldset disabled={busy}>
            <div
              className="side-switch"
              role="group"
              aria-label="Crypto trade side"
            >
              {(["BUY", "SELL"] as Side[]).map((value) => (
                <button
                  type="button"
                  key={value}
                  className={side === value ? "selected" : ""}
                  aria-pressed={side === value}
                  onClick={() => setSide(value)}
                >
                  {value}
                </button>
              ))}
            </div>
            <div className="form-row">
              <label>
                Crypto quantity
                <input
                  value={quantity}
                  onChange={(e) => setQuantity(e.target.value)}
                  inputMode="decimal"
                  required
                  placeholder="e.g. 0.001"
                />
              </label>
              <label>
                Fill price per coin (AUD)
                <input
                  value={price}
                  onChange={(e) => {
                    setPrice(e.target.value);
                    setManual(true);
                  }}
                  inputMode="decimal"
                  required
                />
              </label>
            </div>
            <button
              type="button"
              className="secondary"
              disabled={!quote}
              onClick={() => {
                setManual(false);
                if (quote) setPrice(decimalText(quote.pricePicos, 12));
              }}
            >
              Use latest reference price
            </button>
            <label>
              Virtual trading fee (AUD)
              <input
                value={fee}
                onChange={(e) => setFee(e.target.value)}
                inputMode="decimal"
                required
              />
            </label>
            <label>
              Crypto trade notes
              <textarea
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                maxLength={10000}
                rows={3}
                placeholder="Thesis, entry plan, or exit reason…"
              />
            </label>
            <button>Review simulated crypto trade</button>
          </fieldset>
        </form>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
