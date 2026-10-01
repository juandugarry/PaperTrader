# Phase 3 architecture

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
  src/journal.rs                  versioned commentary and derived trade lifecycles
  src/store.rs                    migrations, profile, transaction orchestration
  src/lib.rs                      commands and platform application-data directory
  migrations/001_foundation.sql   released Phase 1 schema; unchanged
  migrations/002_manual_trading.sql
  migrations/003_trading_journal.sql
```

React 19, TypeScript 5.8, Vite 6, Tauri 2 and bundled SQLite through rusqlite. No new runtime dependency is needed for trading. Test dependencies are Vitest, Testing Library, jsdom and Rust's built-in runner. Both dependency lockfiles are committed; Rust is pinned in rust-toolchain.toml.

All filesystem/database access remains in Rust. Commands create a profile, load a snapshot, add a security, update a manual price, record an execution with optional commentary, and save a journal revision. React cannot submit arbitrary SQL, set a cash balance or edit a past fill. The engine has no persistence or UI dependency.

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

Phase 4 will investigate licensed ASX data retrieval before selecting a provider. Pending orders, charts, analytics, AI, backup/import/reset and signed distribution remain deferred. Brokerage is adjustable on each fill.

## Schema version 3 and journal behaviour

`journal_entries` has one immutable link per execution, a creation time, and a flag indicating whether it was created with the fill. `journal_revisions` stores append-only full snapshots of commentary with an execution link, monotonically increasing version and UTC save time. The first revision contains the commentary supplied with a new fill (possibly blank); later edits never overwrite it. UPDATE/DELETE triggers protect journal links and revisions.

Entry fields: thesis, entry trigger, optional target and invalidation prices, optional planned AUD risk, and notes. Exit fields: exit reason, optional yes/no plan adherence, what went well, what went poorly, what would change, and notes. Each text field is capped at 4,000 characters. Prices use micro-AUD precision; planned risk accepts whole cents. Fields remain optional so a user can complete the journal after a fill. The list prompts for a missing thesis or exit reason without claiming this is full review completeness.

Migration 003 backfills one blank revision per existing execution and marks it as added after the fill. Original execution timestamps and accounting stay unchanged. No historical rationale is inferred. Migrations 001/002 remain unchanged.

New fill, ledger entry, journal link and first revision commit in one IMMEDIATE transaction. A failed journal write rolls back the financial writes. Retries compare submitted commentary with revision 1, so a later edit does not cause the same fill to execute again. Changed payloads reusing a fill request ID are rejected.

Commentary saves include the displayed version. A stale version is rejected with a reload action instead of overwriting newer work. Identical content at the current version is a no-op. A retry after an ambiguous save can safely reload to inspect the result. Edits do not accept execution amounts, quantities or timestamps.

## Position lifecycles and journal P&L

Lifecycles are derived by replaying immutable executions in ID order, independently per security. A BUY from zero shares starts a lifecycle whose stable ID is that execution's ID. Additional buys and partial sells join that lifecycle. A sale to zero shares closes it; a later buy starts a new one. Fills interleaved between securities remain in their own lifecycles. Every fill has its own editable plan/review, allowing additional buys and staged exits to retain distinct reasoning.

The journal reuses the accounting engine twice: with actual brokerage for net results, and with zero brokerage for gross results on the same cent-rounded notionals. Realised transaction costs equal gross minus net; on partial exits these include allocated entry brokerage plus that exit's brokerage. Total brokerage paid also includes fees attributable to still-open shares, and is labelled separately. A lifecycle's net realised P&L excludes unrealised movements. Each exit has gross/net P&L and percentage return over the released fee-inclusive cost basis. Aggregate realised return uses cumulative released cost basis; after a full close this equals total buy cash outlay.

Dates, prices, quantity, brokerage and outcomes come from executions and are never commentary fields. The original opening plan is retained in the first BUY's revision history; the Trade position view displays its latest commentary target/invalidation and offers its full latest plan. Targets and stops do not submit orders or change fill logic. Manual reference prices and their timestamps remain independent.

UI interaction tests mock IPC explicitly. Rust tests exercise SQLite linking, revisions, optimistic conflicts, rollback, Phase 2 migration, partial/full exit costs, additional buys/rebuys, original-plan retention and restart persistence. macOS-native build validation still belongs to GitHub Actions and a Mac; Linux headless tests do not establish desktop readiness.
