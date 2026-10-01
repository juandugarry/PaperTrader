import { useState, type FormEvent } from "react";
import {
  getSnapshot,
  saveJournal,
  type Snapshot,
  type JournalTrade,
  type JournalFill,
} from "../api";
import { formatMoney, formatPrice, percent } from "../domain/money";
import {
  JournalFields,
  ContentSummary,
  contentToDraft,
  draftToContent,
} from "./JournalFields";
export default function JournalWorkspace({
  snapshot,
  onChanged,
}: {
  snapshot: Snapshot;
  onChanged: (s: Snapshot) => void;
}) {
  const [selected, setSelected] = useState<number | null>(null),
    [query, setQuery] = useState(""),
    [filter, setFilter] = useState("all");
  const trades = [...snapshot.journal]
    .reverse()
    .filter(
      (t) =>
        `${t.ticker} ${t.name}`.toLowerCase().includes(query.toLowerCase()) &&
        (filter === "all" ||
          (filter === "closed" ? t.closedAt !== null : t.closedAt === null)),
    );
  const trade = trades.find((t) => t.id === selected) ?? trades[0];
  return (
    <>
      <div className="journal-toolbar">
        <label>
          Search journal
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Ticker or company"
          />
        </label>
        <label>
          Trade status
          <select value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="all">All trades</option>
            <option value="open">Open</option>
            <option value="closed">Closed</option>
          </select>
        </label>
      </div>
      {!trades.length ? (
        <section className="panel empty">
          <h3>
            {snapshot.journal.length
              ? "No matching trades"
              : "Your journal starts with your first fill"}
          </h3>
          <p>
            Each buy and sell is automatically linked to a journal entry.
            <br />
            Record a plan in Trade, then return here to reflect on the outcome.
          </p>
        </section>
      ) : (
        <div className="journal-layout">
          <div className="journal-trade-list">
            {trades.map((t) => {
              const missing = t.fills.filter((f) => {
                const c = f.revisions[f.revisions.length - 1].content;
                return f.execution.side === "BUY"
                  ? !c.thesis.trim()
                  : !c.exitReason.trim();
              }).length;
              return (
                <button
                  key={t.id}
                  className={
                    trade?.id === t.id
                      ? "journal-trade selected"
                      : "journal-trade"
                  }
                  onClick={() => setSelected(t.id)}
                >
                  <span className="eyebrow">
                    TRADE #{t.id} · {t.closedAt ? "CLOSED" : "OPEN"}
                  </span>
                  <strong>
                    {t.ticker} <small>{t.name}</small>
                  </strong>
                  <span>
                    {new Date(t.openedAt).toLocaleDateString("en-AU")} ·{" "}
                    {t.fills.length} fills
                  </span>
                  <span>
                    {t.closedAt ? "Net outcome" : "Realised so far"}{" "}
                    <b>{formatMoney(t.netPnlMicros)}</b>
                  </span>
                  <small>
                    {missing
                      ? `${missing} ${missing === 1 ? "entry needs" : "entries need"} reflection`
                      : "Plans & exit reasons recorded"}
                  </small>
                </button>
              );
            })}
          </div>
          {trade && <TradeDetail trade={trade} onChanged={onChanged} />}
        </div>
      )}
    </>
  );
}
function TradeDetail({
  trade,
  onChanged,
}: {
  trade: JournalTrade;
  onChanged: (s: Snapshot) => void;
}) {
  const metrics = [
    ["Gross realised P&L", trade.grossPnlMicros],
    ["Realised transaction costs", trade.realisedCostsMicros],
    ["Net realised P&L", trade.netPnlMicros],
    ["Brokerage paid (all fills)", trade.brokeragePaidMicros],
  ] as const;
  return (
    <section className="panel journal-detail">
      <div className="section-title">
        <div>
          <span className="eyebrow">
            TRADE #{trade.id} · {trade.closedAt ? "CLOSED" : "OPEN"}
          </span>
          <h2>
            {trade.ticker} · {trade.name}
          </h2>
          <p className="muted">
            Opened {new Date(trade.openedAt).toLocaleString("en-AU")}
            {trade.closedAt
              ? ` · Closed ${new Date(trade.closedAt).toLocaleString("en-AU")}`
              : ` · ${trade.quantityOpen} shares remain`}
          </p>
        </div>
      </div>
      <div className="journal-detail-body">
        <div className="journal-outcome">
          {metrics.map(([label, value]) => (
            <div key={label}>
              <span>{label}</span>
              <strong>{formatMoney(value)}</strong>
            </div>
          ))}
        </div>
        <p className="muted">
          {trade.closedAt
            ? "Closed-trade return"
            : "Return on realised cost basis"}
          :{" "}
          {trade.realisedBasisMicros
            ? percent(trade.netPnlMicros, trade.realisedBasisMicros)
            : "No sells yet"}{" "}
          · {trade.quantityBought} bought / {trade.quantitySold} sold.
          Unrealised P&L remains in Trade.
        </p>
        <h3>Your plan, decisions & reflections</h3>
        {trade.fills.map((fill) => (
          <FillEntry
            key={`${fill.execution.id}-${fill.revisions.length}`}
            fill={fill}
            onChanged={onChanged}
          />
        ))}
      </div>
    </section>
  );
}
function FillEntry({
  fill,
  onChanged,
}: {
  fill: JournalFill;
  onChanged: (s: Snapshot) => void;
}) {
  const latest = fill.revisions[fill.revisions.length - 1],
    execution = fill.execution;
  const [editing, setEditing] = useState(false),
    [draft, setDraft] = useState(() => contentToDraft(latest.content)),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [conflict, setConflict] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      onChanged(
        await saveJournal({
          executionId: execution.id,
          expectedVersion: latest.version,
          content: draftToContent(draft, execution.side),
        }),
      );
      setEditing(false);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      setError(message);
      if (message.includes("another window")) setConflict(true);
    } finally {
      setBusy(false);
    }
  }
  async function reload() {
    setBusy(true);
    setError("");
    try {
      const snapshot = await getSnapshot();
      if (!snapshot) throw new Error("Portfolio unavailable.");
      onChanged(snapshot);
      setEditing(false);
      setConflict(false);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <article className="journal-fill">
      <header>
        <div>
          <span className={`side-tag ${execution.side.toLowerCase()}`}>
            {execution.side}
          </span>
          <strong>
            {execution.quantity} {execution.ticker} @{" "}
            {formatPrice(execution.priceMicros)}
          </strong>
          <small>
            {new Date(execution.createdAt).toLocaleString("en-AU")} ·{" "}
            {formatMoney(execution.brokerageMicros)} brokerage
          </small>
        </div>
        <button
          className="secondary"
          onClick={() => {
            setEditing(!editing);
            setError("");
            setDraft(contentToDraft(latest.content));
          }}
          disabled={busy}
        >
          {editing
            ? "Cancel"
            : execution.side === "BUY"
              ? "Edit entry plan"
              : "Edit exit review"}
        </button>
      </header>
      {execution.side === "SELL" && (
        <div className="fill-result">
          <span>Gross {formatMoney(fill.grossPnlMicros!)}</span>
          <span>Costs {formatMoney(fill.transactionCostsMicros!)}</span>
          <strong>Net {formatMoney(fill.netPnlMicros!)}</strong>
          <span>
            Return {percent(fill.netPnlMicros, fill.releasedCostMicros!)}
          </span>
        </div>
      )}
      {editing ? (
        <form onSubmit={submit}>
          <fieldset disabled={busy}>
            <JournalFields
              side={execution.side}
              draft={draft}
              onChange={setDraft}
            />
            <p className="muted">
              Saving creates a new commentary revision. The original plan and
              financial fill remain unchanged.
            </p>
            <button type="submit">
              {busy ? "Saving…" : "Save journal revision"}
            </button>
          </fieldset>
        </form>
      ) : (
        <ContentSummary side={execution.side} content={latest.content} />
      )}
      <small className="muted">
        Commentary v{latest.version} ·{" "}
        {new Date(latest.createdAt).toLocaleString("en-AU")}
        {!fill.capturedWithFill ? " · Journal added after the fill" : ""}
      </small>
      {fill.revisions.length > 1 && (
        <details className="journal-versions">
          <summary>
            View earlier commentary ({fill.revisions.length - 1})
          </summary>
          {fill.revisions.slice(0, -1).map((r) => (
            <div className="earlier-revision" key={r.version}>
              <span className="eyebrow">
                VERSION {r.version}{" "}
                {r.version === 1
                  ? fill.capturedWithFill
                    ? "· ORIGINAL AT FILL"
                    : "· ADDED AFTER FILL"
                  : ""}
              </span>
              <p className="muted">
                {new Date(r.createdAt).toLocaleString("en-AU")}
              </p>
              <ContentSummary side={execution.side} content={r.content} />
            </div>
          ))}
        </details>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {conflict && (
        <button
          type="button"
          disabled={busy}
          className="secondary"
          onClick={() => void reload()}
        >
          Reload latest journal
        </button>
      )}
    </article>
  );
}
