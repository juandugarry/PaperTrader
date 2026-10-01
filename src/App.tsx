import StocksDirectory from "./components/StocksDirectory";
import MarketSettings from "./components/MarketSettings";
import Notepad from "./components/Notepad";
import ProfileReset from "./components/ProfileReset";
import { useEffect, useState, type FormEvent } from "react";
import {
  createProfile,
  desktopAvailable,
  getSnapshot,
  type Snapshot,
} from "./api";
import { formatMoney, parseMoney } from "./domain/money";
import TradingWorkspace from "./components/TradingWorkspace";
import JournalWorkspace from "./components/JournalWorkspace";

type Page = "Trade" | "Stocks" | "Journal" | "Learn" | "Settings";
const lessons = [
  {
    title: "How stock markets work",
    category: "THE BASICS",
    body: "A share represents a small ownership interest in a company. An exchange connects buyers and sellers. The ASX is Australia’s primary securities exchange. Prices change as people reassess the value and risk of owning a security.",
  },
  {
    title: "Bid, ask & the spread",
    category: "THE BASICS",
    body: "The bid is the highest price a buyer currently offers. The ask is the lowest price a seller currently accepts. The difference is the spread. A wide spread can increase trading costs, especially in less frequently traded securities.",
  },
  {
    title: "Position sizing & risk",
    category: "MANAGING RISK",
    body: "Position sizing starts with the amount you are willing to risk. Dividing that amount by the distance between entry price and planned stop gives a starting quantity, before allowing for brokerage. Stops do not guarantee an exit price: markets can gap past them.",
  },
  {
    title: "Brokerage & transaction costs",
    category: "MANAGING RISK",
    body: "Brokerage reduces your cash when you buy and reduces your proceeds when you sell. At $3 per transaction, a buy and a sell cost $6 in total. Small positions need a larger percentage gain to cover those costs.",
  },
  {
    title: "Market & limit orders",
    category: "ORDER TYPES",
    body: "A market order prioritises execution rather than price. A limit order sets the most you will pay to buy or the least you will accept to sell, but may never fill. PaperTrader records manually entered simulated fills. Pending market, limit and stop orders are not implemented yet.",
  },
];
export default function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(desktopAvailable);
  const [error, setError] = useState("");
  const [page, setPage] = useState<Page>("Trade");
  const [lesson, setLesson] = useState(0);
  const [journalArea, setJournalArea] = useState<"trades" | "notes">("trades");
  const [notepadDirty, setNotepadDirty] = useState(false);
  const [pendingNavigation, setPendingNavigation] = useState<{
    page: Page;
    area: "trades" | "notes";
  } | null>(null);
  function navigate(nextPage: Page, area = journalArea) {
    if (notepadDirty && (nextPage !== "Journal" || area !== "notes")) {
      setPendingNavigation({ page: nextPage, area });
      return;
    }
    setPendingNavigation(null);
    setPage(nextPage);
    setJournalArea(area);
  }
  async function load() {
    setLoading(true);
    setError("");
    try {
      setSnapshot(await getSnapshot());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    if (desktopAvailable) void load();
  }, []);
  if (!desktopAvailable)
    return (
      <div className="welcome">
        <Brand />
        <section className="panel onboarding">
          <span className="eyebrow">DESKTOP APP REQUIRED</span>
          <h1>Your portfolio stays on your device.</h1>
          <p>
            This browser view cannot access the local SQLite database. Launch
            PaperTrader with <code>npm run desktop</code> to create your trader
            profile.
          </p>
          <p className="muted">
            Phase 4 · No cloud account · Fictional money only
          </p>
        </section>
      </div>
    );
  if (loading)
    return (
      <div className="welcome">
        <Brand />
        <p role="status">Opening your local portfolio…</p>
      </div>
    );
  if (error && !snapshot)
    return (
      <div className="welcome">
        <Brand />
        <section className="panel">
          <h1>Unable to open your portfolio</h1>
          <p role="alert">{error}</p>
          <button onClick={() => void load()}>Try again</button>
        </section>
      </div>
    );
  if (!snapshot) return <Onboarding onCreated={setSnapshot} />;
  return (
    <div className="shell" key={snapshot.profileId}>
      <aside>
        <Brand />
        <div className="workspace-label">YOUR WORKSPACE</div>
        <nav aria-label="Main navigation">
          {(["Trade", "Stocks", "Journal", "Learn"] as Page[]).map((tab, i) => (
            <button
              key={tab}
              className={page === tab ? "nav active" : "nav"}
              aria-current={page === tab ? "page" : undefined}
              onClick={() => navigate(tab)}
            >
              <span aria-hidden="true">{["◫", "⌕", "▤", "◇"][i]}</span>
              {tab}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            className={page === "Settings" ? "nav active" : "nav"}
            onClick={() => navigate("Settings")}
          >
            ⚙ Settings
          </button>
          <div className="profile">
            <div className="avatar">
              {snapshot.displayName[0].toUpperCase()}
            </div>
            <div>
              <strong>{snapshot.displayName}</strong>
              <small>Local trader · AUD</small>
            </div>
          </div>
        </div>
      </aside>
      <main>
        <header>
          <span className="eyebrow">ASX · PAPER TRADING</span>
          <span className="badge">
            <span className="dot" /> Offline ready
          </span>
        </header>
        <div className="page-title">
          <div>
            <h1>
              {page === "Trade"
                ? "Your portfolio"
                : page === "Stocks"
                  ? "Explore ASX stocks"
                  : page === "Journal"
                    ? "Trading journal"
                    : page === "Learn"
                      ? "Build your trading knowledge"
                      : "Your local workspace"}
            </h1>
            <p>
              {page === "Trade"
                ? "A clear view of your virtual capital."
                : page === "Stocks"
                  ? "Find a company, look up its ticker, and add it to Trade."
                  : page === "Journal"
                    ? "Good decisions start with a written plan."
                    : page === "Learn"
                      ? "Understand the concepts. Develop your own process."
                      : "An independent portfolio, stored on this device."}
            </p>
          </div>
          <span className="phase">PHASE 4</span>
        </div>
        {pendingNavigation && (
          <div className="unsaved-confirm">
            <p>
              Your note has unsaved changes. Save it first, or discard them
              before leaving.
            </p>
            <button
              className="danger-outline"
              onClick={() => {
                setPage(pendingNavigation.page);
                setJournalArea(pendingNavigation.area);
                setNotepadDirty(false);
                setPendingNavigation(null);
              }}
            >
              Discard note changes and continue
            </button>
            <button
              className="secondary"
              onClick={() => setPendingNavigation(null)}
            >
              Stay in notepad
            </button>
          </div>
        )}
        {page === "Stocks" && (
          <StocksDirectory snapshot={snapshot} onChanged={setSnapshot} />
        )}
        {page === "Trade" && (
          <TradingWorkspace snapshot={snapshot} onChanged={setSnapshot} />
        )}
        {page === "Journal" && (
          <>
            <div
              className="journal-subnav"
              role="group"
              aria-label="Journal area"
            >
              <button
                aria-pressed={journalArea === "trades"}
                className={journalArea === "trades" ? "selected" : ""}
                onClick={() => navigate("Journal", "trades")}
              >
                Trading journal
              </button>
              <button
                aria-pressed={journalArea === "notes"}
                className={journalArea === "notes" ? "selected" : ""}
                onClick={() => navigate("Journal", "notes")}
              >
                Notepad
              </button>
            </div>
            {journalArea === "trades" ? (
              <JournalWorkspace snapshot={snapshot} onChanged={setSnapshot} />
            ) : (
              <Notepad
                snapshot={snapshot}
                onChanged={setSnapshot}
                onDirtyChange={setNotepadDirty}
              />
            )}
          </>
        )}
        {page === "Learn" && (
          <div className="learn-layout">
            <div className="lesson-list">
              {lessons.map((item, index) => (
                <button
                  key={item.title}
                  className={lesson === index ? "lesson selected" : "lesson"}
                  onClick={() => setLesson(index)}
                >
                  <span className="eyebrow">{item.category}</span>
                  <strong>{item.title}</strong>
                  <small>Read lesson →</small>
                </button>
              ))}
            </div>
            <article className="panel lesson-content">
              <span className="eyebrow">{lessons[lesson].category}</span>
              <h2>{lessons[lesson].title}</h2>
              <p>{lessons[lesson].body}</p>
              <div className="reflection">
                <h3>Pause & reflect</h3>
                <p>
                  How could this concept affect the way you plan a paper trade?
                </p>
              </div>
              <small className="muted">
                Educational content only. No security recommendations.
              </small>
            </article>
          </div>
        )}
        {page === "Settings" && (
          <section className="panel settings">
            <h3>Profile & simulation</h3>
            <dl>
              <dt>Trader name</dt>
              <dd>{snapshot.displayName}</dd>
              <dt>Starting virtual capital</dt>
              <dd>{formatMoney(snapshot.startingCapitalMicros)}</dd>
              <dt>Default brokerage</dt>
              <dd>
                {formatMoney(snapshot.defaultBrokerageMicros)} per transaction
              </dd>
              <dt>Currency / primary market</dt>
              <dd>AUD / ASX</dd>
              <dt>Created</dt>
              <dd>
                {new Date(snapshot.createdAt).toLocaleDateString("en-AU")}
              </dd>
              <dt>Storage</dt>
              <dd>SQLite · on this device</dd>
            </dl>
            <p className="muted">
              Profile settings are read-only. Brokerage can be overridden on
              each fill. Backup and restore will be added later.
            </p>
            <MarketSettings market={snapshot.market} />
            <ProfileReset
              onReset={() => {
                setSnapshot(null);
                setPage("Trade");
                setJournalArea("trades");
                setError("");
              }}
            />
          </section>
        )}
        <footer>
          PaperTrader <span>Fictional money. Real learning.</span>
        </footer>
      </main>
    </div>
  );
}
function Brand() {
  return (
    <div className="brand">
      <span className="brand-mark">
        <TrendIcon />
      </span>
      <span>
        PaperTrader<small>LEARN BY DOING</small>
      </span>
    </div>
  );
}
function Onboarding({
  onCreated,
}: {
  onCreated: (snapshot: Snapshot) => void;
}) {
  const [name, setName] = useState("");
  const [capital, setCapital] = useState("1000");
  const [brokerage, setBrokerage] = useState("3.00");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  async function submit(event: FormEvent) {
    event.preventDefault();
    setError("");
    setBusy(true);
    try {
      const startingCapitalMicros = parseMoney(capital);
      const defaultBrokerageMicros = parseMoney(brokerage);
      if (!name.trim() || name.trim().length > 80)
        throw new Error("Enter a trader name of 1–80 characters.");
      if (startingCapitalMicros <= 0)
        throw new Error("Starting capital must be greater than zero.");
      onCreated(
        await createProfile({
          displayName: name.trim(),
          startingCapitalMicros,
          defaultBrokerageMicros,
        }),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="welcome">
      <Brand />
      <section className="panel onboarding">
        <span className="eyebrow">WELCOME TO PAPERTRADER</span>
        <h1>
          Start with curiosity.
          <br />
          Trade with a plan.
        </h1>
        <p>
          Create your own local ASX paper-trading workspace. No account, no real
          money.
        </p>
        <form onSubmit={submit}>
          <fieldset disabled={busy}>
            <label>
              Trader name
              <input
                autoFocus
                autoComplete="nickname"
                maxLength={80}
                required
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Your display name"
              />
            </label>
            <div className="form-row">
              <label>
                Starting capital (AUD)
                <input
                  inputMode="decimal"
                  required
                  value={capital}
                  onChange={(e) => setCapital(e.target.value)}
                />
              </label>
              <label>
                Brokerage per trade (AUD)
                <input
                  inputMode="decimal"
                  required
                  value={brokerage}
                  onChange={(e) => setBrokerage(e.target.value)}
                />
              </label>
            </div>
            <div className="fixed-settings">
              <span>
                Currency <strong>AUD</strong>
              </span>
              <span>
                Market <strong>ASX</strong>
              </span>
            </div>
            {error && (
              <p role="alert" className="error">
                {error}
              </p>
            )}
            <button type="submit">
              {busy ? "Creating portfolio…" : "Create my virtual portfolio →"}
            </button>
          </fieldset>
        </form>
        <small className="muted">
          Your data stays in SQLite on this device. Record simulated trades
          using manually entered prices.
        </small>
      </section>
    </div>
  );
}

function TrendIcon() {
  return (
    <svg
      width="26"
      height="26"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M5 19 19 5M7 5h12v12" />
    </svg>
  );
}
