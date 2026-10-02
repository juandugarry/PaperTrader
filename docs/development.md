# Cloud development notes

Use the existing `/workspace/PaperTrader` checkout. Cloud tasks are already isolated; do not create another worktree unless explicitly requested.

Rust in this workspace is installed under `/workspace/.papertrader-tools`. Activate it for each new shell:

```sh
export CARGO_HOME=/workspace/.papertrader-tools/cargo
export RUSTUP_HOME=/workspace/.papertrader-tools/rustup
export PATH="$CARGO_HOME/bin:$PATH"
cd /workspace/PaperTrader
npm ci --cache /workspace/.papertrader-tools/npm-cache
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked --no-default-features -j 4
```

Native Linux compilation is currently blocked by missing GLib/GTK/WebKitGTK development libraries. The machine's unprivileged package installer cannot write system directories, including after an escalated request. The headless Rust/SQLite suite and React build do not depend on those libraries. macOS build, IPC smoke test and restart verification must still run on a Mac before declaring the desktop package ready.

On a suitably provisioned Linux host, Tauri requires `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, a C toolchain, `pkg-config` and a graphical display. Do not bypass package/TLS verification to install these.

For a real desktop smoke test: run `npm run desktop`, create an Alex profile with $1,000 capital and $3 brokerage, verify $1,000 cash and zero positions/P&L, navigate Journal/Learn/Settings, close and reopen, and verify the same profile/cash. The Phase 2 trading smoke test is below. Never use a tester's existing database as disposable test data.

## Phase 2 smoke test

On a disposable simulation, add BEN and its full security name, buy 24 at 10.115 with 3.00 brokerage, and verify the review shows 245.76 outlay and 754.24 remaining cash. Confirm once; there must be one immutable fill. Set a manual current price of 10.60 and verify the timestamp, 254.40 position value, 8.64 unrealised P&L, and 1008.64 portfolio value. Sell 12 at 10.60 with 3.00 brokerage; verify 878.44 cash, 12 shares and 1.32 realised P&L. Close the remaining 12 at 9.95 with 3.00 brokerage; verify 994.84 cash, no open position and −5.16 realised P&L. Restart to verify persistence. Also test a rejected oversized buy and an oversell without any new history entries. These numbers are test fixtures, not seeded user data.

Phase 3's web build, Rust/SQLite tests and Chromium checks can run here. The GitHub Actions workflow builds the universal macOS app; its remote result cannot be queried from this machine because GitHub API access is denied. The user has opened Phase 3 successfully on their Mac; release 0.4.0 still needs native validation.

## Phase 3 journal smoke test

Use a disposable simulation, not a tester's existing portfolio. Before the BEN buy above, expand Entry plan & notes and enter a thesis, entry trigger, target 10.60, invalidation 9.95 and planned risk 10.00. Review and confirm once. In Journal verify an open lifecycle containing the buy's immutable price/quantity/time/brokerage plus the captured plan. Edit the thesis, save a revision and inspect earlier commentary: the original must remain visible and cash must stay 754.24.

Record the 12-share partial sell above with an exit reason and plan-adherence answer. In Journal verify the same open lifecycle with two fills, 5.82 gross realised P&L, 4.50 realised transaction costs and 1.32 net realised P&L. Complete the remaining exit and review fields; the lifecycle closes with 3.84 gross P&L, 9.00 costs, −5.16 net P&L and approximately −2.10% return. A later BUY must start a separate lifecycle. Restart to verify commentary and versions persist.

For an existing Phase 2 database, verify every legacy fill has an empty journal marked added after the fill, without changes to balances/history. In two app windows, editing an already-changed revision must be rejected with Reload latest journal. No market data or AI is used.

## Reset and notepad smoke test (0.3.1)

Use a disposable profile. In Journal → Notepad create a titled note, save it, search it, edit it and restart to verify persistence. Make unsaved changes and switch notes or pages; confirm the draft remains until explicitly discarded. Delete a note only after confirmation. Save before closing the app.

In Settings open Reset profile, cancel once and verify data remains. Open it again, type RESET and confirm. Verify onboarding returns; restart must still show onboarding. Create a fresh profile and verify only the chosen opening capital exists, with no old securities, prices, executions, journal entries or notes. An old second window must reject writes after reset; reopen it to load the fresh profile. Never perform this test against a portfolio you want to keep.

Provider research succeeded; see [provider sources and integration](market-data.md).

## Phase 4 smoke test

Use a disposable profile and your own free EODHD key, entered in Settings on the Mac (never chat or a fixture). Verify save, restart presence and remove without any system credential prompt. Add a valid ASX ticker, refresh and verify an AUD closing reference price, EODHD source, actual session date and retrieval time. Check one-month/three-month/one-year charts, date exploration and adjusted-series label. Record a manual simulated fill and verify subsequent refresh leaves its price, cash and commentary intact while updating reference valuation.

Disconnect networking and refresh: prior data must remain and the error must be visible. Verify an invalid key/unsupported ticker fails without fake quotes. Check the local request counter, one-minute cooldown, and bulk refresh when enough allowance remains. Opening or restarting the app must not fetch data automatically. Reset must return to onboarding and remove the profile credential/cache, while preserving anonymous request counts. Never reset a portfolio you want to keep.

The explicitly invoked public-demo transport check is:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --no-default-features public_demo_transport_and_history -- --ignored --nocapture
```

It uses EODHD's documented public AAPL demo only. It does not test ASX entitlement or native IPC. HTTPS uses native trusted roots; never disable certificate verification to repair networking.

## Local key-file validation (0.4.2)

Keychain and Credential Manager have been removed. The same credential code compiles in normal headless tests on all platforms. Tests cover presence-only status (without reading key contents), save/replacement/removal, Unix 0600 permissions, invalid-key validation before request reservation, profile guards, and reset deletion. Use only disposable files and dummy keys in tests. Never read or print a real user's key file.

On a desktop, save your own key once after updating from 0.4.1. Navigate Trade/Stocks/Settings and restart: there must be no OS credential prompts. Refresh explicitly to validate provider access. Remove the key and confirm cached data remains; refresh should ask for setup through an ordinary application error. Reset clears the local key file. Older OS credential entries are not accessed or migrated.

GitHub's API remains blocked in this environment, but public Actions run/job HTML is accessible for step outcomes; detailed logs require sign-in.

## Crypto and deposits (0.5.0)

Read [crypto coverage, accounting and smoke tests](crypto.md). Headless tests cover migration 007→008 without changes to stock fills, persistent crypto/deposit history, and shared-cash checks across two database connections. The public REST smoke needs api.kraken.com; the desktop live feed needs wss://ws.kraken.com/v2. Neither needs a token or account. Keep TLS verification enabled. Ordinary tests remain offline. Browser interaction tests verify frontend IPC behavior; native packaging is checked by both macOS and Windows Actions jobs.
