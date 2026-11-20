#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.russellmilliner.DjiMicMiniControl"
VERSION="$(grep -E '^version\s*=' "$REPO_ROOT/Cargo.toml" | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')"
ARCH="$(uname -m)"
WORK_DIR="${FLATPAK_WORK_DIR:-$REPO_ROOT/dist/flatpak-work}"
OUT_DIR="${FLATPAK_OUT_DIR:-$REPO_ROOT/dist}"
BUNDLE_NAME="dji-mic-mini-kde-v${VERSION}-${ARCH}.flatpak"
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"

echo "==> Prefetching Rust crate metadata for KDE DJI Mic Mini Control v${VERSION}..."
cargo metadata --format-version 1 --manifest-path "$REPO_ROOT/Cargo.toml" > /dev/null

echo "==> Building KDE DJI Mic Mini Control v${VERSION} Flatpak inside org.kde.Sdk//6.11 (${ARCH})..."
rm -rf "$WORK_DIR/build-dir" "$WORK_DIR/repo"
mkdir -p "$WORK_DIR" "$OUT_DIR"

flatpak build-init \
  --sdk-extension=org.freedesktop.Sdk.Extension.rust-stable \
  "$WORK_DIR/build-dir" \
  "$APP_ID" \
  org.kde.Sdk \
  org.kde.Platform \
  6.11

flatpak build \
  --share=network \
  --filesystem="$REPO_ROOT":ro \
  --filesystem="$CARGO_HOME_DIR":ro \
  --filesystem="$WORK_DIR":rw \
  --env=PATH=/usr/lib/sdk/rust-stable/bin:/usr/bin:/bin \
  --env=CARGO_HOME="$CARGO_HOME_DIR" \
  --env=CARGO_TARGET_DIR="$WORK_DIR/target" \
  "$WORK_DIR/build-dir" \
  sh -c '
    set -euo pipefail
    REPO_ROOT="$1"
    APP_ID="$2"

    rm -rf "$CARGO_TARGET_DIR"/release/build/dji-mic-mini-kde-* "$CARGO_TARGET_DIR"/release/.fingerprint/dji-mic-mini-kde-*
    cargo build --release --locked --manifest-path "$REPO_ROOT/Cargo.toml"

    install -d /app/bin /app/share/dji-mic-mini-kde/qml /app/share/dji-mic-mini-kde/assets /app/share/knotifications6
    install -Dm755 "$CARGO_TARGET_DIR/release/dji-mic" /app/bin/dji-mic
    install -Dm755 "$CARGO_TARGET_DIR/release/dji-mic-tray" /app/bin/dji-mic-tray
    install -Dm644 "$REPO_ROOT"/qml/*.qml -t /app/share/dji-mic-mini-kde/qml/
    install -Dm644 "$REPO_ROOT/99-dji-mic.rules" -t /app/share/dji-mic-mini-kde/
    install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" -t /app/share/dji-mic-mini-kde/assets/
    install -Dm644 "$REPO_ROOT/packaging/dji-mic-tray.notifyrc" /app/share/knotifications6/dji-mic-tray.notifyrc

    QT_QPA_PLATFORM=offscreen DJI_MIC_APP_DIR=/app/share/dji-mic-mini-kde /app/bin/dji-mic-tray --ui-test

    install -Dm644 "$REPO_ROOT/data/${APP_ID}.desktop" -t /app/share/applications/
    install -Dm644 "$REPO_ROOT/data/${APP_ID}.metainfo.xml" -t /app/share/metainfo/
    install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" "/app/share/icons/hicolor/256x256/apps/${APP_ID}.png"
    install -Dm644 "$REPO_ROOT/assets/dji-mic-tray.png" "/app/share/icons/hicolor/256x256/apps/dji-mic-tray.png"
    install -Dm644 "$REPO_ROOT/LICENSE" -t "/app/share/licenses/${APP_ID}/"
  ' _ "$REPO_ROOT" "$APP_ID"

echo "==> Composing AppStream catalog and bundle icons for Bazaar / KDE Discover..."
flatpak build "$WORK_DIR/build-dir" sh -c '
  set -euo pipefail
  APP_ID="$1"
  IMG_FMT_ARG=""
  if appstreamcli compose --help 2>&1 | grep -q -- "--image-format"; then
    IMG_FMT_ARG="--image-format=png"
  fi
  appstreamcli compose \
    --prefix=/ \
    --origin=flatpak \
    --components="$APP_ID" \
    --result-root=/app/share/app-info \
    --data-dir=/app/share/app-info/xmls \
    --icons-dir=/app/share/app-info/icons/flatpak \
    $IMG_FMT_ARG \
    --no-net \
    /app
  if [ -f /app/share/app-info/xmls/flatpak.xml.gz ]; then
    mv /app/share/app-info/xmls/flatpak.xml.gz "/app/share/app-info/xmls/${APP_ID}.xml.gz"
  fi
  for sz in 64x64 128x128; do
    if [ -f "/app/share/app-info/icons/flatpak/${sz}/${APP_ID}.png" ]; then
      install -Dm644 \
        "/app/share/app-info/icons/flatpak/${sz}/${APP_ID}.png" \
        "/app/share/icons/hicolor/${sz}/apps/${APP_ID}.png"
    fi
  done
' _ "$APP_ID"

flatpak build-finish "$WORK_DIR/build-dir" \
  --command=dji-mic-tray \
  --share=ipc \
  --socket=wayland \
  --socket=fallback-x11 \
  --socket=pulseaudio \
  --device=all \
  --talk-name=org.kde.StatusNotifierWatcher \
  --talk-name=org.freedesktop.Notifications \
  --talk-name=org.kde.kglobalaccel \
  --filesystem=xdg-config/kdeglobals:ro \
  --filesystem=host-etc:ro \
  --filesystem=xdg-config/dji-mic-tray:create \
  --filesystem=xdg-run/dji-mic-tray.sock:create \
  --filesystem=xdg-run/dji-mic-status.json:create

flatpak build-export "$WORK_DIR/repo" "$WORK_DIR/build-dir" stable
flatpak build-bundle "$WORK_DIR/repo" "$OUT_DIR/$BUNDLE_NAME" "$APP_ID" stable --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
(cd "$OUT_DIR" && sha256sum "$BUNDLE_NAME" > "${BUNDLE_NAME}.sha256" && ln -sf "$BUNDLE_NAME" dji-mic-mini-kde.flatpak)
rm -rf "$WORK_DIR/build-dir" "$WORK_DIR/repo"

echo "==> Built Flatpak bundle: $OUT_DIR/$BUNDLE_NAME"

if [[ "${1:-}" != "--no-install" ]]; then
  echo "==> Installing Flatpak bundle..."
  if ! flatpak install --user -y --reinstall "$OUT_DIR/$BUNDLE_NAME"; then
    echo "==> User scope install failed; retrying with --system..."
    flatpak install --system -y --reinstall "$OUT_DIR/$BUNDLE_NAME"
  fi
fi
