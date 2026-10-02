import { useState, type FormEvent } from "react";
import { depositVirtualFunds, type Snapshot } from "../api";
import { formatMoney, parseMoney } from "../domain/money";
export default function VirtualDeposit({
  account,
  onChanged,
}: {
  account: "stocks" | "crypto";
  onChanged: (s: Snapshot) => void;
}) {
  const [open, setOpen] = useState(false),
    [amount, setAmount] = useState(""),
    [description, setDescription] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [review, setReview] = useState<{
    requestId: string;
    account: "stocks" | "crypto";
    amountMicros: number;
    description: string;
  } | null>(null);
  function prepare(event: FormEvent) {
    event.preventDefault();
    setError("");
    try {
      const amountMicros = parseMoney(amount);
      if (amountMicros <= 0 || amountMicros % 10000)
        throw Error("Enter a positive virtual deposit in whole cents.");
      setReview({
        requestId: crypto.randomUUID(),
        account,
        amountMicros,
        description: description.trim(),
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
      onChanged(await depositVirtualFunds(review));
      setOpen(false);
      setReview(null);
      setAmount("");
      setDescription("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="virtual-deposit">
      <button
        className="secondary"
        onClick={() => {
          setOpen(!open);
          setError("");
          setReview(null);
        }}
        disabled={busy}
      >
        Deposit virtual funds
      </button>
      {open && (
        <section className="panel deposit-dialog">
          <h3>Add fictional money</h3>
          <p>
            No bank transfer, payment or real transaction. Deposits increase
            contributions, not investment gains.
          </p>
          {review ? (
            <>
              <p>
                Add <strong>{formatMoney(review.amountMicros)}</strong> to your{" "}
                {account === "stocks" ? "stock / shared" : "separate crypto"}{" "}
                balance?
              </p>
              <button disabled={busy} onClick={() => void confirm()}>
                {busy ? "Adding funds…" : "Confirm virtual deposit"}
              </button>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setReview(null)}
              >
                Edit deposit
              </button>
            </>
          ) : (
            <form onSubmit={prepare}>
              <fieldset disabled={busy}>
                <label>
                  Virtual deposit amount (AUD)
                  <input
                    autoFocus
                    inputMode="decimal"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    required
                    placeholder="e.g. 500.00"
                  />
                </label>
                <label>
                  Deposit note (optional)
                  <input
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                    maxLength={200}
                  />
                </label>
                <button>Review virtual deposit</button>
              </fieldset>
            </form>
          )}
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
        </section>
      )}
    </div>
  );
}
