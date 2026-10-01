# PaperTrader

A local-first desktop workspace for learning ASX paper trading with fictional money. Built with Tauri 2, React, TypeScript and SQLite. No server, cloud account, market-data provider, AI integration or real brokerage connection.

## Phase 2 · manual paper trading

- Fresh local trader profile, virtual starting capital and default brokerage (AUD / ASX).
- Create/search local ASX securities and manually update reference prices with timestamps.
- Review and record simulated BUY/SELL fills with per-fill brokerage.
- Cash ledger, whole-share positions, average entry, cost basis, realised/unrealised P&L and total return.
- Immutable execution history in Trade and Journal; SQLite persistence and versioned migrations.

No securities, trades or personal balances are imported or seeded. The BEN scenario is automated test data only. An existing Phase 1 database migrates in place without deleting its local profile or opening-capital entry. No real orders, live market data, cloud account or AI are involved.

Buy brokerage is included in average cost basis; sell brokerage reduces proceeds. Execution totals round to cents (half up), while fill prices retain up to six decimal places. A partial sale allocates cost proportionally; a full close consumes the exact remaining basis. Unpriced positions show valuation as unavailable. See [accounting rules](docs/architecture.md).

Trading thesis, targets/stops and exit reviews are Phase 3. Pending market/limit/stop orders remain deferred.

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

A successful GitHub Actions build is required before a downloadable artifact exists. Phase 2 macOS packaging has not been verified on this Linux machine. The earlier Phase 1 build was opened successfully on the user’s Mac.

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
