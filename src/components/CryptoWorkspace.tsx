import { useEffect, useMemo, useState, type FormEvent } from "react";
import {
  getCryptoMarket,
  getSnapshot,
  refreshCryptoMarket,
  setupCryptoWallet,
  type CryptoMarket,
  type CryptoUpdate,
  type Snapshot,
} from "../api";
import { cryptoPriceText, quantityText } from "../domain/crypto";
import { formatMoney, parseMoney, percent } from "../domain/money";
import CryptoChart from "./CryptoChart";
import CryptoTradeTicket from "./CryptoTradeTicket";
import VirtualDeposit from "./VirtualDeposit";
import useCryptoLive from "./useCryptoLive";
const empty: CryptoMarket = {
  assets: [],
  quotes: [],
  charts: [],
  catalogFetchedAt: null,
  fxFetchedAt: null,
  lastError: null,
  refreshing: false,
};
export default function CryptoWorkspace({
  snapshot,
  onChanged,
}: {
  snapshot: Snapshot;
  onChanged: (s: Snapshot) => void;
}) {
  const [market, setMarket] = useState<CryptoMarket>(empty),
    [query, setQuery] = useState(""),
    [selected, setSelected] = useState(""),
    [page, setPage] = useState(0),
    [busy, setBusy] = useState(""),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(true),
    [live, setLive] = useState(true);
  const wallet = snapshot.crypto?.wallet;
  useEffect(() => {
    let active = true;
    getCryptoMarket()
      .then((value) => {
        if (active) setMarket(value);
      })
      .catch((e) => {
        if (active) setError(String(e));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);
  const selectedAsset =
    market.assets.find((a) => a.pair === selected) ??
    market.assets.find((a) => a.symbol === "BTC") ??
    market.assets[0];
  const quotes = useMemo(
    () => new Map(market.quotes.map((q) => [q.pair, q])),
    [market.quotes],
  );
  const results = useMemo(() => {
    const words = query.trim().toLowerCase().split(/\s+/);
    return market.assets.filter((a) =>
      words.every((word) =>
        `${a.name} ${a.symbol}`.toLowerCase().includes(word),
      ),
    );
  }, [market.assets, query]);
  const pages = Math.max(1, Math.ceil(results.length / 50)),
    currentPage = Math.min(page, pages - 1);
  const heldPairs = new Set(
    snapshot.crypto?.positions.map((p) => p.pair) ?? [],
  );
  const watched = [
    ...new Set([
      "AUD/USD",
      ...market.assets
        .filter((a) => heldPairs.has(a.pair) || a.pair === selectedAsset?.pair)
        .map((a) => a.wsSymbol),
    ]),
  ];
  function update(value: CryptoUpdate) {
    setMarket(value.market);
    onChanged(value.snapshot);
  }
  const liveStatus = useCryptoLive(
    live && market.assets.length > 0,
    watched,
    update,
  );
  async function refresh(action: "catalog" | "quotes" | "history") {
    setBusy(action);
    setError("");
    try {
      update(
        await refreshCryptoMarket(
          action,
          action === "history" ? (selectedAsset?.pair ?? null) : null,
        ),
      );
    } catch (e) {
      setError(String(e));
      try {
        setMarket(await getCryptoMarket());
        const value = await getSnapshot();
        if (value) onChanged(value);
      } catch {
        /* Keep already loaded data. */
      }
    } finally {
      setBusy("");
    }
  }
  return (
    <>
      <section className="panel crypto-intro">
        <span className="eyebrow">24/7 · VIRTUAL CRYPTO</span>
        <h2>Your crypto practice wallet</h2>
        <p>
          Browse every active USD spot asset in Kraken’s catalogue. No exchange
          account or API key needed. This catalogue covers Kraken markets, not
          every cryptocurrency worldwide.
        </p>
        {wallet ? (
          <>
            <div className="metrics crypto-metrics">
              {[
                [
                  wallet.mode === "shared" ? "Shared cash" : "Crypto cash",
                  wallet.cashMicros,
                ],
                [
                  wallet.mode === "shared"
                    ? "Shared stock + crypto value"
                    : "Crypto wallet value",
                  wallet.portfolioValueMicros,
                ],
                ["Virtual contributions", wallet.contributionsMicros],
                ["Crypto realised P&L", wallet.realisedPnlMicros],
              ].map(([label, value]) => (
                <section className="metric" key={String(label)}>
                  <span>{String(label)}</span>
                  <strong>
                    {value === null ? "Not priced" : formatMoney(Number(value))}
                  </strong>
                </section>
              ))}
            </div>
            <p className="muted">
              {wallet.mode === "shared"
                ? "Stock and crypto trades use one balance. Shared total value and return include both asset classes."
                : "This crypto wallet has its own virtual funds, separate from your stock portfolio."}{" "}
              Return:{" "}
              {percent(wallet.totalReturnMicros, wallet.contributionsMicros)} ·
              Crypto fees: {formatMoney(wallet.feeMicros)}.
            </p>
            <VirtualDeposit
              account={wallet.mode === "shared" ? "stocks" : "crypto"}
              onChanged={onChanged}
            />
          </>
        ) : (
          <WalletSetup onChanged={onChanged} />
        )}
      </section>
      <section className="panel crypto-directory">
        <div className="directory-heading">
          <div>
            <h2>Crypto markets</h2>
            <p className="muted">
              Market prices are exchange-specific. AUD reference prices use
              Kraken’s latest supplied AUD/USD rate; hourly charts show the
              actual USD market.
            </p>
          </div>
          <button
            disabled={!!busy || loading || market.refreshing}
            onClick={() => void refresh("catalog")}
          >
            {busy === "catalog"
              ? "Loading markets…"
              : market.assets.length
                ? "Update catalogue"
                : "Load crypto catalogue"}
          </button>
        </div>
        <div className="crypto-live-controls">
          <label className="adjustment">
            <input
              type="checkbox"
              checked={live}
              onChange={(e) => setLive(e.target.checked)}
            />
            Live price updates
          </label>
          <span role="status">
            {market.assets.length
              ? liveStatus
              : "Load the catalogue to connect the live feed"}
          </span>
          <button
            className="secondary"
            disabled={!!busy || !market.assets.length || market.refreshing}
            onClick={() => void refresh("quotes")}
          >
            {busy === "quotes" ? "Refreshing…" : "Refresh all crypto prices"}
          </button>
        </div>
        <p className="muted">
          No daily app refresh cap. Live updates cover the selected coin and
          your holdings while this tab is visible. Other catalogue prices show
          their cached timestamps. Manual refresh retrieves the whole
          catalogue’s prices. Provider rate limits still apply.
        </p>
        <label>
          Search crypto name or symbol
          <input
            type="search"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
            placeholder="Bitcoin, ETH, SOL…"
          />
        </label>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {market.lastError && !error && (
          <p className="error" role="alert">
            Last refresh: {market.lastError}
          </p>
        )}
        {loading ? (
          <p role="status">Opening cached crypto markets…</p>
        ) : market.assets.length ? (
          <>
            <p className="muted">
              {results.length} matches · {market.assets.length} supported assets
              · Catalogue cached{" "}
              {new Date(market.catalogFetchedAt!).toLocaleString("en-AU", {
                timeZone: "Australia/Sydney",
              })}{" "}
              (Sydney)
            </p>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>Coin / token</th>
                    <th>Kraken pair</th>
                    <th>AUD reference</th>
                    <th>Price time</th>
                  </tr>
                </thead>
                <tbody>
                  {results
                    .slice(currentPage * 50, (currentPage + 1) * 50)
                    .map((asset) => {
                      const quote = quotes.get(asset.pair);
                      return (
                        <tr
                          key={asset.pair}
                          className={
                            asset.pair === selectedAsset?.pair
                              ? "crypto-selected"
                              : ""
                          }
                        >
                          <td>
                            <button
                              className="ticker-button"
                              onClick={() => setSelected(asset.pair)}
                              aria-label={`Open ${asset.symbol} crypto`}
                            >
                              <strong>{asset.symbol}</strong>
                            </button>
                            <small className="security-name">
                              {asset.name}
                            </small>
                          </td>
                          <td>{asset.wsSymbol}</td>
                          <td>
                            {quote
                              ? cryptoPriceText(quote.pricePicos)
                              : "Not priced"}
                          </td>
                          <td>
                            {quote ? (
                              <>
                                <small>
                                  {quote.source === "live"
                                    ? "Live last trade"
                                    : "Retrieved"}{" "}
                                  ·{" "}
                                  {new Date(quote.fetchedAt).toLocaleString(
                                    "en-AU",
                                    { timeZone: "Australia/Sydney" },
                                  )}
                                </small>
                              </>
                            ) : (
                              "—"
                            )}
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
            {!results.length && <p>No crypto assets match your search.</p>}
            <div className="directory-pagination">
              <button
                className="secondary"
                disabled={!currentPage}
                onClick={() => setPage(currentPage - 1)}
              >
                Previous
              </button>
              <span>
                Page {currentPage + 1} of {pages}
              </span>
              <button
                className="secondary"
                disabled={currentPage + 1 >= pages}
                onClick={() => setPage(currentPage + 1)}
              >
                Next
              </button>
            </div>
          </>
        ) : (
          <p>
            Load the catalogue once to browse and cache its prices. Opening the
            app does not fetch crypto data until you open this tab and enable
            live updates.
          </p>
        )}
      </section>
      {selectedAsset && (
        <section className="panel crypto-detail">
          <div className="section-title">
            <div>
              <span className="eyebrow">KRAKEN · {selectedAsset.wsSymbol}</span>
              <h2>
                {selectedAsset.name} ({selectedAsset.symbol})
              </h2>
              <strong className="reference-price">
                {quotes.has(selectedAsset.pair)
                  ? cryptoPriceText(quotes.get(selectedAsset.pair)!.pricePicos)
                  : "Not priced"}
              </strong>
            </div>
            <button
              className="secondary"
              disabled={!!busy || market.refreshing}
              onClick={() => void refresh("history")}
            >
              {busy === "history"
                ? "Loading chart…"
                : "Load / refresh hourly chart"}
            </button>
          </div>
          {quotes.get(selectedAsset.pair) && (
            <p className="muted">
              {cryptoPriceText(
                quotes.get(selectedAsset.pair)!.usdPricePicos,
                "USD",
              )}{" "}
              on Kraken ·{" "}
              {quotes.get(selectedAsset.pair)!.source === "live"
                ? "Last trade"
                : "Retrieved"}{" "}
              {new Date(
                quotes.get(selectedAsset.pair)!.fetchedAt,
              ).toLocaleString("en-AU", { timeZone: "Australia/Sydney" })}{" "}
              (Sydney). AUD conversion retrieved{" "}
              {market.fxFetchedAt
                ? new Date(market.fxFetchedAt).toLocaleString("en-AU", {
                    timeZone: "Australia/Sydney",
                  })
                : "not available"}
              . Cached or thinly traded prices can be old even while connected.
            </p>
          )}
          <CryptoChart
            key={`chart-${selectedAsset.pair}`}
            symbol={selectedAsset.symbol}
            chart={market.charts.find((c) => c.pair === selectedAsset.pair)}
          />
          <CryptoTradeTicket
            key={`ticket-${selectedAsset.pair}`}
            asset={selectedAsset}
            quote={quotes.get(selectedAsset.pair)}
            snapshot={snapshot}
            onChanged={onChanged}
          />
        </section>
      )}
      {!!snapshot.crypto?.positions.length && (
        <section className="panel crypto-history">
          <h3>Crypto holdings</h3>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Coin</th>
                  <th>Quantity</th>
                  <th>Cost basis</th>
                  <th>Value</th>
                  <th>Unrealised P&L</th>
                </tr>
              </thead>
              <tbody>
                {snapshot.crypto.positions.map((p) => (
                  <tr key={p.pair}>
                    <td>
                      <button
                        className="ticker-button"
                        onClick={() => setSelected(p.pair)}
                      >
                        {p.symbol}
                      </button>
                    </td>
                    <td>{quantityText(p.quantityAtoms)}</td>
                    <td>{formatMoney(p.costBasisMicros)}</td>
                    <td>
                      {p.valueMicros === null
                        ? "Not priced"
                        : formatMoney(p.valueMicros)}
                    </td>
                    <td>
                      {p.unrealisedPnlMicros === null
                        ? "Not priced"
                        : formatMoney(p.unrealisedPnlMicros)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
      {!!snapshot.crypto?.executions.length && (
        <section className="panel crypto-history">
          <h3>Crypto fills & notes</h3>
          <p className="muted">
            Recorded fills and notes are immutable. Live updates change
            reference values only.
          </p>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Recorded</th>
                  <th>Coin</th>
                  <th>Side</th>
                  <th>Quantity</th>
                  <th>Fill price</th>
                  <th>Fee</th>
                  <th>Cash</th>
                  <th>Notes</th>
                </tr>
              </thead>
              <tbody>
                {[...snapshot.crypto.executions].reverse().map((e) => (
                  <tr key={e.id}>
                    <td>
                      {new Date(e.createdAt).toLocaleString("en-AU", {
                        timeZone: "Australia/Sydney",
                      })}
                    </td>
                    <td>{e.symbol}</td>
                    <td>{e.side}</td>
                    <td>{quantityText(e.quantityAtoms)}</td>
                    <td>{cryptoPriceText(e.pricePicos)}</td>
                    <td>{formatMoney(e.feeMicros)}</td>
                    <td>{formatMoney(e.cashDeltaMicros)}</td>
                    <td className="crypto-note">{e.notes || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}
    </>
  );
}
function WalletSetup({ onChanged }: { onChanged: (s: Snapshot) => void }) {
  const [mode, setMode] = useState<"separate" | "shared">("separate"),
    [funds, setFunds] = useState("1000.00"),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const [review, setReview] = useState<{
    requestId: string;
    mode: "separate" | "shared";
    startingFundsMicros: number;
  } | null>(null);
  function prepare(e: FormEvent) {
    e.preventDefault();
    setError("");
    try {
      const amount = mode === "separate" ? parseMoney(funds) : 0;
      if (mode === "separate" && (amount <= 0 || amount % 10000))
        throw Error("Choose positive fictional funds in whole cents.");
      setReview({
        requestId: crypto.randomUUID(),
        mode,
        startingFundsMicros: amount,
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
      onChanged(await setupCryptoWallet(review));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="wallet-setup">
      <h3>Choose your virtual crypto funding</h3>
      <p className="muted">
        Choose once for this profile. A separate wallet starts with fresh
        fictional funds; a shared wallet spends your existing stock cash.
        Neither connects to a real wallet.
      </p>
      {review ? (
        <>
          <p>
            {review.mode === "separate"
              ? `Create a separate wallet with ${formatMoney(review.startingFundsMicros)} of fresh virtual funds?`
              : "Use the existing stock balance for crypto trades?"}
          </p>
          <button disabled={busy} onClick={() => void confirm()}>
            {busy ? "Creating wallet…" : "Confirm virtual wallet setup"}
          </button>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => setReview(null)}
          >
            Edit wallet setup
          </button>
        </>
      ) : (
        <form onSubmit={prepare}>
          <fieldset disabled={busy}>
            <label>
              Crypto funding
              <select
                value={mode}
                onChange={(e) =>
                  setMode(e.target.value as "separate" | "shared")
                }
              >
                <option value="separate">
                  Separate virtual crypto wallet (recommended)
                </option>
                <option value="shared">Share my stock balance</option>
              </select>
            </label>
            {mode === "separate" && (
              <label>
                Starting virtual crypto funds (AUD)
                <input
                  value={funds}
                  onChange={(e) => setFunds(e.target.value)}
                  inputMode="decimal"
                  required
                />
              </label>
            )}
            <button>Review wallet setup</button>
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
