#!/usr/bin/env python3
"""
Automated unit, protocol, security, udev, and headless UI overflow test suite
for KDE DJI Mic Mini Control (dji-mic-mini-kde).
"""
from __future__ import annotations

import ast
import os
import struct
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))

import device  # noqa: E402
import protocol  # noqa: E402


def _make_duml_frame(
    sender_pair: bytes,
    seq: int,
    cmd_type: int,
    cmd_set: int,
    cmd_id: int,
    payload: bytes,
) -> bytes:
    """Helper to wrap a DUML payload with valid header CRC-8 and frame CRC-16."""
    length = 11 + len(payload) + 2
    hdr = bytearray([protocol.SOF, length & 0xFF, ((length >> 8) & 0x03) | protocol.VERSION])
    hdr.append(protocol.crc8_dji(bytes(hdr)))
    hdr.extend(sender_pair)
    hdr.extend([seq & 0xFF, (seq >> 8) & 0xFF, cmd_type, cmd_set, cmd_id])
    hdr.extend(payload)
    crc = protocol.crc16_checksum(bytes(hdr), protocol.V2_INIT)
    hdr.extend([crc & 0xFF, (crc >> 8) & 0xFF])
    return bytes(hdr)


# ---------------------------------------------------------------------------
# 1. CRC-8 & CRC-16 Verification
# ---------------------------------------------------------------------------
def test_crc8_header_validation():
    hdr_prefix = bytes([0x55, 37 & 0xFF, 0x04])
    c8 = protocol.crc8_dji(hdr_prefix)
    full_hdr = hdr_prefix + bytes([c8])
    assert protocol.crc8_dji(full_hdr[:3]) == full_hdr[3]


def test_crc16_frame_validation():
    pkt = protocol.build_get_version_packet(seq=0x0042)
    assert pkt[0] == 0x55
    total_len = pkt[1] | ((pkt[2] & 0x03) << 8)
    assert total_len == len(pkt) == 13
    expected_crc16 = struct.unpack("<H", pkt[-2:])[0]
    assert protocol.crc16_checksum(pkt[:-2], protocol.V2_INIT) == expected_crc16


def test_crc16_bytes_residue():
    cmd_v1 = protocol.build_command_packet("v1", 10, "noise-cancel", "strong")
    assert protocol.crc16_checksum(cmd_v1, protocol.V1_INIT) == protocol.V1_RESIDUE

    cmd_v2 = protocol.build_command_packet("v2", 11, "noise-cancel", "strong")
    assert protocol.crc16_checksum(cmd_v2, protocol.V2_INIT) == protocol.V2_RESIDUE


# ---------------------------------------------------------------------------
# 2. DUML Stream Framing (`take_frames`) with iAP2 Noise & Corrupted Packets
# ---------------------------------------------------------------------------
def test_take_frames_handles_iap2_sync_and_corrupted_bytes():
    valid_pkt_1 = protocol.build_get_version_packet(seq=1)
    valid_pkt_2 = protocol.build_command_packet("v1", 2, "low-cut", "on")
    iap2_sync = bytes.fromhex("ff550200ee10")
    corrupted_hdr = bytes([0x55, 0x10, 0x04, 0x00])  # Invalid CRC8

    stream = bytearray(iap2_sync + valid_pkt_1 + corrupted_hdr + iap2_sync + valid_pkt_2)
    frames = protocol.take_frames(stream)

    assert len(frames) == 2
    assert frames[0] == valid_pkt_1
    assert frames[1] == valid_pkt_2


