# Changelog

All notable changes to **KDE DJI Mic Mini Control** (`dji-mic-mini-kde`) will be documented in this file.

## [v0.2.5](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.5) — 2026-11-19

### 🔒 Security & Memory Safety
- **Hardened `udev` Rules (`0660` + `uaccess`)**: Tightened `99-dji-mic.rules` and `src/device.rs` from world-writable `MODE="0666"` to `MODE="0660", TAG+="uaccess"` for both `usb` and `hidraw`, sourced cleanly via `include_str!`.
- **PolicyKit Installer Without `sudo` on User Binaries**: Replaced `konsole -e sudo ~/.local/bin/dji-mic` with direct `pkexec` installation of the embedded `99-dji-mic.rules` content and removed all hardcoded home paths across Rust in favor of XDG directories (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_RUNTIME_DIR`).
- **FFI & Pointer Safety**: Marked all pointer-taking `extern "C"` FFI functions `unsafe`, wrapped all C ABI entry points in `std::panic::catch_unwind` with poison-safe mutex guards (`lock_unpoisoned`), replaced `m_lockedFlickable` with `QPointer<QQuickItem>` and `g_controller` with `std::atomic<DjiMicController*>` cleared in `~DjiMicController()`.
- **Strict Protocol & Input Validation**: Enforced strict CRC-16 frame validation in `decode_frame` (rejecting corrupted frames instead of defaulting to `v1`), fixed wrapping `u16` sequence increment, checked negative `libusb_bulk_transfer` return lengths, added CLI volume argument validation (`normalize_volume_spec`) and strict hex validation in `dji-mic raw`, and fixed `dji-mic mute on/off` & `dji-mic-tray` shortcut exit codes.
- **Subprocess, IPC & Flatpak Hardening**: Added strict wall-clock timeouts (`run_command_with_timeout`) and background reaping (`spawn_reaped`), bounded/timed-out concurrent Unix socket IPC with clean `poll()` shutdown, atomic `0600` config/runtime JSON writes (`write_file_atomic_mode`), config comment sanitization, and removed `org.freedesktop.Flatpak` sandbox escape from `scripts/build-flatpak.sh`.

### ⚡ Performance & Resource Optimization
- **Non-Blocking Background USB Worker (`src/controller/worker.rs`)**: Offloaded all USB polling, slow sysfs/ALSA/PipeWire probes, hardware writes, and global shortcut hardware actions off the Qt GUI main thread onto a dedicated Rust worker thread.
- **Single-Shot JSON Snapshot FFI & Granular Signals**: Replaced ~80+ per-tick C string round-trips with a single lock-minimal `rust_get_ui_snapshot_json()` cache synced on digest changes (`telemetryChanged`, `controlsChanged`, `noiseChanged`, `configChanged`, `windowChanged`, `statusChanged`), skipping UI updates when minimized to tray.
- **Tray Icon & QML Fixes**: Fixed `updateTrayUi()` unconditionally rasterizing two `QPixmap`s 4×/sec (and added `vuActive` to `iconSig`), replaced dynamic `Repeater` arrays in `qml/ControlsTab.qml` with static `SpectralBandBar` delegates, and throttled `dji-mic-status.json` disk writes to changed state (>=1s).
- **O(1) CRC-16 & Native Timestamps**: Replaced the 256×256 brute-force CRC-16 search with O(1) XOR (`0xB92D`) math, lookup-table `hex_encode`, O(N) cursor-based `take_frames`, `OnceLock` `librnnoise.so.0` caching, `libc::localtime_r` + `strftime` timestamps, and `pw-record --raw` PipeWire capture for room noise profiling (fixing ALSA `EBUSY`).

### 🧱 Refactoring, Testing & Packaging
- **Modular Architecture**: Decomposed `src/controller.rs` (2,452 lines) into `src/controller/{mod,config,desktop,alerts,worker,ipc,shortcuts,ffi}.rs` + `src/util.rs`, introduced typed enums (`DumlDialect`, `SettingTarget`, `AiNcMode`, `NotificationEvent`), routed notifications natively over `Qt6DBus`, integrated native `KColorSchemeManager` theme switching, and split `qml/Main.qml` (1,898 lines) into 10 reusable components and 5 tab files.
- **Packaging & Rust Test Suite**: Updated `scripts/install.sh` and `scripts/build-flatpak.sh` for Rust release binaries (compiled inside `org.kde.Sdk//6.11` for Flatpak) + modular QML + `.desktop` actions + `.notifyrc`, updated `README.md`, removed obsolete pre-Rust Python files (`pyproject.toml`, `tests/test_dji_mic.py`), and expanded `cargo test` with `tests/integration_tests.rs` (including headless `--ui-test`).

## [v0.2.4](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.4) — 2026-11-13

### Added
- Added `knotifications6` `.notifyrc` integration (`dji-mic-tray.notifyrc`) and 15 KDE global keyboard shortcuts via `org.kde.kglobalaccel`.

## [v0.2.3](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.3) — 2026-11-06

### Added
- Added Start on Login (`systemd` user unit + XDG autostart fallback) and Start Minimized to System Tray settings.

## [v0.2.3-deps](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.3-deps) — 2026-11-13

### 📦 Bumped Dependencies (1 package)

| Package | Previous | Updated | Scope |
| :--- | :--- | :--- | :--- |
| `cc` | `1.11.1` | **`1.11.2`** | Direct (`Cargo.toml`) |

## [v0.2.2](https://github.com/russellmilliner/dji-mic-mini-kde/releases/tag/v0.2.2) — 2026-11-06

### Added & Changed
- **Version in Window Title Bar & Header**: Added compile-time `appVersion` property (`CARGO_PKG_VERSION`) to `DjiMicController` and displayed `KDE DJI Mic Mini Control v0.2.2` in the main window title bar, top header banner, and About card.
- **Unified Bordered Tab Content Box Attached Directly to `QQC2.TabBar` (Rule 7.6)**: Grouped `mainTabBar` and a bordered `tabContentBox` inside a `spacing: 0` `tabGroupContainer` so each tab attaches directly to the top edge of the surrounding content box.

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
