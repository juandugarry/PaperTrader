# PaperTrader

A local-first desktop workspace for learning ASX paper trading with fictional money. Built for macOS and Windows with Tauri 2, React, TypeScript and SQLite. No server, PaperTrader cloud account, AI integration or real brokerage connection. Optional EODHD retrieval uses your own API key.

## Phase 4 · 0.4.2 · ASX directory, charts and Windows

- **Settings → ASX end-of-day prices:** create a free EODHD account and save its API key in a local file beside your portfolio database. No system credential prompts; the key is readable by software with access to that file. It is never returned through IPC or saved in frontend storage.
- **Trade → select a stock → Refresh price:** fetch its latest published closing price and up to one year of daily history. Refresh all updates your local security list when enough allowance remains.
- Reference prices show their source, session date and retrieval time. Daily charts offer one-month, three-month and one-year views, date exploration, and explicit split/dividend adjustment.
- Closing-price refresh changes reference valuations only. Your immutable simulated fills, cash ledger and commentary stay intact. Manual reference prices remain available.
- Cached charts/prices work offline. Per-stock errors retain previous data. A local 20-request cap resets at UTC midnight, with a one-minute cooldown between refresh batches; opening the app makes no market-data request.

One stock-history request uses one allowance slot. Reserved slots remain counted if a batch stops early, and the quota counter survives profile reset. EODHD may count usage from other applications too. End-of-day data usually publishes 2–3 hours after market close; it is not an intraday quote. This integration is for private personal use, without shared keys or redistribution of provider data. See [provider terms and implementation](docs/market-data.md).

**First use on your Mac:** download the latest successful Actions artifact, open Settings, create the free provider account, paste your key. Add an ASX ticker such as your chosen stock’s local code (no `.AU` suffix), select it in Trade and refresh. Confirm your key has ASX entitlement; no private ASX key was available for cloud validation.

## Updating from 0.4.1

Keychain and Windows Credential Manager integration has been removed at the user's request. Save your API key once in Settings after updating; the app never reads the previous system credential, including during startup, navigation or reset. Your portfolio and cached data remain intact. Keys are now saved in `eodhd-api-key-<profile-id>.txt` alongside the database, with owner-only permissions on macOS/Linux. They are plaintext and should be excluded from shared backups. Reset and Remove API key delete the local file. Previous OS credential entries are not accessed or automatically migrated.

## Stocks directory

Open **Stocks → Load ASX directory** after saving your EODHD key in Settings. One API request retrieves the provider’s active ASX listing directory, with company/security names, tickers and listing types. Search names or tickers, filter shares/ETFs/other types, and use **Add to Trade** to add an AUD security to your local workspace. It does not place a trade or retrieve a price.

The directory is cached in SQLite for offline browsing. Updating it is explicit and shares the 20-request daily allowance and one-minute cooldown with price retrieval. Failed updates retain the previous directory. Reset clears the cache. Coverage and ASX entitlement depend on EODHD; the public demo key cannot retrieve this exchange directory.

## Candlestick charts

In Trade, refresh a stock once, then switch its daily chart between **Line** and **Candlesticks**. Wicks show the actual daily high/low; bodies show open/close. Green means close at or above open, red means below open. Explore dates for the four prices. Candles show unadjusted OHLC; the line chart retains its split/dividend adjustment option. Older close-only caches still display lines until refreshed; fetching candles costs no additional request.

## Windows installer

