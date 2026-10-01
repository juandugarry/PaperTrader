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

For a real desktop smoke test: run `npm run desktop`, create an Alex profile with $1,000 capital and $3 brokerage, verify $1,000 cash and zero positions/P&L, navigate Journal/Learn/Settings, close and reopen, and verify the same profile/cash. Trading is unavailable in Phase 1. Never use a tester's existing database as disposable test data.
