# Phase 1 architecture

## Project structure and dependencies

```text
src/
  App.tsx                onboarding, Trade / Journal / Learn / Settings
  api.ts                 typed Tauri IPC boundary
  domain/money.ts        exact decimal-text parsing and AUD display
  *.test.tsx             interaction tests with explicitly mocked IPC
src-tauri/
  src/domain.rs          pure monetary bounds and input validation
  src/store.rs           SQLite migrations, transactions, ledger queries
  src/lib.rs             desktop commands and app-data path resolution
  migrations/001_foundation.sql
  capabilities/default.json
  tauri.conf.json
```

Runtime dependencies: React 19, Tauri 2 and bundled SQLite through rusqlite. Build tools: Vite 6, TypeScript 5.8 and Tauri CLI. Tests: Vitest, Testing Library, jsdom and Rust's built-in test runner. Lockfiles pin resolved dependencies. SQLite is compiled into the app, so testers do not install a database service.

Tauri keeps filesystem access in Rust. React cannot issue arbitrary SQL or choose a database path. Two commands load the current snapshot and create the initial profile. A mutex serialises database access; SQLite also enforces constraints, foreign keys and transactional writes. WAL and a five-second busy timeout are enabled. Domain validation is independent of SQLite and UI components; there is no speculative provider framework.

## Initial schema

| Table       | Columns and invariants                                                                                                                         |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| settings    | Singleton `id=1`, trimmed display name, non-negative `default_brokerage_micros`, fixed `AUD` and `ASX`, UTC creation timestamp                 |
| portfolios  | Singleton `id=1`, name and UTC creation timestamp                                                                                              |
| cash_ledger | ID, portfolio foreign key, entry kind, signed integer monetary column (positive opening capital in v1), description and UTC creation timestamp |

Phase 1 supports one portfolio per installation. A unique `(portfolio_id, kind)` constraint permits exactly one opening-capital entry. Update/delete triggers make ledger records immutable. Settings, portfolio and ledger creation commit together or roll back together. A repeated setup attempt cannot overwrite an existing profile. No fills or orders exist in Phase 1.

Schema versioning uses SQLite `user_version`; DDL and the version update run in a transaction. New schema versions require new migrations rather than edits to released migration 001. A version newer than the app's supported schema fails startup safely.

## Monetary representation

One AUD equals 1,000,000 integer units. This represents $10.115 exactly as 10,115,000. Profile amounts accept whole cents, bounded to A$1 billion to keep all IPC integers below JavaScript's exact-integer limit. Decimal input is parsed with BigInt before conversion to an integer number; binary floating-point monetary multiplication is avoided. Display formatting alone converts micros to AUD.

Opening cash is `SUM(cash_ledger.amount_micros)`; starting capital is the sum of opening-capital entries. There is no `portfolioBalance` field. In this foundation version portfolio value equals cash, while capital invested and both P&L values are zero. No market price is fabricated.

Phase 2 must define and test the rounding policy for execution notional, partial disposals, brokerage allocation and average-cost accounting before adding trading. Checked integer operations should guard overflow. The current BEN representation test proves `1000 - (24 × 10.115 + 3) = 754.24`; it does not execute a BUY.

## Future migration boundaries

Phase 2 adds securities, manual timestamped prices, immutable executions and signed ledger entry kinds linked to executions. The v1 opening-only CHECK and uniqueness constraint must be migrated to allow multiple trade entries without relaxing opening-capital uniqueness. Position quantities/cost basis should be derived from executions, with a tested average-cost policy and no silent rewriting of past fills.

Phase 3 introduces journal plans and exit reviews linked to executions. Orders/pending fills belong to Phase 5; they are not needed to represent manual executed fills. Market-data adapters and optional AI are deferred. Future price UI must show the timestamp and source. No licensing/provider selection or integration occurs here.

Backup/import/reset, editable settings, additional portfolios, distribution signing and migration recovery UX remain outside this slice. Filesystem paths use Tauri's platform APIs so Windows support can be added without embedding macOS paths in application logic.