Download **PaperTrader-Windows** from the latest successful [GitHub Actions run](https://github.com/juandugarry/PaperTrader/actions), extract the artifact ZIP and run `PaperTrader_0.4.2_x64-setup.exe` (the filename may include a language suffix). This is an unsigned Windows x64 installer; Windows may show a publisher warning. It installs the desktop app and uses WebView2. The standard installer offers shortcuts. Market keys are saved in a local file, and each Windows user has their own local SQLite portfolio. macOS users continue to download **PaperTrader-macOS**.

For development on Windows, install Node 24, the repository’s pinned Rust toolchain, Microsoft C++ Build Tools with the desktop C++ workload, and WebView2. Run `npm ci`, then `npm run desktop`. Create the installer with `npm run tauri -- build --bundles nsis` on Windows. CI tests and builds both supported platforms; no cross compilation is required on your Mac.

## Reset and notepad

- **Settings → Reset profile:** type `RESET` to permanently clear the profile, portfolio, fills, journal, securities, prices and notes, then return to fresh setup. This cannot be undone.
- **Journal → Notepad:** create, search, edit and delete local notes. Save explicitly; unsaved drafts are protected when switching pages or notes. Save before closing the app.
- Existing profiles upgrade without losing progress. Reset occurs only after explicit confirmation.

Prices and charts require your own EODHD key; manual use needs no external account.

## Phase 3 · linked trading journal

- Capture buy thesis, entry trigger, target, stop/invalidation, planned risk and notes alongside a simulated fill, or complete them later.
- Capture sell reasons, plan adherence, what went well/poorly and what you would change.
- Automatically link each fill to its journal, and group additional buys/partial sells into the same position lifecycle.
- Review gross/net realised P&L, allocated transaction costs, percentage return, fill facts and commentary together.
- Edit commentary by adding a revision; original plans, earlier commentary, executions and cash entries remain preserved.
- Filter open/closed journal trades and search by ticker/company. Rebuying after a full close starts a new lifecycle.
- View the opening plan's current target/invalidation in Trade. These are journal levels, not automatic orders.

Phase 2's manual securities, timestamped reference prices, BUY/SELL, brokerage, average-cost positions and portfolio dashboard remain available. No personal balances, trades, securities or journal commentary are imported or seeded. BEN is automated test data only.

Database migration 003 adds blank journals to existing Phase 2 fills, clearly marked as added after the fill. It does not invent an original thesis or rewrite accounting. New fills, cash movements and initial commentary commit together. Journal edits use optimistic version checks to prevent silently overwriting another window's changes.

See [architecture and accounting rules](docs/architecture.md). Phase 4 adds end-of-day retrieval; pending orders, analytics and AI remain deferred. The app has no brokerage connection or real-money functionality.

## Open from your Mac desktop

**Download a built app:** open this repository’s **Actions** tab, select the latest successful **PaperTrader checks** run, and download the **PaperTrader-macOS** artifact. Unzip the download, then unzip `PaperTrader-macOS.zip`. Move `PaperTrader.app` to Applications and create a Finder alias on your Desktop (select the app, File → Make Alias, then move the alias to Desktop). Double-click the app to launch; Node, Rust and a terminal are not needed to run a built app. The universal bundle supports Apple Silicon and Intel Macs.

These development builds are unsigned and not notarised. macOS may block the first launch. After verifying you downloaded this repository’s build, use System Settings → Privacy & Security → Open Anyway if offered. Do not disable Gatekeeper globally. Signing/notarisation remain required for normal distribution.

**Build from a checkout:** install Node.js 22+, Rust, and Apple command-line tools, clone the repository, then double-click `Install PaperTrader.command` in Finder. It builds the native app, installs it in `~/Applications`, creates a Desktop shortcut, and opens it. Later launches only need a double-click on that shortcut. The installer refuses to overwrite an existing app or shortcut. It never deletes your portfolio database.

```sh
git clone https://github.com/juandugarry/PaperTrader.git
cd PaperTrader
# If Finder does not execute the installer, run:
./Install\ PaperTrader.command
```

A successful GitHub Actions build is required before a downloadable artifact exists. This release’s macOS packaging cannot be verified on this Linux machine. The user has successfully opened the earlier Phase 3 build on their Mac.

## Develop on macOS

Install Node.js 22+ and a current stable Rust toolchain. Install Apple command-line developer tools with `xcode-select --install` if needed.

```sh
npm ci
npm run desktop
```

First launch creates `papertrader.sqlite3` in Tauri's application-data directory (normally `~/Library/Application Support/com.papertrader.desktop/` on macOS). Each installation has its own database. Restart the app to reopen the same portfolio. Startup refuses a database with a newer schema rather than silently downgrading it. Do not delete the database to resolve an error; it contains the user's portfolio.

```sh
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --locked --no-default-features
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --no-default-features -- -D warnings
npm run tauri -- build
```

The last command produces macOS app/DMG bundles when run on macOS. Code signing/notarisation and installer testing remain necessary before distribution to testers. This cloud workspace cannot validate macOS packages.

## Linux cloud development

The Rust domain and SQLite tests do not require GUI libraries (`--no-default-features`). Native Tauri development additionally requires GTK 3 and WebKitGTK 4.1 development libraries and a display. See [development notes](docs/development.md) for this workspace's validation and tool activation.

`npm run dev` serves the web UI at Vite's configured port. A standalone browser intentionally shows a desktop-required message: it does not use localStorage or pretend to persist a portfolio. React interaction tests mock IPC; Rust integration tests exercise the real SQLite implementation.

## Architecture

See [architecture and accounting design](docs/architecture.md) for the exact project structure, schema, monetary representation and future migration boundaries.
