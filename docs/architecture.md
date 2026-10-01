# Phase 2 architecture

## Structure

```text
src/
  App.tsx                         profile, navigation, learning, settings
  components/TradingWorkspace.tsx securities, prices, review/confirm, positions/history
  api.ts                          typed Tauri IPC boundary
  domain/money.ts                  exact decimal parsing and money/price display
  domain/trade.ts                  integer order preview (Rust is authoritative)
src-tauri/
  src/domain.rs                   profile and monetary bounds
  src/engine.rs                   pure average-cost trading/accounting engine
  src/trading.rs                  SQLite trading adapter and derived snapshots
  src/store.rs                    migrations, profile, transaction orchestration
  src/lib.rs                      commands and platform application-data directory
  migrations/001_foundation.sql   released Phase 1 schema; unchanged
  migrations/002_manual_trading.sql
```

React 19, TypeScript 5.8, Vite 6, Tauri 2 and bundled SQLite through rusqlite. No new runtime dependency is needed for trading. Test dependencies are Vitest, Testing Library, jsdom and Rust's built-in runner. Both dependency lockfiles are committed; Rust is pinned in rust-toolchain.toml.

All filesystem/database access remains in Rust. Commands create a profile, load a snapshot, add a security, update a manual price, and record an execution. React cannot submit arbitrary SQL, set a cash balance or edit a past fill. The engine has no persistence or UI dependency.

## Schema version 2

| Table       | Purpose / invariants                                                                                                                     |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| settings    | Singleton local trader, default brokerage, AUD, ASX and UTC creation time                                                                |
| portfolios  | One portfolio per installation; no mutable balance                                                                                       |
| securities  | Unique normalised ASX ticker and security name; optional manual price and UTC update timestamp                                           |
| executions  | Append-only BUY/SELL, whole quantity, exact fill price, brokerage, cent-rounded share notional, UTC recording time and unique request ID |
| cash_ledger | Append-only signed cash movements; one opening-capital entry and one linked movement per execution                                       |

Migration 002 copies the original ledger IDs, amounts, descriptions and timestamps into the extended ledger. A partial unique index retains exactly one opening-capital entry per portfolio. Database triggers reject execution/ledger UPDATE and DELETE, and verify that a new trade cash movement matches its execution. Foreign keys tie securities, executions, portfolios and cash entries together.

Migrations use SQLite user_version and transactional DDL. A newer database is refused. No migrations import or seed personal balances, securities or trades. Existing locally created Phase 1 settings/capital remain intact; this is an upgrade, not a reset.

## Exact accounting policy

One AUD = 1,000,000 integer units. Prices allow six decimal places; capital/brokerage are whole cents. Shares are positive integers, with no short selling or fractional shares. Monetary totals are bounded to A$1 billion and quantities to one billion shares to keep IPC values exactly representable. Wide i128 arithmetic detects oversized operations before conversion. JavaScript previews use BigInt; floating-point arithmetic is only used for display percentages and currency formatting.

For each fill, share notional = quantity × price, rounded to nearest cent, half up. A positive fill must round to at least one cent. A marked position may legitimately round to zero value.

- BUY cash movement = −(rounded notional + brokerage). Cost basis increases by that cash outlay.
- SELL cash movement = rounded notional − brokerage. Brokerage can exceed proceeds, but total cash must remain non-negative.
- Average entry uses remaining weighted fill prices excluding brokerage, preserving sub-cent prices. Cost basis includes rounded buy notionals and buy brokerage.
- Partial sales release cost basis in proportion to quantity, rounded half up to a micro-AUD. Remaining basis is the original basis less the released amount. A full close releases the exact remainder.
- Realised P&L = net sell proceeds − released cost basis. It retains its sign and accumulates across closed/reopened positions.
- Current position value = quantity × manually entered current price, rounded to cents. Unrealised P&L = marked value − remaining cost basis; its percentage uses the remaining cost basis.
- Capital invested is remaining open-position cost basis. Cash is the ledger sum. Portfolio value is cash + marked open positions. Total return is portfolio value − opening capital, with its percentage divided by opening capital.

The UI formats money to cents and entry/current prices to up to six decimals. Allocation P&L may internally include fractions of a cent; final closes reconcile exactly. Unrealised P&L does not estimate future sell brokerage.

If any open position lacks a current price, total valuation, total return and unrealised P&L are unavailable. Known individual positions can still display their marked values. Fills never update a reference price implicitly; prices are explicitly manual and timestamped.

Positions and P&L are replayed from executions, not stored as mutable caches. Execution history is ordered by increasing database ID, independent of equal timestamps. Snapshot reads use a consistent SQLite transaction. Trading writes use BEGIN IMMEDIATE to validate against current committed cash and holdings across app instances, and commit the execution and ledger together. Aggregate accounting limits are validated before commit.

Each reviewed fill has a request ID. Retrying an identical request returns the existing result; reusing the ID with different values is rejected. The UI disables confirmation while submitting, retains the ID after an error, and requires an explicit review before an immutable fill is saved.

## Validation and deferred work

The database-level BEN acceptance test creates a disposable $1,000 portfolio, buys 24 BEN at $10.115 with $3 brokerage, and checks exactly $754.24 cash. It then marks, partially sells and closes the position, checking cash/P&L reconciliation. Other tests cover multiple buys, cost-allocation remainder, overselling, insufficient cash, invalid/overflow input, rollback, duplicate retries, immutable fills, migration, prices and restart persistence. Test data is not installed into the application.

Phase 3 adds journal thesis, targets/invalidation, notes, linking and exit reviews. Journal currently exposes the immutable fill history. Live market-data providers, charts, pending orders, analytics and AI remain deferred. Backup/import/reset, editable profile settings, additional portfolios and signed distribution also remain outside Phase 2. Brokerage is adjustable on each fill.
