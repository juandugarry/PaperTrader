import { useState } from "react";
import { type CryptoChart as Chart } from "../api";
import { cryptoPriceText } from "../domain/crypto";
export default function CryptoChart({
  chart,
  symbol,
}: {
  chart?: Chart;
  symbol: string;
}) {
  const [mode, setMode] = useState<"line" | "candles">("candles"),
    [range, setRange] = useState(168),
    [index, setIndex] = useState<number | null>(null);
  const rows = chart?.candles.slice(-range) ?? [];
  if (!rows.length)
    return (
      <section className="price-chart">
        <h4>{symbol} price history</h4>
        <p className="muted">
          Load hourly history to view line and candlestick charts. Cached charts
          work offline.
        </p>
      </section>
    );
  const prices = rows.map((row) => Number(row.closePicos) / 1e12),
    min = Math.min(
      ...rows.map(
        (row) =>
          Number(mode === "candles" ? row.lowPicos : row.closePicos) / 1e12,
      ),
    ),
    max = Math.max(
      ...rows.map(
        (row) =>
          Number(mode === "candles" ? row.highPicos : row.closePicos) / 1e12,
      ),
    ),
    span = max - min || Math.max(max * 0.01, 1e-12);
  const x = (i: number) =>
      40 + (rows.length === 1 ? 250 : (i / (rows.length - 1)) * 500),
    y = (v: string | number) =>
      185 -
      (((typeof v === "string" ? Number(v) / 1e12 : v) - min) / span) * 145;
  const chosenIndex = Math.min(index ?? rows.length - 1, rows.length - 1),
    chosen = rows[chosenIndex];
  return (
    <section className="price-chart">
      <h4>
        {symbol} · hourly USD{" "}
        {mode === "candles" ? "candles" : "closing prices"}
      </h4>
      <div
        className="chart-ranges"
        role="group"
        aria-label="Crypto chart style"
      >
        <button
          className={mode === "line" ? "selected" : ""}
          aria-pressed={mode === "line"}
          onClick={() => setMode("line")}
        >
          Line
        </button>
        <button
          className={mode === "candles" ? "selected" : ""}
          aria-pressed={mode === "candles"}
          onClick={() => setMode("candles")}
        >
          Candlesticks
        </button>
      </div>
      <div
        className="chart-ranges"
        role="group"
        aria-label="Crypto chart range"
      >
        {[
          [24, "1 day"],
          [168, "1 week"],
          [720, "30 days"],
        ].map(([n, label]) => (
          <button
            key={n}
            className={range === n ? "selected" : ""}
            aria-pressed={range === n}
            onClick={() => {
              setRange(Number(n));
              setIndex(null);
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <p className="chart-reading">
        {new Date(chosen.session).toLocaleString("en-AU", {
          timeZone: "Australia/Sydney",
        })}{" "}
        (Sydney) ·{" "}
        {mode === "candles" ? (
          <>
            Open {cryptoPriceText(chosen.openPicos, "USD")} · High{" "}
            {cryptoPriceText(chosen.highPicos, "USD")} · Low{" "}
            {cryptoPriceText(chosen.lowPicos, "USD")} ·{" "}
          </>
        ) : null}
        Close <strong>{cryptoPriceText(chosen.closePicos, "USD")}</strong>
      </p>
      <svg
        className="daily-chart"
        viewBox="0 0 580 225"
        role="img"
        aria-label={`${symbol} hourly USD ${mode} chart`}
      >
        <title>{symbol} hourly USD price history</title>
        <line className="chart-axis" x1="40" x2="540" y1="185" y2="185" />
        <text x="40" y="23">
          USD {max.toLocaleString("en-AU", { maximumFractionDigits: 12 })}
        </text>
        <text x="40" y="215">
          {rows[0].session.slice(0, 10)}
        </text>
        <text x="540" y="215" textAnchor="end">
          {rows.at(-1)!.session.slice(0, 10)}
        </text>
        {mode === "line" ? (
          <polyline
            className="chart-line"
            points={prices.map((p, i) => `${x(i)},${y(p)}`).join(" ")}
            fill="none"
          />
        ) : (
          rows.map((row, i) => {
            const color =
                BigInt(row.closePicos) >= BigInt(row.openPicos)
                  ? "#237b59"
                  : "#c04b49",
              width = Math.max(1, Math.min(12, 350 / rows.length));
            return (
              <g key={row.session}>
                <title>{`${row.session}: O ${cryptoPriceText(row.openPicos, "USD")} H ${cryptoPriceText(row.highPicos, "USD")} L ${cryptoPriceText(row.lowPicos, "USD")} C ${cryptoPriceText(row.closePicos, "USD")}`}</title>
                <line
                  x1={x(i)}
                  x2={x(i)}
                  y1={y(row.highPicos)}
                  y2={y(row.lowPicos)}
                  stroke={color}
                />
                <rect
                  x={x(i) - width / 2}
                  y={Math.min(y(row.openPicos), y(row.closePicos))}
                  width={width}
                  height={Math.max(
                    1,
                    Math.abs(y(row.openPicos) - y(row.closePicos)),
                  )}
                  fill={color}
                />
              </g>
            );
          })
        )}
        <line
          x1={x(chosenIndex)}
          x2={x(chosenIndex)}
          y1="35"
          y2="185"
          className="chart-crosshair"
        />
      </svg>
      <label>
        Explore crypto sessions
        <input
          type="range"
          min={0}
          max={rows.length - 1}
          value={chosenIndex}
          onChange={(e) => setIndex(Number(e.target.value))}
          aria-valuetext={`${chosen.session}: ${cryptoPriceText(chosen.closePicos, "USD")}`}
        />
      </label>
      <p className="muted">
        Kraken · actual USD market history, not historical AUD conversions.
        Wicks show high/low; green means close at or above open. Latest hourly
        candle may still be forming. {rows.length} hours shown; retrieved{" "}
        {new Date(chart!.fetchedAt).toLocaleString("en-AU", {
          timeZone: "Australia/Sydney",
        })}{" "}
        (Sydney). History updates when requested.
      </p>
    </section>
  );
}
