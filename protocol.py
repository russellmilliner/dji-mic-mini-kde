"""
DJI Mic Mini DUML Protocol Parser, Framing, and Command Generator.
Supports both v1 (01.01.xx.xx) and v2 (02.03.xx.xx) wire protocols over USB Interface 6.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional

SOF = 0x55
VERSION = 0x04

# CRC-16 table (reflected 0x8408 polynomial / X-25)
CRC16_TABLE = [
    0x0000, 0x1189, 0x2312, 0x329B, 0x4624, 0x57AD, 0x6536, 0x74BF,
    0x8C48, 0x9DC1, 0xAF5A, 0xBED3, 0xCA6C, 0xDBE5, 0xE97E, 0xF8F7,
    0x1081, 0x0108, 0x3393, 0x221A, 0x56A5, 0x472C, 0x75B7, 0x643E,
    0x9CC9, 0x8D40, 0xBFDB, 0xAE52, 0xDAED, 0xCB64, 0xF9FF, 0xE876,
    0x2102, 0x308B, 0x0210, 0x1399, 0x6726, 0x76AF, 0x4434, 0x55BD,
    0xAD4A, 0xBCC3, 0x8E58, 0x9FD1, 0xEB6E, 0xFAE7, 0xC87C, 0xD9F5,
    0x3183, 0x200A, 0x1291, 0x0318, 0x77A7, 0x662E, 0x54B5, 0x453C,
    0xBDCB, 0xAC42, 0x9ED9, 0x8F50, 0xFBEF, 0xEA66, 0xD8FD, 0xC974,
    0x4204, 0x538D, 0x6116, 0x709F, 0x0420, 0x15A9, 0x2732, 0x36BB,
    0xCE4C, 0xDFC5, 0xED5E, 0xFCD7, 0x8868, 0x99E1, 0xAB7A, 0xBAF3,
    0x5285, 0x430C, 0x7197, 0x601E, 0x14A1, 0x0528, 0x37B3, 0x263A,
    0xDECD, 0xCF44, 0xFDDF, 0xEC56, 0x98E9, 0x8960, 0xBBFB, 0xAA72,
    0x6306, 0x728F, 0x4014, 0x519D, 0x2522, 0x34AB, 0x0630, 0x17B9,
    0xEF4E, 0xFEC7, 0xCC5C, 0xDDD5, 0xA96A, 0xB8E3, 0x8A78, 0x9BF1,
    0x7387, 0x620E, 0x5095, 0x411C, 0x35A3, 0x242A, 0x16B1, 0x0738,
    0xFFCF, 0xEE46, 0xDCDD, 0xCD54, 0xB9EB, 0xA862, 0x9AF9, 0x8B70,
    0x8408, 0x9581, 0xA71A, 0xB693, 0xC22C, 0xD3A5, 0xE13E, 0xF0B7,
    0x0840, 0x19C9, 0x2B52, 0x3ADB, 0x4E64, 0x5FED, 0x6D76, 0x7CFF,
    0x9489, 0x8500, 0xB79B, 0xA612, 0xD2AD, 0xC324, 0xF1BF, 0xE036,
    0x18C1, 0x0948, 0x3BD3, 0x2A5A, 0x5EE5, 0x4F6C, 0x7DF7, 0x6C7E,
    0xA50A, 0xB483, 0x8618, 0x9791, 0xE32E, 0xF2A7, 0xC03C, 0xD1B5,
    0x2942, 0x38CB, 0x0A50, 0x1BD9, 0x6F66, 0x7EEF, 0x4C74, 0x5DFD,
    0xB58B, 0xA402, 0x9699, 0x8710, 0xF3AF, 0xE226, 0xD0BD, 0xC134,
    0x39C3, 0x284A, 0x1AD1, 0x0B58, 0x7FE7, 0x6E6E, 0x5CF5, 0x4D7C,
    0xC60C, 0xD785, 0xE51E, 0xF497, 0x8028, 0x91A1, 0xA33A, 0xB2B3,
    0x4A44, 0x5BCD, 0x6956, 0x78DF, 0x0C60, 0x1DE9, 0x2F72, 0x3EFB,
    0xD68D, 0xc704, 0xF59F, 0xE416, 0x90A9, 0x8120, 0xB3BB, 0xA232,
    0x5AC5, 0x4B4C, 0x79D7, 0x685E, 0x1CE1, 0x0D68, 0x3FF3, 0x2E7A,
    0xE70E, 0xF687, 0xC41C, 0xD595, 0xA12A, 0xB0A3, 0x8238, 0x93B1,
    0x6B46, 0x7ACF, 0x4854, 0x59DD, 0x2D62, 0x3CEB, 0x0E70, 0x1FF9,
    0xF78F, 0xE606, 0xD49D, 0xC514, 0xB1AB, 0xA022, 0x92B9, 0x8330,
    0x7BC7, 0x6A4E, 0x58D5, 0x495C, 0x3DE3, 0x2C6A, 0x1EF1, 0x0F78,
]

V1_INIT = 0x0000
V1_RESIDUE = 0xBB01

V2_INIT = 0x3692
V2_RESIDUE = 0x0000

BATTERY_GAUGE_MAP: Dict[int, int] = {
    0: 100,
    1: 100,
    2: 85,
    3: 70,
    4: 55,
    5: 40,
    6: 25,
    7: 5,
}

V1_GAIN_STEP_DB_MAP: Dict[int, int] = {
    1: -12,
    2: -6,
    3: 0,
    4: 6,
    5: 12,
}


def crc8_dji(data: bytes) -> int:
    """Compute DJI DUML CRC-8 over 3-byte header (poly 0x31 reflected = 0x8C, init 0x77)."""
    crc = 0x77
    for b in data:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = ((crc >> 1) ^ 0x8C) & 0xFF
            else:
                crc = (crc >> 1) & 0xFF
    return crc


def crc16_step(crc: int, byte: int) -> int:
    return (crc >> 8) ^ CRC16_TABLE[(crc ^ byte) & 0xFF]


def crc16_checksum(data: bytes, init: int = V2_INIT) -> int:
    crc = init
    for b in data:
        crc = crc16_step(crc, b)
    return crc


def crc16_bytes(body: bytes, init: int, residue: int) -> bytes:
    """Find the 2 CRC bytes such that checksum(body + crc, init) == residue."""
    v = crc16_checksum(body, init)
    for b0 in range(256):
        v1 = crc16_step(v, b0)
        target = residue ^ (v1 >> 8)
        if target in CRC16_TABLE:
            idx = CRC16_TABLE.index(target)
            b1 = (v1 ^ idx) & 0xFF
            return bytes([b0, b1])
    raise RuntimeError("CRC table does not cover residue")


def gauge_to_percent(gauge: Optional[int]) -> Optional[int]:
    if gauge is None:
        return None
    return BATTERY_GAUGE_MAP.get(gauge)


@dataclass
class TxInfo:
    unit: int = 1
    connected: bool = True
    level: Optional[int] = None
    serial: Optional[str] = None
    firmware: Optional[str] = None
    product_name: Optional[str] = None
    bluetooth_id: Optional[str] = None
    voice_tone: Optional[str] = None
    charging: Optional[bool] = None
    battery: Optional[int] = None  # 1 (full, 100%) to 7 (empty, 5%)
    battery_percent: Optional[int] = None
    nc_enabled: Optional[bool] = None
    nc_mode: Optional[str] = None
    low_cut: Optional[bool] = None
    auto_off_15m: Optional[bool] = None
    nc_button_enabled: Optional[bool] = None
    leds_enabled: Optional[bool] = None
    gain_db: Optional[int] = None
    signal_level: Optional[int] = None
    raw_state_byte: Optional[int] = None
    raw_flags_byte: Optional[int] = None
    raw_led_byte: Optional[int] = None


@dataclass
class RxInfo:
    serial: Optional[str] = None
    firmware: Optional[str] = None
    hardware_model: Optional[str] = None
    hardware_revision: Optional[str] = None
    product_name: Optional[str] = None
    bluetooth_id: Optional[str] = None
    battery: Optional[int] = None
    battery_percent: Optional[int] = None
    gain_step_code: Optional[int] = None
    raw_flags0: Optional[int] = None
    raw_flags1: Optional[int] = None
    raw_plug_free_byte: Optional[int] = None


@dataclass
class DeviceStatus:
    connected: bool = False
    dialect: str = "v1"
    gain_dial_db: Optional[int] = None
    nc_enabled: bool = False
    rx: Optional[RxInfo] = None
    tx: List[Optional[TxInfo]] = field(default_factory=lambda: [None, None])
    settings: Dict[str, str] = field(default_factory=dict)
    last_sequence: int = 0
    heartbeat_count: int = 0
    aux_5b05_hex: str = ""
    aux_0474_hex: str = ""
    last_heartbeat_hex: str = ""
    paired_tx_count_hid: Optional[int] = None
    active_tx_index_hid: Optional[int] = None

    def tx_count(self) -> int:
        return sum(1 for t in self.tx if t is not None and t.connected)

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def parse_ascii_string(data: bytes) -> str:
    null_pos = data.find(b"\x00")
    if null_pos != -1:
        data = data[:null_pos]
    return "".join(chr(b) for b in data if 0x20 <= b < 0x7F).strip()


def take_frames(buffer: bytearray) -> List[bytes]:
    """Scan and extract complete DUML frames from stream buffer (filtering out iAP2 sync bytes)."""
    frames: List[bytes] = []
    while True:
        start = -1
        for i in range(len(buffer) - 3):
            if buffer[i] == SOF and (buffer[i + 2] & 0xFC) == VERSION:
                flen = buffer[i + 1] | ((buffer[i + 2] & 0x03) << 8)
                if 13 <= flen <= 256 and crc8_dji(bytes(buffer[i : i + 3])) == buffer[i + 3]:
                    start = i
                    break

        if start == -1:
            if len(buffer) > 3:
                del buffer[:-3]
            break

        if start > 0:
            del buffer[:start]

        if len(buffer) < 4:
            break

        frame_len = buffer[1] | ((buffer[2] & 0x03) << 8)
        if len(buffer) < frame_len:
            break

        frame = bytes(buffer[:frame_len])
        del buffer[:frame_len]
        frames.append(frame)

    return frames


def detect_dialect(frame: bytes) -> Optional[str]:
    if len(frame) < 12:
        return None
    if frame[8] == 0x00 and frame[9] == 0x5B and frame[10] == 0x03:
        if frame[11] == 0x00:
            return "v1"
        elif frame[11] == 0x03:
            return "v2"
    return None


def decode_v1_heartbeat(status: DeviceStatus, frame: bytes) -> Optional[DeviceStatus]:
    """Decode v1 periodic heartbeat frame (56B when 0 TX, 70B when 1 TX, 84B when 2 TX)."""
    header_len = 14
    rx_len = 22
    if len(frame) < header_len + rx_len:
        return None

    status.dialect = "v1"
    status.connected = True
    status.last_sequence = frame[6] | (frame[7] << 8)
    status.heartbeat_count += 1
    status.last_heartbeat_hex = frame.hex()

    off = header_len
    tx_list: List[Optional[TxInfo]] = [None, None]
    state_byte = 0
    flags_byte = 0
    led_off: Optional[bool] = None

    for slot_idx in range(2):
        if off + 1 >= len(frame):
            break
        s_byte = frame[off]
        f_byte = frame[off + 1]
        if (f_byte & 0x20) != 0:
            # Present TX (23 bytes)
            if off + 23 > len(frame):
                break
            state_byte = s_byte
            flags_byte = f_byte
            level = frame[off + 2]
            raw_led = frame[off + 3]
            this_led_off = (raw_led & 0x80) != 0
            led_off = this_led_off

            fw_bytes = frame[off + 4 : off + 8]
            fw_str = f"{fw_bytes[0]:02d}.{fw_bytes[1]:02d}.{fw_bytes[2]:02d}.{fw_bytes[3]:02d}"
            str_len = min(14, frame[off + 8])
            serial = parse_ascii_string(frame[off + 9 : off + 9 + str_len])

            # Battery gauge in bits 2..4 of flags_byte (1 = 100% full .. 7 = 5% empty)
            bat_code = (f_byte >> 2) & 0x07
            bat_pct = gauge_to_percent(bat_code)
            nc_active = (f_byte & 0x01) != 0
            nc_strength = "strong" if (s_byte & 0x20) else "basic"
            low_cut_on = (s_byte & 0x04) != 0
            tx_auto_off = (s_byte & 0x10) != 0
            nc_btn_on = (s_byte & 0x80) != 0

            prev_tx = status.tx[slot_idx]
            tx_list[slot_idx] = TxInfo(
                unit=slot_idx + 1,
                connected=True,
                level=level,
                serial=serial,
                firmware=fw_str,
                product_name=(prev_tx.product_name if prev_tx and prev_tx.product_name else "DJI Mic Mini TX"),
                bluetooth_id=prev_tx.bluetooth_id if prev_tx else None,
                voice_tone=prev_tx.voice_tone if prev_tx else "standard",
                charging=False,
                battery=bat_code,
                battery_percent=bat_pct,
                nc_enabled=nc_active,
                nc_mode=nc_strength,
                low_cut=low_cut_on,
                auto_off_15m=tx_auto_off,
                nc_button_enabled=nc_btn_on,
                leds_enabled=not this_led_off,
                raw_state_byte=s_byte,
                raw_flags_byte=f_byte,
                raw_led_byte=raw_led,
            )
            off += 23
        else:
            # Absent stub (9 bytes)
            if off + 9 > len(frame):
                break
            if state_byte == 0 and flags_byte == 0:
                state_byte = s_byte
                flags_byte = f_byte
            off += 9

    status.tx = tx_list
    status.nc_enabled = (flags_byte & 0x01) != 0

    # Parse RX slot (22 bytes)
    if off + rx_len <= len(frame):
        rx_flags0 = frame[off]
        rx_flags1 = frame[off + 1]
        fw_bytes = frame[off + 2 : off + 6]
        fw_str = f"{fw_bytes[0]:02d}.{fw_bytes[1]:02d}.{fw_bytes[2]:02d}.{fw_bytes[3]:02d}"
        str_len = min(14, frame[off + 6])
        serial = parse_ascii_string(frame[off + 7 : off + 7 + str_len])
        plug_free_byte = frame[off + 21]
        plug_free = plug_free_byte == 0x01

        gain_code = (rx_flags0 >> 4) & 0x07
        if gain_code in V1_GAIN_STEP_DB_MAP:
            status.gain_dial_db = V1_GAIN_STEP_DB_MAP[gain_code]

        prev_rx = status.rx
        status.rx = RxInfo(
            serial=serial,
            firmware=fw_str,
            hardware_model=prev_rx.hardware_model if prev_rx and prev_rx.hardware_model else "DJI MIC MINI RX",
            hardware_revision=prev_rx.hardware_revision if prev_rx else None,
            product_name=prev_rx.product_name if prev_rx and prev_rx.product_name else "DJI Mic Mini Receiver",
            bluetooth_id=prev_rx.bluetooth_id if prev_rx else None,
            gain_step_code=gain_code,
            raw_flags0=rx_flags0,
            raw_flags1=rx_flags1,
            raw_plug_free_byte=plug_free_byte,
        )

        status.settings["noise-cancel"] = "strong" if (state_byte & 0x20) else "basic"
        status.settings["noise-cancel-power"] = "on" if status.nc_enabled else "off"
        status.settings["noise-cancel-button"] = "on" if (state_byte & 0x80) else "off"
        status.settings["tx-auto-off-15m"] = "on" if (state_byte & 0x10) else "off"
        status.settings["low-cut"] = "on" if (state_byte & 0x04) else "off"
        status.settings["stereo"] = "stereo" if (rx_flags0 & 0x08) else "mono"
        status.settings["camera-power"] = "on" if (rx_flags0 & 0x01) else "off"
        status.settings["auto-off-15m"] = "on" if (rx_flags0 & 0x02) else "off"
        status.settings["safety-track"] = "on" if (rx_flags1 & 0x80) else "off"
        status.settings["clip-limiter"] = "on" if (rx_flags1 & 0x20) else "off"
        status.settings["plug-free"] = "on" if plug_free else "off"
        if led_off is not None:
            status.settings["mic-leds"] = "off" if led_off else "on"

    return status


def decode_v2_status(status: DeviceStatus, frame: bytes) -> Optional[DeviceStatus]:
    """Decode v2 periodic status push (52-byte header + 32-byte slot per TX)."""
    header_len = 52
    slot_len = 32
    if len(frame) < header_len + 2:
        return None

    remainder = len(frame) - (header_len + 2)
    if remainder % slot_len != 0:
        return None

    tx_slots_in_frame = min(2, remainder // slot_len)

    flags1 = frame[20]
    flags2 = frame[21]
    dial_raw = frame[22]
    status.gain_dial_db = int.from_bytes([dial_raw], byteorder="little", signed=True)
    flags3 = frame[48]

    status.dialect = "v2"
    status.connected = True
    status.last_sequence = frame[6] | (frame[7] << 8)
    status.heartbeat_count += 1
    status.last_heartbeat_hex = frame.hex()
    status.nc_enabled = (flags1 & 0x20) != 0

    if status.rx is None:
        status.rx = RxInfo(product_name="DJI Mic Mini Receiver")
    status.rx.raw_flags0 = flags1
    status.rx.raw_flags1 = flags2
    status.rx.battery = (flags2 >> 5) & 0x07
    status.rx.battery_percent = gauge_to_percent(status.rx.battery)

    connected_mask = frame[44]
    for i in range(2):
        is_conn = (connected_mask & (1 << i)) != 0
        if is_conn:
            if status.tx[i] is None:
                status.tx[i] = TxInfo(unit=i + 1, connected=True)
            else:
                status.tx[i].connected = True
        else:
            status.tx[i] = None

    tx_flags = None
    low_cut_flags = None
    for pos in range(tx_slots_in_frame):
        slot_off = header_len + pos * slot_len
        unit_byte = frame[slot_off + 1]
        idx = unit_byte - 1
        if 0 <= idx < 2 and status.tx[idx] is not None:
            tx = status.tx[idx]
            slot_flags_6 = frame[slot_off + 6]
            slot_flags_7 = frame[slot_off + 7]
            slot_flags_9 = frame[slot_off + 9]

            tx_flags = slot_flags_6
            low_cut_flags = slot_flags_9

            if slot_flags_9 & 0x80:
                tx.voice_tone = "bright"
            elif slot_flags_9 & 0x40:
                tx.voice_tone = "rich"
            else:
                tx.voice_tone = "standard"

            tx.charging = (slot_flags_7 & 0x02) != 0
            tx.battery = (slot_flags_7 >> 2) & 0x07
            tx.battery_percent = gauge_to_percent(tx.battery)
            tx.nc_enabled = (slot_flags_7 & 0x01) != 0
            tx.leds_enabled = not ((slot_flags_6 & 0x02) != 0)
            tx.auto_off_15m = (slot_flags_6 & 0x10) != 0
            tx.nc_button_enabled = (slot_flags_6 & 0x80) != 0
            tx.low_cut = (slot_flags_9 & 0x20) != 0
            tx.raw_state_byte = slot_flags_6
            tx.raw_flags_byte = slot_flags_7

    status.settings["noise-cancel"] = "strong" if (flags1 & 0x08) else "basic"
    status.settings["noise-cancel-power"] = "on" if (flags1 & 0x20) else "off"
    status.settings["camera-power"] = "on" if (flags1 & 0x80) else "off"
    status.settings["auto-off-15m"] = "on" if (flags2 & 0x01) else "off"
    status.settings["stereo"] = "stereo" if (flags2 & 0x04) else "mono"
    status.settings["plug-free"] = "on" if (flags3 & 0x02) else "off"
    status.settings["clip-limiter"] = "on" if (flags3 & 0x10) else "off"
    status.settings["safety-track"] = "on" if (flags3 & 0x40) else "off"

    if tx_flags is not None:
        status.settings["mic-leds"] = "off" if (tx_flags & 0x02) else "on"
        status.settings["tx-auto-off-15m"] = "on" if (tx_flags & 0x10) else "off"
        status.settings["noise-cancel-button"] = "on" if (tx_flags & 0x80) else "off"

    if low_cut_flags is not None:
        status.settings["low-cut"] = "on" if (low_cut_flags & 0x20) else "off"

    return status


def decode_v2_records(status: DeviceStatus, frame: bytes) -> Optional[DeviceStatus]:
    """Decode v2 identity and live audio level records."""
    records_start = 14
    record_hdr_len = 6
    if len(frame) < records_start + 2:
        return None

    end = len(frame) - 2
    i = records_start
    matched = False

    while i + record_hdr_len <= end:
        tag = frame[i]
        index = frame[i + 1]
        length = frame[i + 5]
        data_start = i + record_hdr_len
        data_end = data_start + length
        if data_end > end:
            break

        data = frame[data_start:data_end]

        if tag == 0x01 and len(data) > 4:
            fw_str = f"{data[3]:02d}.{data[2]:02d}.{data[1]:02d}.{data[0]:02d}"
            serial = parse_ascii_string(data[4:])
            if index == 0:
                if status.rx is None:
                    status.rx = RxInfo()
                status.rx.serial = serial
                status.rx.firmware = fw_str
            elif index in (1, 2):
                idx = index - 1
                if status.tx[idx] is None:
                    status.tx[idx] = TxInfo(unit=index)
                status.tx[idx].serial = serial
                status.tx[idx].firmware = fw_str
            matched = True

        elif tag == 0x04 and len(data) > 0:
            bt_id = parse_ascii_string(data)
            if index == 0:
                if status.rx is None:
                    status.rx = RxInfo()
                status.rx.bluetooth_id = bt_id
            elif index in (1, 2):
                idx = index - 1
                if status.tx[idx] is None:
                    status.tx[idx] = TxInfo(unit=index)
                status.tx[idx].bluetooth_id = bt_id
            matched = True

        elif tag == 0x05 and len(data) == 1 and index in (1, 2):
            idx = index - 1
            if status.tx[idx] is None:
                status.tx[idx] = TxInfo(unit=index)
            status.tx[idx].level = data[0]
            matched = True

        elif tag == 0x06 and len(data) > 0:
            prod = parse_ascii_string(data)
            if index == 0:
                if status.rx is None:
                    status.rx = RxInfo()
                status.rx.product_name = prod
            elif index in (1, 2):
                idx = index - 1
                if status.tx[idx] is None:
                    status.tx[idx] = TxInfo(unit=index)
                status.tx[idx].product_name = prod
            matched = True

        i = data_end

    return status if matched else None


def decode_version_response(status: DeviceStatus, frame: bytes) -> Optional[DeviceStatus]:
    """Decode 0x00/0x01 GetVersion response (e.g. 'DJI MIC MINI RX' + FW + HW revision)."""
    if len(frame) < 35:
        return None
    if (frame[8] & 0x80) == 0 or frame[9] != 0x00 or frame[10] != 0x01:
        return None
    payload = frame[11:-2]
    if len(payload) < 24:
        return None
    model_str = parse_ascii_string(payload[2:22])
    if status.rx is None:
        status.rx = RxInfo()
    if model_str:
        status.rx.hardware_model = model_str
    if len(payload) >= 30:
        fw = payload[22:26]
        hw = payload[26:30]
        if not status.rx.firmware:
            status.rx.firmware = f"{fw[3]:02d}.{fw[2]:02d}.{fw[1]:02d}.{fw[0]:02d}"
        status.rx.hardware_revision = f"{hw[0]:02d}.{hw[1]:02d}.{hw[2]:02d}.{hw[3]:02d}"
    return status


def decode_frame(status: DeviceStatus, frame: bytes) -> Optional[DeviceStatus]:
    """Process any valid DUML frame and update device status."""
    if len(frame) < 13:
        return None

    # Check if it's a GetVersion response (0x00/0x01)
    if (frame[8] & 0x80) != 0 and frame[9] == 0x00 and frame[10] == 0x01:
        return decode_version_response(status, frame)

    # Check auxiliary periodic frames (0x5B/0x05 and 0x04/0x74)
    if frame[8] == 0x00 and frame[9] == 0x5B and frame[10] == 0x05:
        status.aux_5b05_hex = frame[11:-2].hex()
        return status
    if frame[8] == 0x00 and frame[9] == 0x04 and frame[10] == 0x74:
        status.aux_0474_hex = frame[11:-2].hex()
        return status

    dialect = detect_dialect(frame)
    if dialect == "v1":
        return decode_v1_heartbeat(status, frame)
    elif dialect == "v2":
        res = decode_v2_status(status, frame)
        if res is not None:
            return res
        return decode_v2_records(status, frame)
    return None


def build_get_version_packet(seq: int = 0x0101) -> bytes:
    """Build read-only DUML 0x00/0x01 GetVersion request."""
    length = 13
    hdr = bytearray([SOF, length & 0xFF, ((length >> 8) & 0x03) | VERSION])
    hdr.append(crc8_dji(bytes(hdr)))
    hdr.extend([0x02, 0x5A, seq & 0xFF, (seq >> 8) & 0xFF, 0x40, 0x00, 0x01])
    crc = crc16_checksum(bytes(hdr), V2_INIT)
    hdr.extend([crc & 0xFF, (crc >> 8) & 0xFF])
    return bytes(hdr)


SETTINGS_SPEC: Dict[str, Dict[str, Any]] = {
    "noise-cancel": {
        "label": "Noise Cancel Mode",
        "target": "tx",
        "v1": 0x031D,
        "v2": 0x0037,
        "values": {"basic": 0x00, "strong": 0x01},
        "default": "basic",
    },
    "noise-cancel-power": {
        "label": "Noise Cancelling",
        "target": "tx",
        "v1": None,
        "v2": 0x0038,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "noise-cancel-button": {
        "label": "Toggle NC via TX Power Button",
        "target": "tx",
        "v1": None,
        "v2": 0x000F,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "low-cut": {
        "label": "Low Cut Filter",
        "target": "tx",
        "v1": 0x0303,
        "v2": 0x0003,
        "values": {"off": 0x00, "on": 0x01},
        "default": "off",
    },
    "stereo": {
        "label": "Audio Channels",
        "target": "rx",
        "v1": 0x0008,
        "v2": 0x0008,
        "values": {"mono": 0x00, "stereo": 0x02},
        "default": "mono",
    },
    "safety-track": {
        "label": "Safety Track (-6dB)",
        "target": "rx",
        "v1": 0x0021,
        "v2": 0x0021,
        "values": {"off": 0x00, "on": 0x01},
        "default": "off",
    },
    "clip-limiter": {
        "label": "Clipping Control",
        "target": "rx",
        "v1": 0x001E,
        "v2": 0x001E,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "auto-off-15m": {
        "label": "Auto Power Off RX (15 min)",
        "target": "rx",
        "v1": 0x0010,
        "v2": 0x0010,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "tx-auto-off-15m": {
        "label": "Auto Power Off TX (15 min)",
        "target": "tx",
        "v1": None,
        "v2": 0x0010,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "camera-power": {
        "label": "Power On/Off with Camera",
        "target": "rx",
        "v1": 0x0020,
        "v2": 0x0020,
        "values": {"off": 0x00, "on": 0x01},
        "default": "on",
    },
    "plug-free": {
        "label": "Plug-Free External Speaker",
        "target": "rx",
        "v1": 0x0023,
        "v2": 0x0023,
        "values": {"off": 0x00, "on": 0x01},
        "default": "off",
    },
    "mic-leds": {
        "label": "Transmitter LEDs",
        "target": "tx",
        "v1": 0x030A,
        "v2": 0x000A,
        "values": {"on": 0x00, "off": 0x02},
        "default": "on",
    },
    "voice-tone": {
        "label": "Voice Tone",
        "target": "tx_unit",
        "v1": None,
        "v2": 0x0029,
        "values": {"standard": 0x00, "rich": 0x01, "bright": 0x02},
        "default": "standard",
    },
}


def build_command_packet(
    dialect: str,
    seq: int,
    setting_id: str,
    value_str: str,
    tx_unit: Optional[int] = None,
) -> bytes:
    """Build a command frame for the requested setting."""
    spec = SETTINGS_SPEC.get(setting_id)
    if not spec:
        raise ValueError(f"Unknown setting: {setting_id}")

    val_map = spec["values"]
    if value_str.lower() not in val_map:
        raise ValueError(
            f"Invalid value '{value_str}' for {setting_id}. Options: {list(val_map.keys())}"
        )

    wire_val = val_map[value_str.lower()]

    if dialect == "v1":
        cmd_id = spec["v1"]
        if cmd_id is None:
            raise ValueError(f"Setting '{setting_id}' is not modifiable over USB on v1 firmware")
        body = bytearray(
            [
                SOF,
                19,
                0x04,
                0x03,
                0x02,
                0x5A,
                seq & 0xFF,
                (seq >> 8) & 0xFF,
                0x40,
                0x5B,
                0x01,
                0x00,
                (cmd_id >> 8) & 0xFF,
                cmd_id & 0xFF,
                0x00,
                0x01,
                wire_val,
            ]
        )
        crc = crc16_bytes(bytes(body), V1_INIT, V1_RESIDUE)
        return bytes(body + crc)
    else:
        cmd_id = spec["v2"]
        target_type = spec["target"]
        if target_type == "rx":
            target = 0x0000
        elif target_type == "tx":
            target = 0xFFFF
        elif target_type == "tx_unit":
            if tx_unit not in (1, 2):
                raise ValueError("Target TX unit must be 1 or 2 for this setting")
            target = tx_unit
        else:
            target = 0xFFFF

        body = bytearray(
            [
                SOF,
                22,
                0x04,
                0xFC,
                0x02,
                0x5A,
                seq & 0xFF,
                (seq >> 8) & 0xFF,
                0x40,
                0x5B,
                0x01,
                0x02,
                target & 0xFF,
                (target >> 8) & 0xFF,
                0x00,
                0x00,
                cmd_id & 0xFF,
                (cmd_id >> 8) & 0xFF,
                0x01,
                wire_val,
            ]
        )
        crc = crc16_bytes(bytes(body), V2_INIT, V2_RESIDUE)
        return bytes(body + crc)
