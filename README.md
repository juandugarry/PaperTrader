# PaperTrader

A local-first desktop workspace for learning ASX paper trading with fictional money. Built with Tauri 2, React, TypeScript and SQLite. No server, cloud account, market-data provider, AI integration or real brokerage connection.

## Current release · 0.3.1

- **Settings → Reset profile:** type `RESET` to permanently clear the profile, portfolio, fills, journal, securities, prices and notes, then return to fresh setup. This cannot be undone.
- **Journal → Notepad:** create, search, edit and delete local notes. Save explicitly; unsaved drafts are protected when switching pages or notes. Save before closing the app.
- Existing profiles upgrade without losing progress. Reset occurs only after explicit confirmation.

Automatic ASX refresh and historical charts remain pending provider selection and licensing verification; see [Phase 4 research status](docs/market-data.md).

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

See [architecture and accounting rules](docs/architecture.md). Live market retrieval is Phase 4; pending orders, analytics and AI remain deferred. The app has no brokerage connection or real-money functionality.

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
