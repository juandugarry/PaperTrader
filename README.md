# PaperTrader

A local-first desktop workspace for learning ASX paper trading with fictional money. Built with Tauri 2, React, TypeScript and SQLite. No server, cloud account, market-data provider, AI integration or real brokerage connection.

## Phase 1

- First-launch trader name, starting virtual capital and default brokerage (AUD / ASX).
- Atomic profile and portfolio creation with an immutable opening cash-ledger entry.
- Cash and starting capital derived from the ledger, not a mutable balance.
- Trade dashboard, Journal foundation, local Learn lessons and read-only Settings.
- Local SQLite persistence and transactional schema migration.

BUY/SELL execution, position calculations and journal editing are deliberately deferred to Phases 2 and 3. Dashboard investment/P&L values are zero because this version cannot execute trades. The BEN test checks exact monetary representation; it is not yet a trading-engine acceptance test.

## Open from your Mac desktop

**Download a built app:** open this repository’s **Actions** tab, select the latest successful **Phase 1 checks** run, and download the **PaperTrader-macOS** artifact. Unzip the download, then unzip `PaperTrader-macOS.zip`. Move `PaperTrader.app` to Applications and create a Finder alias on your Desktop (select the app, File → Make Alias, then move the alias to Desktop). Double-click the app to launch; Node, Rust and a terminal are not needed to run a built app. The universal bundle supports Apple Silicon and Intel Macs.

These development builds are unsigned and not notarised. macOS may block the first launch. After verifying you downloaded this repository’s build, use System Settings → Privacy & Security → Open Anyway if offered. Do not disable Gatekeeper globally. Signing/notarisation remain required for normal distribution.

**Build from a checkout:** install Node.js 22+, Rust, and Apple command-line tools, clone the repository, then double-click `Install PaperTrader.command` in Finder. It builds the native app, installs it in `~/Applications`, creates a Desktop shortcut, and opens it. Later launches only need a double-click on that shortcut. The installer refuses to overwrite an existing app or shortcut. It never deletes your portfolio database.

```sh
git clone https://github.com/juandugarry/PaperTrader.git
cd PaperTrader
# If Finder does not execute the installer, run:
./Install\ PaperTrader.command
```

A successful GitHub Actions build is required before a downloadable artifact exists. macOS packaging has not been verified on this Linux machine.

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

See [Phase 1 design](docs/architecture.md) for the exact project structure, schema, monetary representation and future migration boundaries.
