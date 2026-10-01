import {
  JournalFields,
  ContentSummary,
  emptyDraft,
  draftToContent,
} from "./JournalFields";
import { useState, type FormEvent } from "react";
import {
  createSecurity,
  setPrice,
  executeTrade,
  type Snapshot,
  type Security,
  type Side,
  type TradeInput,
  type Execution,
} from "../api";
import {
  formatMoney,
  formatPrice,
  moneyText,
  parseMoney,
  percent,
} from "../domain/money";
import { previewTrade } from "../domain/trade";
const message = (error: unknown) =>
  error instanceof Error ? error.message : String(error);
export function ExecutionHistory({ executions }: { executions: Execution[] }) {
  return (
    <section className="panel execution-history">
      <div className="section-title">
        <h3>Executed trades</h3>
        <span className="count">{executions.length} fills · immutable</span>
      </div>
      {!executions.length ? (
        <p className="table-empty">Your simulated fills will appear here.</p>
      ) : (
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Recorded</th>
                <th>Security</th>
                <th>Side</th>
                <th>Quantity</th>
                <th>Fill price</th>
                <th>Brokerage</th>
                <th>Cash movement</th>
              </tr>
            </thead>
            <tbody>
              {[...executions].reverse().map((e) => (
                <tr key={e.id}>
                  <td>{new Date(e.createdAt).toLocaleString("en-AU")}</td>
                  <td>
                    <strong>{e.ticker}</strong>
                  </td>
                  <td>
                    <span className={`side-tag ${e.side.toLowerCase()}`}>
                      {e.side}
                    </span>
                  </td>
                  <td>{e.quantity}</td>
                  <td>{formatPrice(e.priceMicros)}</td>
                  <td>{formatMoney(e.brokerageMicros)}</td>
                  <td>
                    {e.cashDeltaMicros < 0 ? "−" : "+"}
                    {formatMoney(Math.abs(e.cashDeltaMicros))}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
export default function TradingWorkspace({
  snapshot,
  onChanged,
}: {
  snapshot: Snapshot;
  onChanged: (s: Snapshot) => void;
}) {
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [adding, setAdding] = useState(false);
  const { trading } = snapshot;
  const selected =
    trading.securities.find((s) => s.id === selectedId) ??
    trading.securities[0];
  const filtered = trading.securities.filter((s) =>
    `${s.ticker} ${s.name}`.toLowerCase().includes(query.toLowerCase()),
  );
  const holding = trading.positions.find((p) => p.securityId === selected?.id);
  const openingPlan = (securityId: number) =>
    snapshot.journal
      .find((t) => t.securityId === securityId && t.closedAt === null)
      ?.fills[0]?.revisions.at(-1)?.content;
  const selectedPlan = selected ? openingPlan(selected.id) : undefined;

  const metrics = [
    ["Available cash", snapshot.cashMicros],
    ["Capital invested", trading.capitalInvestedMicros],
    ["Unrealised P&L", trading.unrealisedPnlMicros],
    ["Realised P&L", trading.realisedPnlMicros],
  ] as const;
  return (
    <>
      <div className="hero panel">
        <div>
          <span className="eyebrow">TOTAL PORTFOLIO VALUE</span>
          <h2>
            {trading.portfolioValueMicros === null
              ? "Not priced"
              : formatMoney(trading.portfolioValueMicros)}
          </h2>
          <span className="muted">
            Starting capital {formatMoney(snapshot.startingCapitalMicros)}
          </span>
        </div>
        <div className="return">
          <span className="eyebrow">TOTAL RETURN</span>
          <strong>
            {percent(trading.totalReturnMicros, snapshot.startingCapitalMicros)}
          </strong>
          <span>{formatMoney(trading.brokeragePaidMicros)} brokerage paid</span>
        </div>
      </div>
      <div className="metrics">
        {metrics.map(([label, value]) => (
          <section className="panel metric" key={label}>
            <span>{label}</span>
            <strong>
              {value === null ? "Not priced" : formatMoney(value)}
            </strong>
          </section>
        ))}
      </div>
      {trading.portfolioValueMicros === null && (
        <p className="notice">
          Add a manual current price for every open position to calculate
          portfolio value and unrealised P&L.
        </p>
      )}
      <section className="panel positions">
        <div className="section-title">
          <h3>Open positions</h3>
          <span className="count">{trading.positions.length} positions</span>
        </div>
        {!trading.positions.length ? (
          <div className="empty compact">
            <h3>Your next decision starts here</h3>
            <p>Add a security below, then record a manual simulated buy.</p>
          </div>
        ) : (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th>Security</th>
                  <th>Quantity</th>
                  <th>Avg. entry</th>
                  <th>Current price</th>
                  <th>Value</th>
                  <th>Target / invalidation</th>
                  <th>Unrealised P&L</th>
                </tr>
              </thead>
              <tbody>
                {trading.positions.map((p) => (
                  <tr key={p.securityId}>
                    <td>
                      <button
                        className="ticker-button"
                        onClick={() => setSelectedId(p.securityId)}
                      >
                        {p.ticker}
                      </button>
                      <small className="security-name">{p.name}</small>
                    </td>
                    <td>{p.quantity}</td>
                    <td>{formatPrice(p.averageEntryMicros)}</td>
                    <td>
                      {formatPrice(p.currentPriceMicros)}
                      <small className="security-name">
                        {p.priceUpdatedAt
                          ? new Date(p.priceUpdatedAt).toLocaleString("en-AU")
                          : "No manual price"}
                      </small>
                    </td>
                    <td>
                      {p.valueMicros === null
                        ? "—"
                        : formatMoney(p.valueMicros)}
                    </td>
                    <td>
                      {openingPlan(p.securityId)?.targetMicros == null
                        ? "Not set"
                        : formatPrice(openingPlan(p.securityId)!.targetMicros)}
                      <small className="security-name">
                        Stop{" "}
                        {openingPlan(p.securityId)?.stopMicros == null
                          ? "not set"
                          : formatPrice(openingPlan(p.securityId)!.stopMicros)}
                      </small>
                    </td>
                    <td
                      className={
                        p.unrealisedPnlMicros === null
                          ? ""
                          : p.unrealisedPnlMicros < 0
                            ? "negative"
                            : "positive"
                      }
                    >
                      {p.unrealisedPnlMicros === null
                        ? "—"
                        : formatMoney(p.unrealisedPnlMicros)}
                      <small className="security-name">
                        {percent(p.unrealisedPnlMicros, p.costBasisMicros)}
                      </small>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
      <div className="trading-layout">
        <section className="panel security-library">
          <div className="section-title">
            <h3>Securities</h3>
            <button className="secondary" onClick={() => setAdding(!adding)}>
              {adding ? "Cancel" : "Add security"}
            </button>
          </div>
          <div className="security-body">
            {adding && (
              <AddSecurity
                onSaved={(s) => {
                  onChanged(s);
                  setSelectedId(
                    s.trading.securities.find(
                      (security) =>
                        !trading.securities.some(
                          (old) => old.id === security.id,
                        ),
                    )?.id ?? null,
                  );
                  setAdding(false);
                  setQuery("");
                }}
              />
            )}
            <label>
              Search local securities
              <input
                placeholder="Ticker or company name"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            <div className="security-options">
              {filtered.map((s) => (
                <button
                  key={s.id}
                  className={
                    selected?.id === s.id
                      ? "security-option selected"
                      : "security-option"
                  }
                  onClick={() => setSelectedId(s.id)}
                >
                  <strong>{s.ticker}</strong>
                  <span>{s.name}</span>
                  <small>{formatPrice(s.currentPriceMicros)}</small>
                </button>
              ))}
            </div>
            {!filtered.length && (
              <p className="muted">
                {trading.securities.length
                  ? "No matching securities."
                  : "Your security list is empty. Add an ASX ticker to begin."}
              </p>
            )}
          </div>
        </section>
        <section className="panel security-detail">
          {selected ? (
            <>
              <div className="section-title">
                <div>
                  <h3>
                    {selected.ticker} · {selected.name}
                  </h3>
                  <small className="muted">
                    {holding
                      ? `${holding.quantity} shares held · ${formatMoney(holding.costBasisMicros)} cost basis`
                      : "No open position"}
                  </small>
                </div>
              </div>
              <div className="detail-body">
                <PriceEditor
                  key={`price-${selected.id}`}
                  security={selected}
                  onSaved={onChanged}
                />
                {selectedPlan && (
                  <details className="opening-plan">
                    <summary>Opening plan · latest commentary</summary>
                    <ContentSummary side="BUY" content={selectedPlan} />
                    <p className="muted">
                      Targets and stops are journal levels. They do not execute
                      orders.
                    </p>
                  </details>
                )}
                <TradeTicket
                  key={`trade-${selected.id}`}
                  security={selected}
                  cash={snapshot.cashMicros}
                  owned={holding?.quantity ?? 0}
                  defaultBrokerage={snapshot.defaultBrokerageMicros}
                  onSaved={onChanged}
                />
              </div>
            </>
          ) : (
            <div className="empty">
              <h3>Your local trading workspace</h3>
              <p>
                Add a security to record fills and manually update prices.
                <br />
                No live market connection is used.
              </p>
            </div>
          )}
        </section>
      </div>
      <ExecutionHistory executions={trading.executions} />
      <div className="notice">
        <span aria-hidden="true">ⓘ</span>
        <p>
          <strong>Simulated trading only.</strong> Prices and fills are manually
          entered. Nothing is sent to an exchange or broker. Average entry
          excludes brokerage; cost basis and P&L include it.
        </p>
      </div>
    </>
  );
}
function AddSecurity({ onSaved }: { onSaved: (s: Snapshot) => void }) {
  const [ticker, setTicker] = useState(""),
    [name, setName] = useState(""),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      onSaved(
        await createSecurity({
          ticker: ticker.trim().toUpperCase(),
          name: name.trim(),
        }),
      );
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <form onSubmit={submit} className="add-security">
      <fieldset disabled={busy}>
        <label>
          ASX ticker
          <input
            required
            maxLength={10}
            value={ticker}
            onChange={(e) => setTicker(e.target.value.toUpperCase())}
            placeholder="e.g. BEN"
          />
        </label>
        <label>
          Security name
          <input
            required
            maxLength={120}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Company or security name"
          />
        </label>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <button type="submit">{busy ? "Saving…" : "Save security"}</button>
      </fieldset>
    </form>
  );
}
function PriceEditor({
  security,
  onSaved,
}: {
  security: Security;
  onSaved: (s: Snapshot) => void;
}) {
  const [price, setPriceText] = useState(
      security.currentPriceMicros === null
        ? ""
        : moneyText(security.currentPriceMicros),
    ),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      const priceMicros = parseMoney(price, 6);
      if (!priceMicros) throw new Error("Price must be greater than zero.");
      onSaved(await setPrice({ securityId: security.id, priceMicros }));
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <form className="price-editor" onSubmit={submit}>
      <fieldset disabled={busy}>
        <label>
          Manual current price (AUD)
          <div className="price-input-row">
            <input
              required
              inputMode="decimal"
              value={price}
              onChange={(e) => setPriceText(e.target.value)}
              placeholder="Enter a reference price"
            />
            <button type="submit" className="secondary">
              {busy ? "Saving…" : "Update price"}
            </button>
          </div>
        </label>
        <p className="muted">
          {security.priceUpdatedAt
            ? `Manually updated ${new Date(security.priceUpdatedAt).toLocaleString("en-AU")}`
            : "No current price recorded."}{" "}
          Fills do not update this price.
        </p>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </fieldset>
    </form>
  );
}
function TradeTicket({
  security,
  cash,
  owned,
  defaultBrokerage,
  onSaved,
}: {
  security: Security;
  cash: number;
  owned: number;
  defaultBrokerage: number;
  onSaved: (s: Snapshot) => void;
}) {
  const [side, setSide] = useState<Side>("BUY"),
    [quantity, setQuantity] = useState(""),
    [price, setPriceText] = useState(""),
    [brokerage, setBrokerage] = useState(moneyText(defaultBrokerage));
  const [review, setReview] = useState<
      (TradeInput & { notionalMicros: number; cashDeltaMicros: number }) | null
    >(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [success, setSuccess] = useState("");
  const [journalDraft, setJournalDraft] = useState(emptyDraft);
  function prepare(e: FormEvent) {
    e.preventDefault();
    setError("");
    setSuccess("");
    try {
      const preview = previewTrade(side, quantity, price, brokerage);
      if (side === "SELL" && preview.quantity > owned)
        throw new Error("You cannot sell more shares than you own.");
      if (cash + preview.cashDeltaMicros < 0)
        throw new Error("Insufficient available cash, including brokerage.");
      setReview({
        ...preview,
        requestId: crypto.randomUUID(),
        securityId: security.id,
        side,
        journal: draftToContent(journalDraft, side),
      });
    } catch (e) {
      setError(message(e));
    }
  }
  async function confirm() {
    if (!review) return;
    setBusy(true);
    setError("");
    try {
      const {
        requestId,
        securityId,
        side,
        quantity,
        priceMicros,
        brokerageMicros,
        journal,
      } = review;
      onSaved(
        await executeTrade({
          requestId,
          securityId,
          side,
          quantity,
          priceMicros,
          brokerageMicros,
          journal,
        }),
      );
      setSuccess(
        `${side} recorded: ${quantity} ${security.ticker} shares. Cash and positions updated.`,
      );
      setReview(null);
      setQuantity("");
      setPriceText("");
      setJournalDraft(emptyDraft());
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="trade-ticket">
      <span className="eyebrow">MANUAL SIMULATED FILL</span>
      <p className="muted">
        Available cash {formatMoney(cash)} · {owned} shares held
      </p>
      {review ? (
        <section className="fill-review" aria-label="Review simulated fill">
          <h3>
            Review {review.side} · {security.ticker}
          </h3>
          <dl>
            <dt>Quantity / fill price</dt>
            <dd>
              {review.quantity} × {formatPrice(review.priceMicros)}
            </dd>
            <dt>Share value</dt>
            <dd>{formatMoney(review.notionalMicros)}</dd>
            <dt>Brokerage</dt>
            <dd>{formatMoney(review.brokerageMicros)}</dd>
            <dt>
              {review.side === "BUY"
                ? "Total cash outlay"
                : "Net cash proceeds"}
            </dt>
            <dd>
              {formatMoney(
                review.side === "BUY"
                  ? -review.cashDeltaMicros
                  : review.cashDeltaMicros,
              )}
            </dd>
            <dt>Cash after fill</dt>
            <dd>{formatMoney(cash + review.cashDeltaMicros)}</dd>
          </dl>
          <p className="muted">
            Recorded fills are permanent. Confirm the quantity, price and
            brokerage before continuing.
          </p>
          {review.journal && (
            <details className="review-plan">
              <summary>
                {review.side === "BUY"
                  ? "Review entry plan"
                  : "Review exit reflection"}
              </summary>
              <ContentSummary side={review.side} content={review.journal} />
            </details>
          )}
          <div className="review-actions">
            <button disabled={busy} onClick={() => void confirm()}>
              {busy ? "Recording…" : "Confirm simulated fill"}
            </button>
            <button
              disabled={busy}
              className="secondary"
              onClick={() => {
                setReview(null);
                setError("");
              }}
            >
              Back to edit
            </button>
          </div>
        </section>
      ) : (
        <form onSubmit={prepare}>
          <fieldset disabled={busy}>
            <div className="side-switch" role="group" aria-label="Trade side">
              {(["BUY", "SELL"] as Side[]).map((s) => (
                <button
                  key={s}
                  type="button"
                  aria-pressed={side === s}
                  className={side === s ? "selected" : ""}
                  onClick={() => {
                    setSide(s);
                    setError("");
                    setSuccess("");
                  }}
                >
                  {s}
                </button>
              ))}
            </div>
            <div className="form-row">
              <label>
                Quantity (whole shares)
                <input
                  required
                  inputMode="numeric"
                  value={quantity}
                  onChange={(e) => setQuantity(e.target.value)}
                />
              </label>
              <label>
                Fill price (AUD)
                <input
                  required
                  inputMode="decimal"
                  value={price}
                  onChange={(e) => setPriceText(e.target.value)}
                  placeholder="Manual execution price"
                />
              </label>
            </div>
            <label>
              Brokerage (AUD)
              <input
                required
                inputMode="decimal"
                value={brokerage}
                onChange={(e) => setBrokerage(e.target.value)}
              />
            </label>
            <details className="capture-journal">
              <summary>
                {side === "BUY" ? "Entry plan & notes" : "Exit review & notes"}
              </summary>
              <p className="muted">
                Capture your thinking now, or complete it later in Journal.
                Earlier versions are preserved.
              </p>
              <JournalFields
                side={side}
                draft={journalDraft}
                onChange={setJournalDraft}
              />
            </details>
            <button type="submit">Review simulated {side.toLowerCase()}</button>
          </fieldset>
        </form>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {success && (
        <p role="status" className="success">
          {success}
        </p>
      )}
    </div>
  );
}
