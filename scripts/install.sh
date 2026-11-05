#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.local/share/applications"
mkdir -p "$HOME/.local/share/icons/hicolor/256x256/apps"
mkdir -p "$HOME/.local/share/icons/hicolor/64x64/apps"
mkdir -p "$HOME/.config/systemd/user"

chmod +x "$REPO_ROOT/dji-mic" "$REPO_ROOT/dji-mic-tray"
ln -sf "$REPO_ROOT/dji-mic" "$HOME/.local/bin/dji-mic"
ln -sf "$REPO_ROOT/dji-mic-tray" "$HOME/.local/bin/dji-mic-tray"

if [[ -f "$REPO_ROOT/assets/dji-mic-tray.png" ]]; then
  cp -f "$REPO_ROOT/assets/dji-mic-tray.png" "$HOME/.local/share/icons/hicolor/256x256/apps/dji-mic-tray.png"
  cp -f "$REPO_ROOT/assets/dji-mic-tray.png" "$HOME/.local/share/icons/hicolor/64x64/apps/dji-mic-tray.png"
fi

cat > "$HOME/.local/share/applications/dji-mic-tray.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=KDE DJI Mic Mini Control
GenericName=DJI Mic Mini Telemetry & Hardware Control
Comment=KDE Plasma 6 System Tray, USB Protocol Telemetry, Audio Controls & Battery Monitor for DJI Mic Mini (2ca3:4011)
Exec=$HOME/.local/bin/dji-mic-tray --show
Icon=$REPO_ROOT/assets/dji-mic-tray.png
Terminal=false
Categories=AudioVideo;Audio;Mixer;Utility;Qt;KDE;
StartupNotify=false
StartupWMClass=dji-mic-tray
Keywords=DJI;Mic;Microphone;Battery;USB;Audio;Tray;Control;
EOF

cp -f "$REPO_ROOT/systemd/dji-mic-tray.service" "$HOME/.config/systemd/user/dji-mic-tray.service"
systemctl --user daemon-reload
systemctl --user enable dji-mic-tray.service >/dev/null 2>&1 || true
systemctl --user restart dji-mic-tray.service

echo "[OK] Installed dji-mic, dji-mic-tray, desktop launcher, and restarted dji-mic-tray.service."
