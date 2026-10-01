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

Phase 2's web build, Rust/SQLite tests and Chromium checks can run here. The GitHub Actions workflow builds the universal macOS app; its remote result cannot be queried from this machine because GitHub API access is denied. Phase 1 was opened successfully on the user's Mac; the new Phase 2 native build still needs validation there.
