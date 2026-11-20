#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NO_BUILD=0
for arg in "$@"; do
  if [[ "$arg" == "--no-build" ]]; then
    NO_BUILD=1
  fi
done

if [[ "$NO_BUILD" -eq 0 && -f "$REPO_ROOT/Cargo.toml" ]]; then
  echo "==> Building release binaries (cargo build --release)..."
  (cd "$REPO_ROOT" && cargo build --release)
fi

if [[ -x "$REPO_ROOT/target/release/dji-mic" && -x "$REPO_ROOT/target/release/dji-mic-tray" ]]; then
  BIN_CLI="$REPO_ROOT/target/release/dji-mic"
  BIN_TRAY="$REPO_ROOT/target/release/dji-mic-tray"
elif [[ -x "$REPO_ROOT/bin/dji-mic" && -x "$REPO_ROOT/bin/dji-mic-tray" ]]; then
  BIN_CLI="$REPO_ROOT/bin/dji-mic"
  BIN_TRAY="$REPO_ROOT/bin/dji-mic-tray"
else
  echo "Error: Release binaries not found in $REPO_ROOT/target/release/ or $REPO_ROOT/bin/. Run 'cargo build --release' first." >&2
  exit 1
fi

APP_SHARE="$HOME/.local/share/dji-mic-mini-kde"
mkdir -p "$HOME/.local/bin" \
         "$APP_SHARE/qml" \
         "$APP_SHARE/assets" \
         "$HOME/.local/share/applications" \
         "$HOME/.local/share/knotifications6" \
         "$HOME/.local/share/icons/hicolor/256x256/apps" \
         "$HOME/.local/share/icons/hicolor/64x64/apps" \
         "$HOME/.config/systemd/user"

install -Dm755 "$BIN_CLI" "$HOME/.local/bin/dji-mic"
install -Dm755 "$BIN_TRAY" "$HOME/.local/bin/dji-mic-tray"

install -Dm644 "$REPO_ROOT"/qml/*.qml -t "$APP_SHARE/qml/"
if [[ -f "$REPO_ROOT/99-dji-mic.rules" ]]; then
  install -Dm644 "$REPO_ROOT/99-dji-mic.rules" "$APP_SHARE/99-dji-mic.rules"
fi
if [[ -f "$REPO_ROOT/assets/dji-mic-tray.png" ]]; then
  install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" "$APP_SHARE/assets/dji-mic-tray.png"
  install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" "$HOME/.local/share/icons/hicolor/256x256/apps/dji-mic-tray.png"
  install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" "$HOME/.local/share/icons/hicolor/64x64/apps/dji-mic-tray.png"
fi

# Install desktop entry preserving all 14 [Desktop Action] global shortcuts & notifyrc config
install -Dm644 "$REPO_ROOT/packaging/dji-mic-tray.desktop" "$HOME/.local/share/applications/dji-mic-tray.desktop"
install -Dm644 "$REPO_ROOT/packaging/dji-mic-tray.notifyrc" "$HOME/.local/share/knotifications6/dji-mic-tray.notifyrc"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
fi

install -Dm644 "$REPO_ROOT/systemd/dji-mic-tray.service" "$HOME/.config/systemd/user/dji-mic-tray.service"
systemctl --user daemon-reload
systemctl --user enable dji-mic-tray.service >/dev/null 2>&1 || true
systemctl --user restart dji-mic-tray.service

echo "[OK] Installed release binaries, modular QML UI, desktop shortcuts, notifyrc, and restarted dji-mic-tray.service."
