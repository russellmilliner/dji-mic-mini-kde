# Changelog

All notable changes to **KDE DJI Mic Mini Control** (`dji-mic-mini-kde`) will be documented in this file.

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
