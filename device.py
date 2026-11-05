"""
DJI Mic Mini USB communication layer using libusb-1.0 via ctypes,
plus Linux sysfs and HID interface discovery.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import glob
import os
import select
import subprocess
import time
from dataclasses import asdict, dataclass
from typing import Any, Dict, List, Optional

from protocol import (
    DeviceStatus,
    build_command_packet,
    build_get_version_packet,
    decode_frame,
    detect_dialect,
    take_frames,
)

DJI_VENDOR_ID = 0x2CA3
DJI_PRODUCT_ID = 0x4011

DJI_INTERFACE = 6
EP_BULK_OUT = 0x06
EP_BULK_IN = 0x86

UDEV_RULE_PATH = "/etc/udev/rules.d/99-dji-mic.rules"
UDEV_RULE_CONTENT = (
    "# DJI Mic Mini - non-root USB access\n"
    'SUBSYSTEM=="usb", ATTRS{idVendor}=="2ca3", ATTRS{idProduct}=="4011", MODE="0666", TAG+="uaccess"\n'
)
UDEV_INSTALL_ONELINER = (
    "echo 'SUBSYSTEM==\"usb\", ATTRS{idVendor}==\"2ca3\", ATTRS{idProduct}==\"4011\", MODE=\"0666\", TAG+=\"uaccess\"' "
    "| sudo tee /etc/udev/rules.d/99-dji-mic.rules && "
    "sudo udevadm control --reload-rules && "
    "sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=2ca3"
)

SAFE_HID_COMMANDS = frozenset({0x59, 0x5E, 0x6A})

_lib_name = ctypes.util.find_library("usb-1.0") or "libusb-1.0.so.0"
try:
    _lib = ctypes.CDLL(_lib_name)
except OSError:
    _lib = None


class DeviceDescriptor(ctypes.Structure):
    _fields_ = [
        ("bLength", ctypes.c_uint8),
        ("bDescriptorType", ctypes.c_uint8),
        ("bcdUSB", ctypes.c_uint16),
        ("bDeviceClass", ctypes.c_uint8),
        ("bDeviceSubClass", ctypes.c_uint8),
        ("bDeviceProtocol", ctypes.c_uint8),
        ("bMaxPacketSize0", ctypes.c_uint8),
        ("idVendor", ctypes.c_uint16),
        ("idProduct", ctypes.c_uint16),
        ("bcdDevice", ctypes.c_uint16),
        ("iManufacturer", ctypes.c_uint8),
        ("iProduct", ctypes.c_uint8),
        ("iSerialNumber", ctypes.c_uint8),
        ("bNumConfigurations", ctypes.c_uint8),
    ]


if _lib:
    _lib.libusb_init.argtypes = [ctypes.POINTER(ctypes.c_void_p)]
    _lib.libusb_init.restype = ctypes.c_int

    _lib.libusb_exit.argtypes = [ctypes.c_void_p]
    _lib.libusb_exit.restype = None

    _lib.libusb_get_device_list.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.POINTER(ctypes.c_void_p)),
    ]
    _lib.libusb_get_device_list.restype = ctypes.c_ssize_t

    _lib.libusb_free_device_list.argtypes = [ctypes.POINTER(ctypes.c_void_p), ctypes.c_int]
    _lib.libusb_free_device_list.restype = None

    _lib.libusb_get_device_descriptor.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(DeviceDescriptor),
    ]
    _lib.libusb_get_device_descriptor.restype = ctypes.c_int

    _lib.libusb_open.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_void_p)]
    _lib.libusb_open.restype = ctypes.c_int

    _lib.libusb_close.argtypes = [ctypes.c_void_p]
    _lib.libusb_close.restype = None

    _lib.libusb_claim_interface.argtypes = [ctypes.c_void_p, ctypes.c_int]
    _lib.libusb_claim_interface.restype = ctypes.c_int

    _lib.libusb_release_interface.argtypes = [ctypes.c_void_p, ctypes.c_int]
    _lib.libusb_release_interface.restype = ctypes.c_int

    _lib.libusb_bulk_transfer.argtypes = [
        ctypes.c_void_p,
        ctypes.c_uint8,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_int,
        ctypes.POINTER(ctypes.c_int),
        ctypes.c_uint,
    ]
    _lib.libusb_bulk_transfer.restype = ctypes.c_int

    _lib.libusb_strerror.argtypes = [ctypes.c_int]
    _lib.libusb_strerror.restype = ctypes.c_char_p


class DjiMicException(Exception):
    pass


class DjiMicPermissionError(DjiMicException):
    pass


class DjiMicNotFoundError(DjiMicException):
    pass


@dataclass
class UsbSysfsInfo:
    present: bool = False
    vid_pid: str = "2ca3:4011"
    sysfs_path: str = ""
    bus_num: int = 0
    dev_num: int = 0
    port_path: str = ""
    usb_version: str = ""
    speed_mbps: str = ""
    max_power: str = ""
    manufacturer: str = ""
    product: str = ""
    usb_serial: str = ""
    hidraw_node: str = ""
    hid_phys: str = ""
    usbfs_node: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def read_sysfs_file(path: str) -> str:
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return f.read().strip()
    except Exception:
        return ""


def get_usb_sysfs_info() -> UsbSysfsInfo:
    """Inspect Linux /sys/bus/usb/devices and /sys/class/hidraw for DJI Mic Mini (2ca3:4011)."""
    info = UsbSysfsInfo()
    for p in sorted(glob.glob("/sys/bus/usb/devices/*")):
        if ":" in os.path.basename(p):
            continue
        vid = read_sysfs_file(os.path.join(p, "idVendor")).lower()
        pid = read_sysfs_file(os.path.join(p, "idProduct")).lower()
        if vid == "2ca3" and pid == "4011":
            info.present = True
            info.sysfs_path = os.path.realpath(p)
            info.port_path = os.path.basename(p)
            try:
                info.bus_num = int(read_sysfs_file(os.path.join(p, "busnum")) or "0")
                info.dev_num = int(read_sysfs_file(os.path.join(p, "devnum")) or "0")
            except ValueError:
                pass
            if info.bus_num and info.dev_num:
                info.usbfs_node = f"/dev/bus/usb/{info.bus_num:03d}/{info.dev_num:03d}"
            info.usb_version = read_sysfs_file(os.path.join(p, "version"))
            spd = read_sysfs_file(os.path.join(p, "speed"))
            info.speed_mbps = f"{spd} Mbps" if spd else ""
            info.max_power = read_sysfs_file(os.path.join(p, "bMaxPower"))
            info.manufacturer = read_sysfs_file(os.path.join(p, "manufacturer"))
            info.product = read_sysfs_file(os.path.join(p, "product"))
            info.usb_serial = read_sysfs_file(os.path.join(p, "serial"))
            break

    for hp in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        uevent = read_sysfs_file(os.path.join(hp, "device", "uevent"))
        if "00002CA3:00004011" in uevent.upper():
            info.hidraw_node = f"/dev/{os.path.basename(hp)}"
            for line in uevent.splitlines():
                if line.startswith("HID_PHYS="):
                    info.hid_phys = line.split("=", 1)[1].strip()
            break

    return info


@dataclass
class UdevStatus:
    rule_installed: bool = False
    rule_path: str = UDEV_RULE_PATH
    rule_content_valid: bool = False
    usbfs_node: str = ""
    usbfs_writable: bool = False
    hidraw_node: str = ""
    hidraw_writable: bool = False
    ready: bool = False
    summary: str = ""
    install_command: str = UDEV_INSTALL_ONELINER

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


def check_udev_status(sysfs_info: Optional[UsbSysfsInfo] = None) -> UdevStatus:
    """Check whether the udev rule for 2ca3:4011 is installed and USB device permissions are active."""
    if sysfs_info is None:
        sysfs_info = get_usb_sysfs_info()

    st = UdevStatus(
        usbfs_node=sysfs_info.usbfs_node,
        hidraw_node=sysfs_info.hidraw_node,
    )

    rule_dirs = ["/etc/udev/rules.d", "/var/run/host/etc/udev/rules.d"]
    direct_paths = [
        UDEV_RULE_PATH,
        "/var/run/host/etc/udev/rules.d/99-dji-mic.rules",
    ]
    found_direct = False
    for dp in direct_paths:
        if os.path.exists(dp):
            st.rule_installed = True
            st.rule_path = UDEV_RULE_PATH
            txt = read_sysfs_file(dp).lower()
            st.rule_content_valid = (
                "2ca3" in txt and "4011" in txt and ("0666" in txt or "uaccess" in txt)
            )
            found_direct = True
            break

    if not found_direct:
        for rdir in rule_dirs:
            for rp in sorted(glob.glob(os.path.join(rdir, "*.rules"))):
                txt = read_sysfs_file(rp).lower()
                if "2ca3" in txt and "4011" in txt:
                    st.rule_installed = True
                    st.rule_path = rp.replace("/var/run/host", "")
                    st.rule_content_valid = "0666" in txt or "uaccess" in txt
                    break
            if st.rule_installed:
                break

    if st.usbfs_node and os.path.exists(st.usbfs_node):
        st.usbfs_writable = os.access(st.usbfs_node, os.R_OK | os.W_OK)

    if st.hidraw_node and os.path.exists(st.hidraw_node):
        st.hidraw_writable = os.access(st.hidraw_node, os.R_OK | os.W_OK)

    if st.rule_installed and st.rule_content_valid:
        if not sysfs_info.present:
            st.ready = True
            st.summary = f"Active ({st.rule_path}) — Ready for USB connection"
        elif st.usbfs_writable:
            st.ready = True
            st.summary = f"Active & Verified ({st.rule_path} • {st.usbfs_node} writable)"
        else:
            st.ready = False
            st.summary = (
                f"Rule installed ({st.rule_path}), but {st.usbfs_node} is not writable yet "
                "(run udevadm trigger or replug receiver)"
            )
    else:
        st.ready = bool(sysfs_info.present and st.usbfs_writable)
        st.summary = (
            "Missing /etc/udev/rules.d/99-dji-mic.rules — non-root USB access requires rule installation"
        )

    return st


def install_udev_rule_as_root() -> bool:
    """Install /etc/udev/rules.d/99-dji-mic.rules and reload udev when running as root."""
    if os.geteuid() != 0:
        raise DjiMicPermissionError(
            "Installing /etc/udev/rules.d/99-dji-mic.rules requires root privileges.\n"
            f"Run:\n  sudo dji-mic udev --install\nor:\n  {UDEV_INSTALL_ONELINER}"
        )
    with open(UDEV_RULE_PATH, "w", encoding="utf-8") as f:
        f.write(UDEV_RULE_CONTENT)
    os.chmod(UDEV_RULE_PATH, 0o644)
    subprocess.run(["udevadm", "control", "--reload-rules"], check=False, timeout=5.0)
    subprocess.run(
        ["udevadm", "trigger", "--subsystem-match=usb", "--attr-match=idVendor=2ca3"],
        check=False,
        timeout=5.0,
    )
    return True


def query_hid_service_info(hidraw_node: str) -> Dict[str, Optional[int]]:
    """Query safe read-only HID report 0x55 commands (0x59 paired TX count, 0x6a index)."""
    res: Dict[str, Optional[int]] = {"paired_tx_count": None, "active_tx_index": None}
    if not hidraw_node or not os.path.exists(hidraw_node):
        return res
    fd = -1
    try:
        fd = os.open(hidraw_node, os.O_RDWR | os.O_NONBLOCK)
        for cmd, key in ((0x59, "paired_tx_count"), (0x6A, "active_tx_index")):
            if cmd not in SAFE_HID_COMMANDS:
                raise ValueError(f"Blocked unsafe HID command: 0x{cmd:02x}")
            pkt = bytes([0x55, cmd]) + bytes(62)
            os.write(fd, pkt)
            r, _, _ = select.select([fd], [], [], 0.08)
            if r:
                resp = os.read(fd, 64)
                if len(resp) >= 3 and resp[0] == 0x55 and resp[1] == cmd:
                    res[key] = resp[2]
    except Exception:
        pass
    finally:
        if fd >= 0:
            try:
                os.close(fd)
            except Exception:
                pass
    return res


class DjiMicDevice:
    """Manages connection, heartbeats, and commands to the DJI Mic Mini over USB Interface 6."""

    def __init__(self):
        if not _lib:
            raise DjiMicException("libusb-1.0 not found on the system")
        self.ctx = ctypes.c_void_p()
        self.handle = ctypes.c_void_p()
        self.seq = 1
        self.rx_buffer = bytearray()
        self.status = DeviceStatus()
        self.interface_claimed = False
        self.version_queried = False

    def __enter__(self) -> "DjiMicDevice":
        self.open()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        self.close()

    def is_open(self) -> bool:
        return bool(self.handle and self.interface_claimed)

    def open(self) -> None:
        """Find and open the DJI Mic Mini device."""
        self.close()
        ret = _lib.libusb_init(ctypes.byref(self.ctx))
        if ret != 0:
            raise DjiMicException(f"Failed to initialize libusb: error {ret}")

        dev_list = ctypes.POINTER(ctypes.c_void_p)()
        cnt = _lib.libusb_get_device_list(self.ctx, ctypes.byref(dev_list))
        if cnt < 0:
            _lib.libusb_exit(self.ctx)
            self.ctx = ctypes.c_void_p()
            raise DjiMicException("Failed to enumerate USB devices")

        found_dev = None
        for i in range(cnt):
            dev = dev_list[i]
            desc = DeviceDescriptor()
            _lib.libusb_get_device_descriptor(dev, ctypes.byref(desc))
            if desc.idVendor == DJI_VENDOR_ID and desc.idProduct == DJI_PRODUCT_ID:
                found_dev = dev
                break

        if not found_dev:
            _lib.libusb_free_device_list(dev_list, 1)
            _lib.libusb_exit(self.ctx)
            self.ctx = ctypes.c_void_p()
            raise DjiMicNotFoundError(
                "DJI Mic Mini not found. Make sure the receiver is plugged in via USB-C."
            )

        open_handle = ctypes.c_void_p()
        ret = _lib.libusb_open(found_dev, ctypes.byref(open_handle))
        _lib.libusb_free_device_list(dev_list, 1)

        if ret == -3:  # LIBUSB_ERROR_ACCESS
            _lib.libusb_exit(self.ctx)
            self.ctx = ctypes.c_void_p()
            raise DjiMicPermissionError(
                "Permission denied accessing DJI Mic Mini USB device (2ca3:4011).\n"
                "Ensure /etc/udev/rules.d/99-dji-mic.rules is installed."
            )
        elif ret != 0:
            err_msg = _lib.libusb_strerror(ret).decode("utf-8", errors="replace")
            _lib.libusb_exit(self.ctx)
            self.ctx = ctypes.c_void_p()
            raise DjiMicException(f"Failed to open DJI Mic Mini: {err_msg} ({ret})")

        self.handle = open_handle

        ret = _lib.libusb_claim_interface(self.handle, DJI_INTERFACE)
        if ret == -6:  # LIBUSB_ERROR_BUSY (claimed by dji-mic-tray)
            self.close()
            raise DjiMicException(
                "INTERFACE_BUSY: DJI Mic Mini interface 6 is currently claimed by dji-mic-tray."
            )
        elif ret != 0:
            err_msg = _lib.libusb_strerror(ret).decode("utf-8", errors="replace")
            self.close()
            raise DjiMicException(
                f"Failed to claim DJI Mic Mini interface {DJI_INTERFACE}: {err_msg} ({ret})"
            )

        self.interface_claimed = True
        self.version_queried = False

    def close(self) -> None:
        """Release interface and close USB connection."""
        if self.handle:
            if self.interface_claimed:
                try:
                    _lib.libusb_release_interface(self.handle, DJI_INTERFACE)
                except Exception:
                    pass
                self.interface_claimed = False
            try:
                _lib.libusb_close(self.handle)
            except Exception:
                pass
            self.handle = ctypes.c_void_p()

        if self.ctx:
            try:
                _lib.libusb_exit(self.ctx)
            except Exception:
                pass
            self.ctx = ctypes.c_void_p()

    def read_raw(self, timeout_ms: int = 120) -> bytes:
        """Perform a single bulk transfer read from EP 0x86."""
        if not self.handle:
            return b""
        buf_size = 512
        raw_buf = (ctypes.c_uint8 * buf_size)()
        transferred = ctypes.c_int(0)

        ret = _lib.libusb_bulk_transfer(
            self.handle,
            EP_BULK_IN,
            raw_buf,
            buf_size,
            ctypes.byref(transferred),
            timeout_ms,
        )

        if ret == 0 and transferred.value > 0:
            return bytes(raw_buf[: transferred.value])
        if ret in (-4, -1, -5):  # LIBUSB_ERROR_NO_DEVICE, IO, NOT_FOUND
            raise DjiMicNotFoundError("DJI Mic Mini USB receiver was disconnected.")
        return b""

    def write_raw(self, data: bytes, timeout_ms: int = 500) -> bool:
        """Perform a single bulk transfer write to EP 0x06."""
        if not self.handle:
            return False
        raw_buf = (ctypes.c_uint8 * len(data))(*data)
        transferred = ctypes.c_int(0)

        ret = _lib.libusb_bulk_transfer(
            self.handle,
            EP_BULK_OUT,
            raw_buf,
            len(data),
            ctypes.byref(transferred),
            timeout_ms,
        )

        if ret in (-4, -1, -5):
            raise DjiMicNotFoundError("DJI Mic Mini USB receiver was disconnected.")
        return ret == 0 and transferred.value == len(data)

    def request_version(self) -> bool:
        """Send read-only 0x00/0x01 GetVersion packet to retrieve hardware model & revision."""
        pkt = build_get_version_packet(0x0101)
        return self.write_raw(pkt, timeout_ms=250)

    def poll_frames(self, timeout_ms: int = 100) -> List[bytes]:
        """Read pending data and return any completed DUML frames."""
        if (
            self.status.connected
            and self.status.rx is not None
            and self.status.rx.hardware_revision is None
            and not self.version_queried
        ):
            self.version_queried = True
            try:
                self.request_version()
            except Exception:
                pass

        chunk = self.read_raw(timeout_ms)
        if chunk:
            self.rx_buffer.extend(chunk)
            frames = take_frames(self.rx_buffer)
            for f in frames:
                dialect = detect_dialect(f)
                if dialect:
                    self.status.dialect = dialect
                decode_frame(self.status, f)
            return frames
        return []

    def sync_status(self, duration_s: float = 0.8) -> DeviceStatus:
        """Listen to incoming frames for duration_s to build fresh state."""
        sysfs = get_usb_sysfs_info()
        if sysfs.hidraw_node:
            hid_info = query_hid_service_info(sysfs.hidraw_node)
            self.status.paired_tx_count_hid = hid_info.get("paired_tx_count")
            self.status.active_tx_index_hid = hid_info.get("active_tx_index")

        start = time.time()
        while time.time() - start < duration_s:
            self.poll_frames(timeout_ms=80)
            if (
                self.status.connected
                and self.status.rx is not None
                and self.status.rx.hardware_revision is not None
                and time.time() - start >= 0.35
            ):
                break
            time.sleep(0.015)
        return self.status

    def set_setting(
        self,
        setting_id: str,
        value: str,
        tx_unit: Optional[int] = None,
        timeout_s: float = 1.5,
    ) -> bool:
        """Send a setting command and wait for ACK."""
        if not self.status.connected:
            self.sync_status(0.4)

        dialect = self.status.dialect or "v1"
        seq = self.seq
        self.seq = (self.seq + 1) & 0xFFFF
        if self.seq == 0:
            self.seq = 1

        cmd_pkt = build_command_packet(dialect, seq, setting_id, value, tx_unit=tx_unit)

        if not self.write_raw(cmd_pkt):
            raise DjiMicException(f"Failed to write command packet for {setting_id}")

        start = time.time()
        ack_received = False

        while time.time() - start < timeout_s:
            frames = self.poll_frames(timeout_ms=80)
            for f in frames:
                if len(f) >= 13 and (f[8] & 0x80) != 0 and f[9] == 0x5B and f[10] == 0x01:
                    ack_seq = f[6] | (f[7] << 8)
                    if ack_seq == seq:
                        ack_received = True
                        break
            if ack_received:
                break
            time.sleep(0.01)

        time.sleep(0.15)
        self.poll_frames(timeout_ms=80)
        return ack_received
