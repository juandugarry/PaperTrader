import { useState, type FormEvent } from "react";
import { resetProfile } from "../api";
export default function ProfileReset({ onReset }: { onReset: () => void }) {
  const [confirming, setConfirming] = useState(false),
    [confirmation, setConfirmation] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  async function submit(e: FormEvent) {
    e.preventDefault();
    if (confirmation !== "RESET") return;
    setBusy(true);
    setError("");
    try {
      await resetProfile(confirmation);
      onReset();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="reset-section">
      <h3>Start completely afresh</h3>
      <p>
        Reset removes your trader profile, portfolio, cash ledger, all trades
        and journal revisions, crypto wallet and fills, virtual deposits,
        securities, cached market data, notepad notes and saved API key from
        this device. You’ll return to first-launch setup. Your daily market-data
        allowance does not reset.
      </p>
      {confirming ? (
        <form onSubmit={submit} aria-label="Confirm profile reset">
          <fieldset disabled={busy}>
            <p className="error">
              This permanently clears all local progress. There is no undo.
            </p>
            <label>
              Type RESET to confirm
              <input
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <div className="review-actions">
              <button
                className="danger"
                type="submit"
                disabled={confirmation !== "RESET" || busy}
              >
                {busy ? "Resetting…" : "Permanently reset profile"}
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => {
                  setConfirming(false);
                  setConfirmation("");
                  setError("");
                }}
              >
                Keep my profile
              </button>
            </div>
          </fieldset>
        </form>
      ) : (
        <button
          type="button"
          className="danger-outline"
          onClick={() => setConfirming(true)}
        >
          Reset profile…
        </button>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