# ---------------------------------------------------------------------------
# 3. V1 Heartbeat Decoding (DJI Mic Mini `01.01.00.56`)
# ---------------------------------------------------------------------------
def _build_v1_heartbeat_frame(
    gauge_step: int = 1,
    s_byte: int = 0xB4,
    rx_flags0: int = 0x2B,
    rx_flags1: int = 0xA0,
    level: int = 42,
) -> bytes:
    """Construct a valid 70-byte v1 heartbeat frame (1 TX connected + 1 absent stub + 22B RX)."""
    f_byte = 0x21 | ((gauge_step & 0x07) << 2)
    # 3-byte prefix after cmd_id (0x00 dialect v1 + 2 reserved bytes)
    payload = bytearray([0x00, 0x00, 0x00])

    # Slot 1: 23-byte active TX
    tx1 = bytearray(23)
    tx1[0] = s_byte
    tx1[1] = f_byte
    tx1[2] = level
    tx1[3] = 0x00  # LEDs on
    tx1[4:8] = bytes([1, 1, 0, 56])
    tx1[8] = 14
    tx1[9:23] = b"8PDXN5E035JPX5"
    payload.extend(tx1)

    # Slot 2: 9-byte absent stub
    tx2_stub = bytearray(9)
    tx2_stub[0] = s_byte
    tx2_stub[1] = 0x00  # bit 0x20 == 0 -> absent
    payload.extend(tx2_stub)

    # RX slot: 22 bytes
    rx_slot = bytearray(22)
    rx_slot[0] = rx_flags0  # gain_code = (0x2B >> 4) & 0x07 = 2 (-6 dB), stereo=1, auto-off=1, cam-power=1
    rx_slot[1] = rx_flags1  # safety-track=1 (0x80), clip-limiter=1 (0x20)
    rx_slot[2:6] = bytes([1, 1, 0, 56])
    rx_slot[6] = 14
    rx_slot[7:21] = b"8PCXN5H02429TH"
    rx_slot[21] = 0x00  # plug-free off
    payload.extend(rx_slot)

    return _make_duml_frame(b"\x01\x0a", seq=0x0123, cmd_type=0x00, cmd_set=0x5B, cmd_id=0x03, payload=bytes(payload))


def test_decode_v1_heartbeat_with_tx1():
    pkt = _build_v1_heartbeat_frame(gauge_step=1, level=64)
    frames = protocol.take_frames(bytearray(pkt))
    assert len(frames) == 1

    st = protocol.DeviceStatus()
    res = protocol.decode_frame(st, frames[0])
    assert res is not None
    assert st.dialect == "v1"
    assert st.connected is True
    assert st.rx is not None
    assert st.rx.serial == "8PCXN5H02429TH"
    assert st.rx.firmware == "01.01.00.56"
    assert st.gain_dial_db == -6
    assert st.settings["stereo"] == "stereo"
    assert st.settings["low-cut"] == "on"
    assert st.settings["noise-cancel"] == "strong"
    assert st.settings["safety-track"] == "on"
    assert st.settings["clip-limiter"] == "on"

    assert st.tx_count() == 1
    tx1 = st.tx[0]
    assert tx1 is not None
    assert tx1.connected is True
    assert tx1.serial == "8PDXN5E035JPX5"
    assert tx1.firmware == "01.01.00.56"
    assert tx1.battery == 1
    assert tx1.battery_percent == 100
    assert tx1.level == 64
    assert tx1.nc_enabled is True
    assert tx1.nc_mode == "strong"
    assert tx1.low_cut is True


@pytest.mark.parametrize(
    "gauge_step,expected_pct",
    [
        (1, 100),
        (2, 85),
        (3, 70),
        (4, 55),
        (5, 40),
        (6, 25),
        (7, 5),
    ],
)
def test_tx_battery_gauge_mapping(gauge_step: int, expected_pct: int):
    pkt = _build_v1_heartbeat_frame(gauge_step=gauge_step)
    st = protocol.DeviceStatus()
    assert protocol.decode_frame(st, pkt) is not None
    assert st.tx[0] is not None
    assert st.tx[0].battery == gauge_step
    assert st.tx[0].battery_percent == expected_pct


# ---------------------------------------------------------------------------
# 4. V2 Dialect Status & GetVersion (`0x00/0x01`) Decoding
# ---------------------------------------------------------------------------
def test_decode_v2_status_and_records():
    # 52-byte header (11B DUML prefix + 41B v2 status header) + 32B TX1 slot + 2B CRC = 86B total
    # Wait: frame[11] == 0x03 for v2
    payload = bytearray(41 + 32)
    payload[0] = 0x03  # frame[11] = 0x03 -> dialect v2
    payload[20 - 11] = 0x28  # flags1: nc_enabled (0x20), strong (0x08)
    payload[21 - 11] = (2 << 5) | 0x05  # flags2: RX battery step 2 (85%), stereo (0x04), auto-off (0x01)
    payload[22 - 11] = 6  # +6 dB gain dial
    payload[44 - 11] = 0x01  # TX1 connected mask
    payload[48 - 11] = 0x50  # safety-track (0x40) + clip-limiter (0x10)

    # TX1 slot starts at frame[52] -> payload[41]
    tx_off = 41
    payload[tx_off + 1] = 1  # unit 1
    payload[tx_off + 6] = 0x90  # nc_button + auto-off
    payload[tx_off + 7] = (3 << 2) | 0x01  # battery step 3 (70%) + nc_enabled
    payload[tx_off + 9] = 0x60  # rich tone (0x40) + low-cut (0x20)

    pkt = _make_duml_frame(b"\x16\x0a", seq=20, cmd_type=0x00, cmd_set=0x5B, cmd_id=0x03, payload=bytes(payload))
    st = protocol.DeviceStatus()
    assert protocol.decode_frame(st, pkt) is not None
    assert st.dialect == "v2"
    assert st.gain_dial_db == 6
    assert st.rx is not None
    assert st.rx.battery_percent == 85
    assert st.tx[0] is not None
    assert st.tx[0].battery_percent == 70
    assert st.tx[0].voice_tone == "rich"
    assert st.tx[0].low_cut is True


