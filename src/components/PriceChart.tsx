import { useState } from "react";
import { type PriceHistory } from "../api";
import { formatPrice } from "../domain/money";
export default function PriceChart({
  history,
  ticker,
}: {
  history?: PriceHistory;
  ticker: string;
}) {
  const [range, setRange] = useState(365),
    [adjusted, setAdjusted] = useState(false),
    [date, setDate] = useState<string | null>(null);
  const all = history?.prices ?? [];
  if (!all.length)
    return (
      <section className="price-chart">
        <h4>Daily price history</h4>
        <p className="muted">
          Refresh this stock to load its daily chart. Cached history remains
          available offline.
        </p>
      </section>
    );
  const end = all.at(-1)!;
  const cutoff = new Date(`${end.sessionDate}T00:00:00Z`);
  cutoff.setUTCDate(cutoff.getUTCDate() - range);
  const prices = all.filter(
    (p) => p.sessionDate >= cutoff.toISOString().slice(0, 10),
  );
  const canAdjust = prices.every((p) => p.adjustedCloseMicros !== null);
  const useAdjusted = adjusted && canAdjust;
  const values = prices.map((p) =>
    useAdjusted ? p.adjustedCloseMicros! : p.closeMicros,
  );
  const min = Math.min(...values),
    max = Math.max(...values),
    span = max - min || Math.max(max * 0.01, 1);
  const x = (i: number) =>
      40 + (prices.length === 1 ? 250 : (i / (prices.length - 1)) * 500),
    y = (v: number) => 185 - ((v - min) / span) * 145;
  const index = Math.max(
    0,
    date ? prices.findIndex((p) => p.sessionDate === date) : prices.length - 1,
  );
  const chosen = prices[index];
  return (
    <section className="price-chart">
      <div className="section-title">
        <h4>{ticker} · daily closing prices</h4>
        <div className="chart-ranges" role="group" aria-label="Chart range">
          {[
            [30, "1 month"],
            [90, "3 months"],
            [365, "1 year"],
          ].map(([n, label]) => (
            <button
              type="button"
              key={n}
              className={range === n ? "selected" : ""}
              aria-pressed={range === n}
              onClick={() => {
                setRange(Number(n));
                setDate(null);
              }}
            >
              {label}
            </button>
          ))}
        </div>
      </div>
      <label className="adjustment">
        <input
          type="checkbox"
          checked={useAdjusted}
          disabled={!canAdjust}
          onChange={(e) => setAdjusted(e.target.checked)}
        />{" "}
        Adjust for splits and dividends
      </label>
      <p className="chart-reading">
        {chosen.sessionDate} · <strong>{formatPrice(values[index])}</strong> AUD
      </p>
      <svg
        className="daily-chart"
        viewBox="0 0 580 225"
        role="img"
        aria-label={`${ticker} ${useAdjusted ? "adjusted" : "unadjusted"} daily closing prices, ${prices[0].sessionDate} to ${end.sessionDate}`}
      >
        <title>{ticker} daily closing prices in AUD</title>
        <line x1="40" x2="540" y1="185" y2="185" className="chart-axis" />
        <text x="40" y="23">
          {formatPrice(max)}
        </text>
        <text x="40" y="215">
          {prices[0].sessionDate}
        </text>
        <text x="540" y="215" textAnchor="end">
          {end.sessionDate}
        </text>
        <polyline
          points={values.map((v, i) => `${x(i)},${y(v)}`).join(" ")}
          fill="none"
          className="chart-line"
        />
        <line
          x1={x(index)}
          x2={x(index)}
          y1="35"
          y2="185"
          className="chart-crosshair"
        />
        <circle
          cx={x(index)}
          cy={y(values[index])}
          r="4"
          className="chart-point"
        />
      </svg>
      <label>
        Explore trading sessions
        <input
          type="range"
          min={0}
          max={prices.length - 1}
          value={index}
          onChange={(e) => setDate(prices[Number(e.target.value)].sessionDate)}
          aria-valuetext={`${chosen.sessionDate}: ${formatPrice(values[index])} AUD`}
        />
      </label>
      <p className="muted">
        EODHD ·{" "}
        {useAdjusted
          ? "Adjusted for splits and dividends; history may be revised."
          : "Unadjusted closes; splits can create jumps."}{" "}
        Trading sessions are spaced evenly. Latest cached session{" "}
        {end.sessionDate}; not an intraday quote.
      </p>
      <p className="muted">
        Retrieved {new Date(history!.fetchedAt).toLocaleString("en-AU")}.{" "}
        {prices.length} sessions shown.
      </p>
    </section>
  );
}
