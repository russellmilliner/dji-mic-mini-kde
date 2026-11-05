#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.russellmilliner.DjiMicMiniControl"
VERSION="$(grep -E '^version\s*=' "$REPO_ROOT/pyproject.toml" | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/')"
ARCH="$(uname -m)"
WORK_DIR="${FLATPAK_WORK_DIR:-$REPO_ROOT/dist/flatpak-work}"
OUT_DIR="${FLATPAK_OUT_DIR:-$REPO_ROOT/dist}"
BUNDLE_NAME="dji-mic-mini-kde-v${VERSION}-${ARCH}.flatpak"

echo "==> Building KDE DJI Mic Mini Control v${VERSION} Flatpak (${ARCH})..."
rm -rf "$WORK_DIR/build-dir" "$WORK_DIR/repo"
mkdir -p "$WORK_DIR" "$OUT_DIR"

flatpak build-init \
  "$WORK_DIR/build-dir" \
  "$APP_ID" \
  org.kde.Sdk \
  org.kde.Platform \
  6.11

flatpak build \
  --share=network \
  --filesystem="$REPO_ROOT":ro \
  --filesystem="$WORK_DIR":rw \
  "$WORK_DIR/build-dir" \
  sh -c "
    set -euo pipefail
    pip3 install --prefix=/app --no-deps --no-build-isolation \
      https://files.pythonhosted.org/packages/71/e5/aaaa629a17f5abd9d6f80d5bd8cbbc8b91434f017d03eb5df7d4258ecb41/pyqt6_sip-13.13.0-cp313-cp313-manylinux1_x86_64.manylinux_2_5_x86_64.whl \
      https://files.pythonhosted.org/packages/c3/a0/bd1399740dfa80c0a94d20b02d89962a31458233dcf70eaa09bfbccf3d0f/pyqt6-6.11.0-cp310-abi3-manylinux_2_34_x86_64.whl

    install -d /app/lib/python3.13/site-packages/PyQt6/Qt6
    ln -sf /usr/lib/plugins /app/lib/python3.13/site-packages/PyQt6/Qt6/plugins
    ln -sf /usr/lib/qml /app/lib/python3.13/site-packages/PyQt6/Qt6/qml

    install -d /app/share/dji-mic-mini-kde/qml /app/share/dji-mic-mini-kde/assets /app/bin
    install -m644 '$REPO_ROOT/protocol.py' '$REPO_ROOT/device.py' '$REPO_ROOT/audio.py' '$REPO_ROOT/99-dji-mic.rules' -t /app/share/dji-mic-mini-kde/
    install -m755 '$REPO_ROOT/dji-mic' '$REPO_ROOT/dji-mic-tray' -t /app/share/dji-mic-mini-kde/
    install -m644 '$REPO_ROOT/qml/Main.qml' -t /app/share/dji-mic-mini-kde/qml/
    install -m644 '$REPO_ROOT/assets/dji-mic-tray.png' -t /app/share/dji-mic-mini-kde/assets/

    printf '%s\n' \
      '#!/usr/bin/env sh' \
      'export PYTHONPATH=/app/lib/python3.13/site-packages:/app/share/dji-mic-mini-kde' \
      'export QT_PLUGIN_PATH=/usr/lib/plugins' \
      'export QML2_IMPORT_PATH=/usr/lib/qml' \
      'export QT_QUICK_CONTROLS_STYLE=org.kde.desktop' \
      'exec /usr/bin/python3 /app/share/dji-mic-mini-kde/dji-mic-tray \"\$@\"' \
      > /app/bin/dji-mic-tray
    chmod 0755 /app/bin/dji-mic-tray

    printf '%s\n' \
      '#!/usr/bin/env sh' \
      'export PYTHONPATH=/app/lib/python3.13/site-packages:/app/share/dji-mic-mini-kde' \
      'exec /usr/bin/python3 /app/share/dji-mic-mini-kde/dji-mic \"\$@\"' \
      > /app/bin/dji-mic
    chmod 0755 /app/bin/dji-mic

    QT_QPA_PLATFORM=offscreen /app/bin/dji-mic-tray --ui-test

    install -Dm644 '$REPO_ROOT/data/${APP_ID}.desktop' -t /app/share/applications/
    install -Dm644 '$REPO_ROOT/data/${APP_ID}.metainfo.xml' -t /app/share/metainfo/
    install -Dm644 '$REPO_ROOT/assets/dji-mic-tray.png' /app/share/icons/hicolor/256x256/apps/${APP_ID}.png
    install -Dm644 '$REPO_ROOT/LICENSE' -t /app/share/licenses/${APP_ID}/
  "

flatpak build-finish "$WORK_DIR/build-dir" \
  --command=dji-mic-tray \
  --share=ipc \
  --socket=wayland \
  --socket=fallback-x11 \
  --socket=pulseaudio \
  --device=all \
  --talk-name=org.kde.StatusNotifierWatcher \
  --talk-name=org.freedesktop.Notifications \
  --talk-name=org.freedesktop.Flatpak \
  --filesystem=xdg-config/kdeglobals:ro \
  --filesystem=host-etc:ro \
  --filesystem=xdg-config/dji-mic-tray:create \
  --filesystem=xdg-run/dji-mic-tray.sock:create \
  --filesystem=xdg-run/dji-mic-status.json:create

flatpak build-export "$WORK_DIR/repo" "$WORK_DIR/build-dir" stable
flatpak build-bundle "$WORK_DIR/repo" "$OUT_DIR/$BUNDLE_NAME" "$APP_ID" stable --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo
(cd "$OUT_DIR" && sha256sum "$BUNDLE_NAME" > "${BUNDLE_NAME}.sha256" && ln -sf "$BUNDLE_NAME" dji-mic-mini-kde.flatpak)
rm -rf "$WORK_DIR"

echo "==> Built Flatpak bundle: $OUT_DIR/$BUNDLE_NAME"

if [[ "${1:-}" != "--no-install" ]]; then
  echo "==> Installing Flatpak bundle into user installation..."
  flatpak install --user -y --reinstall "$OUT_DIR/$BUNDLE_NAME"
fi
