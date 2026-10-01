#!/bin/bash
# Double-click in Finder to build a native app and add a Desktop shortcut.
set -euo pipefail
trap 'result=$?; if [ "$result" -ne 0 ]; then echo; echo "Installation stopped. See the error above."; fi; echo; read -r -p "Press Return to close this window… " reply; exit "$result"' EXIT
cd "$(dirname "$0")"
if [ "$(uname -s)" != Darwin ]; then
  echo "This installer is for macOS. See README.md for cloud development."
  exit 1
fi
export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.cargo/bin:$PATH"
if ! command -v npm >/dev/null || ! command -v rustup >/dev/null; then
  echo "First install Node.js 22+ from https://nodejs.org and Rust from https://rustup.rs."
  echo "Then double-click this installer again."
  exit 1
fi
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Apple developer tools are required. Run: xcode-select --install"
  echo "Once installation finishes, double-click this installer again."
  exit 1
fi
if [ ! -d "$HOME/Desktop" ]; then
  echo "Your Desktop folder could not be found."
  exit 1
fi
mkdir -p "$HOME/Applications"
app_destination="$HOME/Applications/PaperTrader.app"
shortcut_destination="$HOME/Desktop/PaperTrader.app"
if [ -e "$app_destination" ] || [ -L "$app_destination" ] || [ -e "$shortcut_destination" ] || [ -L "$shortcut_destination" ]; then
  echo "An app or shortcut named PaperTrader already exists."
  echo "Move the existing app/shortcut aside before installing this build. Your SQLite portfolio is kept separately and will not be removed."
  exit 1
fi
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
npm ci
npm run tauri -- build --bundles app
app_source="src-tauri/target/release/bundle/macos/PaperTrader.app"
if [ ! -d "$app_source" ]; then
  echo "The build did not produce the expected app bundle."
  exit 1
fi
ditto "$app_source" "$app_destination"
ln -s "$app_destination" "$shortcut_destination"
echo "Installed. Double-click PaperTrader on your Desktop whenever you want to open it."
open "$app_destination"