def test_decode_get_version_response():
    payload = bytearray(30)
    payload[0:2] = b"\x00\x02"
    model = b"DJI MIC MINI RX"
    payload[2 : 2 + len(model)] = model
    payload[22:26] = bytes([56, 0, 1, 1])  # 01.01.00.56
    payload[26:30] = bytes([1, 0, 0, 0])   # 01.00.00.00

    pkt = _make_duml_frame(b"\x5a\x02", seq=1, cmd_type=0x80, cmd_set=0x00, cmd_id=0x01, payload=bytes(payload))
    st = protocol.DeviceStatus()
    assert protocol.decode_frame(st, pkt) is not None
    assert st.rx is not None
    assert st.rx.hardware_model == "DJI MIC MINI RX"
    assert st.rx.firmware == "01.01.00.56"
    assert st.rx.hardware_revision == "01.00.00.00"


# ---------------------------------------------------------------------------
# 5. Security & Command Allowlist Verification
# ---------------------------------------------------------------------------
def test_safe_hid_commands_allowlist_blocks_destructive_commands():
    assert device.SAFE_HID_COMMANDS == frozenset({0x59, 0x5E, 0x6A})
    # Factory reset (0x55) and power off (0x52) must NEVER be in SAFE_HID_COMMANDS
    assert 0x55 not in device.SAFE_HID_COMMANDS
    assert 0x52 not in device.SAFE_HID_COMMANDS

    # Unknown / destructive settings must be rejected by build_command_packet
    with pytest.raises(ValueError):
        protocol.build_command_packet("v1", 1, "factory-reset", "on")


def test_no_shell_true_in_codebase():
    """Verify no Python file in the repository uses subprocess(..., shell=True)."""
    py_files = [
        REPO_ROOT / "protocol.py",
        REPO_ROOT / "audio.py",
        REPO_ROOT / "device.py",
        REPO_ROOT / "dji-mic",
        REPO_ROOT / "dji-mic-tray",
    ]
    for path in py_files:
        source = path.read_text(encoding="utf-8")
        tree = ast.parse(source, filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Call):
                for kw in node.keywords:
                    if kw.arg == "shell" and isinstance(kw.value, ast.Constant) and kw.value.value is True:
                        pytest.fail(f"Security violation: shell=True found in {path}")


def test_udev_status_check():
    ust = device.check_udev_status()
    assert isinstance(ust.rule_installed, bool)
    assert ust.rule_path == "/etc/udev/rules.d/99-dji-mic.rules"
    assert "2ca3" in device.UDEV_RULE_CONTENT
    assert "4011" in device.UDEV_RULE_CONTENT


# ---------------------------------------------------------------------------
# 6. Headless UI Layout & Overflow Verification (`--ui-test`)
# ---------------------------------------------------------------------------
def test_headless_qml_ui_overflow_suite():
    env = os.environ.copy()
    env["QT_QPA_PLATFORM"] = "offscreen"
    proc = subprocess.run(
        [sys.executable, str(REPO_ROOT / "dji-mic-tray"), "--ui-test"],
        env=env,
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc.returncode == 0, f"UI test failed:\nSTDOUT: {proc.stdout}\nSTDERR: {proc.stderr}"
    assert "UI_TEST_OK" in proc.stdout
    assert (REPO_ROOT / "assets" / "dji-mic-tray.png").is_file()
