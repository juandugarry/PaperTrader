import { useEffect, useState, type FormEvent } from "react";
import {
  openMarketSignup,
  marketKeyStatus,
  saveMarketKey,
  removeMarketKey,
  type MarketKeyStatus,
  type MarketSnapshot,
} from "../api";
export default function MarketSettings({ market }: { market: MarketSnapshot }) {
  const [status, setStatus] = useState<MarketKeyStatus | null>(null),
    [key, setKey] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [message, setMessage] = useState("");
  useEffect(() => {
    let active = true;
    marketKeyStatus()
      .then((s) => {
        if (active) setStatus(s);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
  async function save(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await saveMarketKey(key.trim());
      setKey("");
      setStatus({ configured: true, supported: true });
      setMessage(
        "API key saved in macOS Keychain. You can refresh prices from Trade.",
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    setBusy(true);
    setError("");
    setMessage("");
    try {
      await removeMarketKey();
      setKey("");
      setStatus({ configured: false, supported: true });
      setMessage(
        "API key removed. Previously cached prices remain on this device.",
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="market-settings">
      <h3>ASX end-of-day prices</h3>
      <p>
        Connect your own free EODHD account to refresh closing prices and up to
        one year of daily history. For private personal use. Prices usually
        publish 2–3 hours after market close.
      </p>
      <p className="muted">
        20 requests per UTC day. One stock uses one request. This app has
        reserved {market.requestsToday} of {market.dailyLimit} today; usage
        elsewhere may reduce your provider allowance. Refresh is manual so
        opening the app uses no requests.
      </p>
      <p>
        <button
          type="button"
          className="secondary"
          onClick={() => {
            void openMarketSignup().catch((e) => setError(String(e)));
          }}
        >
          Create a free EODHD account
        </button>{" "}
        Use your own account key, not their restricted demo key. Check ASX
        access in your account.
      </p>
      {status && !status.supported ? (
        <p className="notice">
          Saved API keys currently require the macOS app.
        </p>
      ) : (
        <>
          <p className="key-status">
            {status
              ? status.configured
                ? "API key saved · macOS Keychain"
                : "No API key saved"
              : "Checking Keychain…"}
          </p>
          <form onSubmit={save}>
            <fieldset disabled={busy || !status?.supported}>
              <label>
                EODHD API key
                <input
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                  placeholder={
                    status?.configured
                      ? "Paste a replacement key"
                      : "Paste your API key"
                  }
                  required
                  maxLength={160}
                />
              </label>
              <div className="review-actions">
                <button type="submit" disabled={!key.trim() || busy}>
                  {busy
                    ? "Updating…"
                    : status?.configured
                      ? "Replace API key"
                      : "Save API key"}
                </button>
                {status?.configured && (
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void remove()}
                  >
                    Remove API key
                  </button>
                )}
              </div>
            </fieldset>
          </form>
        </>
      )}
      <p className="muted">
        The key is stored in macOS Keychain, never in your portfolio database.
        Reset removes it along with your profile. Closing prices are reference
        valuations; refresh never changes your recorded fills.
      </p>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {message && <p role="status">{message}</p>}
    </section>
  );
}
