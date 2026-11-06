# Changelog

All notable changes to **KDE DJI Mic Mini Control** (`dji-mic-mini-kde`) will be documented in this file.

## [v0.2.1](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.1) — 2026-11-06

### Added
- **Hardware-Aware `KStatusNotifierItem` System Tray Integration** (`src/cpp/qt_bridge.cpp`, `src/lib.rs`):
  - Migrated from `QSystemTrayIcon` (which hardcodes `Status = "Active"`) to KDE Frameworks 6 `KStatusNotifierItem` (`Category = Hardware`).
  - Automatically switches tray `Status` between `Active` (shown on the KDE panel when TX1/TX2 is linked, an app is capturing from the DJI Mic Mini in PipeWire, or the AI NC virtual mic is active) and `Passive` (tucked into the hidden tray drawer when idle).
- **AI Noise Cancellation & Ambient Room Noise Learning** (`src/audio.rs`, `qml/Main.qml`):
  - Added 3.5-second ambient room noise profiler (`learn_noise_profile`) that records a live 48 kHz mono calibration sample from the DJI Mic Mini RX, computes a 4-band spectral fingerprint (Low Rumble `<150 Hz`, Fan/Mains Hum `150–400 Hz`, Mid Room `400–2500 Hz`, High Hiss `>2500 Hz`), and automatically calibrates hardware TX Noise Cancellation (`Basic`/`Strong`), `150 Hz` Low-Cut filter, Clip Limiter, and the PipeWire `filter-chain` virtual microphone (`dji_mic_mini_ai_nc_source`).

### Changed & Fixed
- **KDE Plasma 6 / Qt 6 QML Reviewer Polish**:
  - Scaled ALSA Capture Volume label widths by `root.zoomScale`, guarded `w->raise()` when running under `QT_QPA_PLATFORM=offscreen`, and added synthetic `QEvent::HoverLeave` dispatch on Ploopy high-resolution wheel scroll start.

## [v0.2.0-deps](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.0-deps) — 2026-11-06

### 📦 Bumped Dependencies (1 package)

| Package | Previous | Updated | Scope |
| :--- | :--- | :--- | :--- |
| `cc` | `1.11.0` | **`1.11.1`** | Direct (`Cargo.toml`) |

## [v0.2.0](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.0) — 2026-11-04

### Highlights
- Added official KDE Plasma 6 Flatpak package (io.github.russellmilliner.DjiMicMiniControl)

## [v0.1.0](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.1.0) — 2026-11-04

### Commits
- feat: KDE DJI Mic Mini Control v0.1.0 initial release (60b636f)

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-04

### Added
- **KDE DJI Mic Mini Control (`dji-mic-tray`)**: Native KDE Plasma 6 / Kirigami (`Kirigami.ApplicationWindow`) 5-tab control center and system tray daemon for the DJI Mic Mini (`2ca3:4011`).
- **DUML `v1` (`01.01.00.56`) & `v2` USB Telemetry (`protocol.py`, `device.py`)**: Non-intrusive `libusb-1.0` bulk interface claim on USB Interface 6 (`EP 0x86 IN` / `EP 0x06 OUT`) decoding live `~15.4 Hz` `0x5B/0x03` heartbeats and `0x00/0x01` GetVersion responses without detaching Linux `snd-usb-audio`.
- **Wireless Transmitter Battery & Audio Telemetry**: Real-time 7-step battery gauge (`100%` down to `5%`), live microphone VU meter (`0–255`), serial numbers, firmware versions, and hardware setting flags for TX1 and TX2.
- **Hardware & Linux Audio Controls (`audio.py`)**: Instant USB hardware toggles (Noise Cancellation Basic/Strong, Stereo/Mono, Low-Cut filter, Safety Track `-6 dB`, Clip Limiter, TX LEDs, Auto-Off, Camera Power Sync, Plug-Free Speaker) and PipeWire/WirePlumber/ALSA capture volume & mute controls.
- **Desktop Disconnection & Low-Battery Popup Alerts**: Automatic `notify-send` / system tray alerts when the USB-C receiver or wireless transmitters disconnect or drop below the low-battery threshold.
- **Built-in `udev` Rule Manager**: CLI (`dji-mic udev --check` / `sudo dji-mic udev --install`) and GUI Polkit installer for `/etc/udev/rules.d/99-dji-mic.rules`.
- **Security Hardening & Automated `pytest` Suite**: Peer-authenticated Unix IPC socket (`SO_PEERCRED`, `0600`), closed HID/DUML command allowlist, AST verification (`shell=False`), and headless `--ui-test` layout/TabBar/text overflow verification across all 5 tabs.
