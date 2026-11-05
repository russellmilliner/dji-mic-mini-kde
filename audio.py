"""
Linux Audio Subsystem Integration (ALSA & PipeWire / WirePlumber) for DJI Mic Mini.
"""

from __future__ import annotations

import re
import shutil
import subprocess
from typing import Any, Dict, Optional


def get_alsa_card_info() -> Dict[str, Any]:
    """Find ALSA sound card index, ID, and description for DJI Mic Mini."""
    info: Dict[str, Any] = {
        "card_index": None,
        "card_id": None,
        "card_name": None,
        "usb_bus_path": None,
    }
    try:
        with open("/proc/asound/cards", "r", encoding="utf-8", errors="replace") as f:
            lines = f.read().splitlines()
        for idx, line in enumerate(lines):
            if any(k in line for k in ("DJI MIC MINI", "Wireless Mic Rx", "[MINI", "[Rx")):
                m = re.match(r"\s*(\d+)\s+\[(\S+)\s*\]:\s*(.+)", line)
                if m:
                    info["card_index"] = m.group(1)
                    info["card_id"] = m.group(2)
                    info["card_name"] = m.group(3).strip()
                    if idx + 1 < len(lines):
                        bus_m = re.search(r"at\s+(usb-\S+)", lines[idx + 1])
                        if bus_m:
                            info["usb_bus_path"] = bus_m.group(1).rstrip(",")
                    return info
    except Exception:
        pass
    return info


def get_alsa_card_index() -> Optional[str]:
    info = get_alsa_card_info()
    return info.get("card_index")


def get_alsa_volume() -> Optional[Dict[str, Any]]:
    """Query current capture volume and mute state via ALSA mixer or wpctl fallback."""
    card = get_alsa_card_index()
    if card is not None and shutil.which("amixer"):
        try:
            res = subprocess.run(
                ["amixer", "-c", str(card), "get", "Mic"],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                pct_match = re.search(r"\[(\d+)%\]", res.stdout)
                db_match = re.search(r"\[(-?\d+\.?\d*dB)\]", res.stdout)
                switch_match = re.search(r"\[(on|off)\]", res.stdout)
                return {
                    "percent": int(pct_match.group(1)) if pct_match else None,
                    "db": db_match.group(1) if db_match else None,
                    "muted": (switch_match.group(1) == "off") if switch_match else False,
                }
        except Exception:
            pass

    pw_id = get_pipewire_source_id()
    if pw_id and shutil.which("wpctl"):
        try:
            res = subprocess.run(
                ["wpctl", "get-volume", str(pw_id)],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                vol_m = re.search(r"Volume:\s*([0-9.]+)", res.stdout)
                muted = "MUTED" in res.stdout
                pct = int(round(float(vol_m.group(1)) * 100)) if vol_m else None
                return {
                    "percent": pct,
                    "db": "0.00dB" if pct == 100 else None,
                    "muted": muted,
                }
        except Exception:
            pass
    return None


def set_alsa_volume(val_str: str) -> bool:
    """Set capture volume via ALSA mixer and PipeWire wpctl."""
    ok = False
    card = get_alsa_card_index()
    if card is not None and shutil.which("amixer"):
        try:
            res = subprocess.run(
                ["amixer", "-c", str(card), "set", "Mic", val_str],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                ok = True
        except Exception:
            pass

    pw_id = get_pipewire_source_id()
    if pw_id and shutil.which("wpctl"):
        try:
            res = subprocess.run(
                ["wpctl", "set-volume", str(pw_id), val_str],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                ok = True
        except Exception:
            pass
    return ok


def set_alsa_mute(muted: bool) -> bool:
    """Mute or unmute capture via ALSA mixer and PipeWire wpctl."""
    ok = False
    card = get_alsa_card_index()
    if card is not None and shutil.which("amixer"):
        cmd_val = "mute" if muted else "unmute"
        try:
            res = subprocess.run(
                ["amixer", "-c", str(card), "set", "Mic", cmd_val],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                ok = True
        except Exception:
            pass

    pw_id = get_pipewire_source_id()
    if pw_id and shutil.which("wpctl"):
        try:
            res = subprocess.run(
                ["wpctl", "set-mute", str(pw_id), "1" if muted else "0"],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                ok = True
        except Exception:
            pass
    return ok


def get_pipewire_source_id() -> Optional[str]:
    """Find WirePlumber source ID for DJI Mic Mini."""
    if not shutil.which("wpctl"):
        return None
    try:
        res = subprocess.run(
            ["wpctl", "status"],
            capture_output=True,
            text=True,
            timeout=1.5,
            check=False,
        )
        if res.returncode != 0:
            return None

        in_sources = False
        for line in res.stdout.splitlines():
            if "Sources:" in line:
                in_sources = True
                continue
            if in_sources:
                if line.strip().startswith("Filters:") or line.strip().startswith("Streams:"):
                    in_sources = False
                    continue
                if "DJI MIC MINI" in line or "Wireless Mic Rx" in line:
                    m = re.search(r"(\d+)\.\s+(?:DJI MIC MINI|Wireless Mic Rx)", line)
                    if m:
                        return m.group(1)
        return None
    except Exception:
        return None


def get_pipewire_stream_details() -> Dict[str, Any]:
    """Return full PipeWire / PulseAudio source details for DJI Mic Mini."""
    details: Dict[str, Any] = {
        "wpctl_id": get_pipewire_source_id(),
        "pactl_id": None,
        "node_name": None,
        "sample_spec": None,
        "stream_state": None,
    }
    if shutil.which("pactl"):
        try:
            res = subprocess.run(
                ["pactl", "list", "sources", "short"],
                capture_output=True,
                text=True,
                timeout=1.5,
                check=False,
            )
            if res.returncode == 0:
                for line in res.stdout.splitlines():
                    if "DJI_MIC_MINI" in line or "Wireless_Mic_Rx" in line:
                        parts = line.split("\t")
                        if len(parts) >= 5:
                            details["pactl_id"] = parts[0].strip()
                            details["node_name"] = parts[1].strip()
                            details["sample_spec"] = parts[3].strip()
                            details["stream_state"] = parts[4].strip()
                        break
        except Exception:
            pass
    return details
