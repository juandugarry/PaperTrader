import { useEffect, useState } from "react";
import {
  getSnapshot,
  marketKeyStatus,
  refreshMarketPrices,
  type Snapshot,
  type Security,
} from "../api";
import { formatPrice } from "../domain/money";
import PriceChart from "./PriceChart";
export function PriceProvenance({ security }: { security: Security }) {
  return (
    <small className="security-name">
      {security.currentPriceMicros === null
        ? "No reference price"
        : security.priceSource === "eodhd"
          ? `EODHD close · ${security.priceAsOf}`
          : "Manual reference price"}
      {security.priceUpdatedAt && (
        <>
          <br />
          Updated {new Date(security.priceUpdatedAt).toLocaleString("en-AU")}
        </>
      )}
    </small>
  );
}
export default function MarketPrices({
  snapshot,
  security,
  onChanged,
}: {
  snapshot: Snapshot;
  security: Security;
  onChanged: (s: Snapshot) => void;
}) {
  const [configured, setConfigured] = useState<boolean | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [result, setResult] = useState("");
  useEffect(() => {
    let active = true;
    marketKeyStatus()
      .then((s) => {
        if (active) setConfigured(s.configured);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, []);
  async function refresh(ids: number[]) {
    setBusy(true);
    setError("");
    setResult("");
    try {
      const s = await refreshMarketPrices(ids);
      onChanged(s);
      const failed = s.trading.securities.filter(
        (x) => ids.includes(x.id) && x.marketError,
      );
      setResult(
        failed.length
          ? `${ids.length - failed.length} updated; ${failed.length} could not refresh. Cached prices kept for failed stocks.`
          : `Updated ${ids.length} stock${ids.length === 1 ? "" : "s"} from EODHD. See session dates below.`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function reload() {
    setError("");
    try {
      const s = await getSnapshot();
      if (!s) throw new Error("Profile unavailable. Reopen the app.");
      onChanged(s);
    } catch (e) {
      setError(String(e));
    }
  }
  const left = Math.max(
    0,
    snapshot.market.dailyLimit - snapshot.market.requestsToday,
  );
  const ids = snapshot.trading.securities.map((s) => s.id);
  return (
    <section className="market-prices">
      <div className="section-title">
        <div>
          <h4>End-of-day market data</h4>
          <p className="reference-price">
            {formatPrice(security.currentPriceMicros)} AUD
          </p>
          <PriceProvenance security={security} />
        </div>
      </div>
      <p className="muted">
        Latest published closing price, usually available 2–3 hours after market
        close. {left} local requests left today (UTC). Cached values may be
        older than today.
      </p>
      {configured === false && (
        <p className="notice">
          Add your free EODHD API key in Settings to refresh prices.
        </p>
      )}
      <div className="review-actions">
        <button
          disabled={
            busy || snapshot.market.refreshing || !configured || left < 1
          }
          onClick={() => void refresh([security.id])}
        >
          {busy ? "Refreshing…" : `Refresh ${security.ticker} price`}
        </button>
        <button
          className="secondary"
          disabled={
            busy ||
            snapshot.market.refreshing ||
            !configured ||
            ids.length > left ||
            ids.length > 20 ||
            ids.length < 2
          }
          onClick={() => void refresh(ids)}
        >
          Refresh all ({ids.length} requests)
        </button>
      </div>
      {snapshot.market.refreshing && !busy && (
        <p role="status">
          A refresh is running in another window.{" "}
          <button className="secondary" onClick={() => void reload()}>
            Reload refresh status
          </button>
        </p>
      )}
      {busy && (
        <p role="status">
          Fetching daily history. You can keep working; existing prices stay
          available.
        </p>
      )}
      {result && <p role="status">{result}</p>}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {security.marketError && (
        <p role="alert" className="error">
          Last refresh: {security.marketError}
        </p>
      )}
      <PriceChart
        key={security.id}
        ticker={security.ticker}
        history={snapshot.market.histories.find(
          (h) => h.securityId === security.id,
        )}
      />
    </section>
  );
}
