import { useEffect, useMemo, useState } from "react";
import {
  createSecurity,
  getAsxDirectory,
  getSnapshot,
  refreshAsxDirectory,
  type AsxDirectory,
  type AsxListing,
  type Snapshot,
} from "../api";

export default function StocksDirectory({
  snapshot,
  onChanged,
}: {
  snapshot: Snapshot;
  onChanged: (value: Snapshot) => void;
}) {
  const [directory, setDirectory] = useState<AsxDirectory>({
    entries: [],
    fetchedAt: null,
  });
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState("");
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  useEffect(() => {
    let active = true;
    getAsxDirectory()
      .then((value) => {
        if (active) setDirectory(value);
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
  const results = useMemo(() => {
    const words = query.toLocaleLowerCase().trim().split(/\s+/);
    return directory.entries.filter(
      (item) =>
        (!kind || item.kind === kind) &&
        words.every((word) =>
          `${item.ticker} ${item.name}`.toLocaleLowerCase().includes(word),
        ),
    );
  }, [directory, query, kind]);
  const kinds = [...new Set(directory.entries.map((item) => item.kind))].sort();
  const pages = Math.max(1, Math.ceil(results.length / 50));
  const currentPage = Math.min(page, pages - 1);
  async function refresh() {
    setBusy("refresh");
    setError("");
    setMessage("");
    try {
      setDirectory(await refreshAsxDirectory());
      setPage(0);
    } catch (e) {
      setError(String(e));
    } finally {
      try {
        const value = await getSnapshot();
        if (value) onChanged(value);
      } catch (e) {
        setError(String(e));
      }
      setBusy("");
    }
  }
  async function add(item: AsxListing) {
    setBusy(item.ticker);
    setError("");
    setMessage("");
    try {
      onChanged(
        await createSecurity({
          ticker: item.ticker,
          name: [...item.name].slice(0, 120).join(""),
        }),
      );
      setMessage(
        `${item.ticker} added. Open Trade to set or refresh its price and place a simulated trade.`,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy("");
    }
  }
  return (
    <section className="panel stocks-directory">
      <div className="directory-heading">
        <div>
          <h2>ASX company names & tickers</h2>
          <p className="muted">
            Active ASX listings supplied by EODHD, including shares and ETFs.
            Coverage depends on the provider.
          </p>
        </div>
        <button
          disabled={
            !!busy ||
            loading ||
            snapshot.market.refreshing ||
            snapshot.market.requestsToday >= snapshot.market.dailyLimit
          }
          onClick={() => void refresh()}
        >
          {busy === "refresh"
            ? "Loading directory…"
            : directory.fetchedAt
              ? "Update directory"
              : "Load ASX directory"}
        </button>
      </div>
      <p className="muted">
        Requires your EODHD key in Settings. One request loads the directory;
        searching and adding tickers use no requests. Prices can be refreshed
        separately in Trade.
      </p>
      <small className="muted">
        {snapshot.market.dailyLimit - snapshot.market.requestsToday} requests
        remaining today (UTC).{" "}
        {directory.fetchedAt &&
          `Cached on ${new Date(directory.fetchedAt).toLocaleString("en-AU", { timeZone: "Australia/Sydney" })} (Sydney). Available offline.`}
      </small>
      <div className="form-row directory-filters">
        <label>
          Search company or ticker
          <input
            type="search"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setPage(0);
            }}
            placeholder="e.g. BHP or Commonwealth"
          />
        </label>
        <label>
          Listing type
          <select
            value={kind}
            onChange={(e) => {
              setKind(e.target.value);
              setPage(0);
            }}
          >
            <option value="">All types</option>
            {kinds.map((value) => (
              <option key={value}>{value}</option>
            ))}
          </select>
        </label>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {message && <p role="status">{message}</p>}
      {loading ? (
        <p role="status">Opening cached directory…</p>
      ) : !directory.entries.length ? (
        <p>
          Load the directory once to browse ASX tickers. Your existing portfolio
          stays as it is.
        </p>
      ) : (
        <>
          <p className="muted">
            {results.length.toLocaleString()} matching listings ·{" "}
            {directory.entries.length.toLocaleString()} cached
          </p>
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Ticker</th>
                  <th>Company / security</th>
                  <th>Type</th>
                  <th>Trading workspace</th>
                </tr>
              </thead>
              <tbody>
                {results
                  .slice(currentPage * 50, (currentPage + 1) * 50)
                  .map((item) => {
                    const added = snapshot.trading.securities.some(
                      (s) => s.ticker === item.ticker,
                    );
                    const supported =
                      item.currency === "AUD" &&
                      /^[A-Z0-9]{1,10}$/.test(item.ticker);
                    return (
                      <tr key={item.ticker}>
                        <td>
                          <strong>{item.ticker}</strong>
                        </td>
                        <td>{item.name}</td>
                        <td>{item.kind}</td>
                        <td>
                          {added ? (
                            "Added"
                          ) : supported ? (
                            <button
                              className="secondary"
                              disabled={!!busy}
                              onClick={() => void add(item)}
                              aria-label={`Add ${item.ticker} to Trade`}
                            >
                              Add to Trade
                            </button>
                          ) : (
                            <span className="muted">
                              Browse only ({item.currency})
                            </span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
              </tbody>
            </table>
          </div>
          {!results.length && <p>No listings match your search.</p>}
          <div className="directory-pagination">
            <button
              className="secondary"
              disabled={currentPage === 0}
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
      )}
    </section>
  );
}
