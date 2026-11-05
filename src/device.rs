use crate::protocol::{
    apply_frame_to_status, build_command_packet, build_get_version_packet, take_frames,
    DeviceStatus, DumlFrame, CMD_ID_GET_VERSION, CMD_SET_GENERAL, CMD_SET_MIC,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{c_int, c_uchar, c_uint, c_void, CStr};
use std::fs;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

pub const VENDOR_ID: u16 = 0x2CA3;
pub const PRODUCT_ID: u16 = 0x4011;
pub const INTERFACE_NUM: c_int = 6;
pub const EP_OUT: c_uchar = 0x06;
pub const EP_IN: c_uchar = 0x86;

pub const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/99-dji-mic.rules";
pub const UDEV_RULE_CONTENT: &str = r#"# DJI Mic Mini / Mic 2 / Mic 3 — non-root USB (libusb) and HID (/dev/hidraw*) access
SUBSYSTEM=="usb", ATTR{idVendor}=="2ca3", ATTR{idProduct}=="4011", MODE="0666", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="2ca3", ATTRS{idProduct}=="4011", MODE="0666", TAG+="uaccess"
"#;

pub const UDEV_INSTALL_ONELINER: &str = r#"sudo tee /etc/udev/rules.d/99-dji-mic.rules >/dev/null <<'EOF'
# DJI Mic Mini / Mic 2 / Mic 3 — non-root USB (libusb) and HID (/dev/hidraw*) access
SUBSYSTEM=="usb", ATTR{idVendor}=="2ca3", ATTR{idProduct}=="4011", MODE="0666", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="2ca3", ATTRS{idProduct}=="4011", MODE="0666", TAG+="uaccess"
EOF
sudo udevadm control --reload-rules && sudo udevadm trigger --subsystem-match=usb --attr-match=idVendor=2ca3 && sudo udevadm trigger --subsystem-match=hidraw"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbSysfsInfo {
    pub present: bool,
    pub vid_pid: String,
    pub sysfs_path: String,
    pub bus_num: i32,
    pub dev_num: i32,
    pub port_path: String,
    pub usb_version: String,
    pub speed_mbps: String,
    pub max_power: String,
    pub manufacturer: String,
    pub product: String,
    pub usb_serial: String,
    pub hidraw_node: String,
    pub hid_phys: String,
    pub usbfs_node: String,
}

impl Default for UsbSysfsInfo {
    fn default() -> Self {
        Self {
            present: false,
            vid_pid: "2ca3:4011".to_string(),
            sysfs_path: String::new(),
            bus_num: 0,
            dev_num: 0,
            port_path: String::new(),
            usb_version: String::new(),
            speed_mbps: String::new(),
            max_power: String::new(),
            manufacturer: String::new(),
            product: String::new(),
            usb_serial: String::new(),
            hidraw_node: String::new(),
            hid_phys: String::new(),
            usbfs_node: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdevStatus {
    pub rule_path: String,
    pub rule_installed: bool,
    pub rule_content_valid: bool,
    pub usbfs_node: String,
    pub usbfs_writable: bool,
    pub hidraw_node: String,
    pub hidraw_writable: bool,
    pub ready: bool,
    pub summary: String,
    pub install_command: String,
}

impl Default for UdevStatus {
    fn default() -> Self {
        Self {
            rule_path: UDEV_RULE_PATH.to_string(),
            rule_installed: false,
            rule_content_valid: false,
            usbfs_node: String::new(),
            usbfs_writable: false,
            hidraw_node: String::new(),
            hidraw_writable: false,
            ready: false,
            summary: "Checking udev status...".to_string(),
            install_command: UDEV_INSTALL_ONELINER.to_string(),
        }
    }
}

fn read_trimmed(p: &Path) -> String {
    fs::read_to_string(p)
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn get_usb_sysfs_info() -> UsbSysfsInfo {
    let mut info = UsbSysfsInfo::default();
    let usb_root = Path::new("/sys/bus/usb/devices");
    let Ok(entries) = fs::read_dir(usb_root) else {
        return info;
    };

    for entry in entries.flatten() {
        let dev_dir = entry.path();
        if !dev_dir.is_dir() {
            continue;
        }
        let vid = read_trimmed(&dev_dir.join("idVendor")).to_lowercase();
        let pid = read_trimmed(&dev_dir.join("idProduct")).to_lowercase();
        if vid == "2ca3" && pid == "4011" {
            info.present = true;
            info.sysfs_path = fs::canonicalize(&dev_dir)
                .unwrap_or_else(|_| dev_dir.clone())
                .to_string_lossy()
                .to_string();
            info.port_path = entry.file_name().to_string_lossy().to_string();
            info.bus_num = read_trimmed(&dev_dir.join("busnum")).parse().unwrap_or(0);
            info.dev_num = read_trimmed(&dev_dir.join("devnum")).parse().unwrap_or(0);
            info.usb_version = read_trimmed(&dev_dir.join("version"));
            let spd = read_trimmed(&dev_dir.join("speed"));
            info.speed_mbps = if spd.is_empty() {
                String::new()
            } else {
                format!("{} Mbps", spd)
            };
            info.max_power = read_trimmed(&dev_dir.join("bMaxPower"));
            info.manufacturer = read_trimmed(&dev_dir.join("manufacturer"));
            info.product = read_trimmed(&dev_dir.join("product"));
            info.usb_serial = read_trimmed(&dev_dir.join("serial"));
            if info.bus_num > 0 && info.dev_num > 0 {
                info.usbfs_node = format!("/dev/bus/usb/{:03}/{:03}", info.bus_num, info.dev_num);
            }

            if let Ok(sub_entries) = fs::read_dir(&dev_dir) {
                for sub in sub_entries.flatten() {
                    let sub_path = sub.path();
                    if let Ok(sub2_entries) = fs::read_dir(&sub_path) {
                        for sub2 in sub2_entries.flatten() {
                            let p2 = sub2.path();
                            let hidraw_dir = p2.join("hidraw");
                            if hidraw_dir.is_dir() {
                                if let Ok(hr_entries) = fs::read_dir(&hidraw_dir) {
                                    for hr in hr_entries.flatten() {
                                        info.hidraw_node = format!(
                                            "/dev/{}",
                                            hr.file_name().to_string_lossy()
                                        );
                                    }
                                }
                                let uev = p2.join("uevent");
                                if uev.is_file() {
                                    for line in read_trimmed(&uev).lines() {
                                        if let Some(rest) = line.strip_prefix("HID_PHYS=") {
                                            info.hid_phys = rest.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            break;
        }
    }
    info
}

fn check_node_writable(path: &str) -> bool {
    if path.is_empty() || !Path::new(path).exists() {
        return false;
    }
    unsafe {
        let c_path = std::ffi::CString::new(path).unwrap_or_default();
        libc::access(c_path.as_ptr(), libc::R_OK | libc::W_OK) == 0
    }
}

pub fn check_udev_status(usb_info: Option<&UsbSysfsInfo>) -> UdevStatus {
    let owned_info;
    let info = match usb_info {
        Some(i) => i,
        None => {
            owned_info = get_usb_sysfs_info();
            &owned_info
        }
    };

    let mut st = UdevStatus::default();
    let rpath = Path::new(UDEV_RULE_PATH);
    st.rule_installed = rpath.exists();
    if st.rule_installed {
        if let Ok(txt) = fs::read_to_string(rpath) {
            let lower = txt.to_lowercase();
            st.rule_content_valid = lower.contains("2ca3") && lower.contains("4011");
        }
    }

    st.usbfs_node = info.usbfs_node.clone();
    st.hidraw_node = info.hidraw_node.clone();
    if info.present {
        st.usbfs_writable = check_node_writable(&info.usbfs_node);
        st.hidraw_writable =
            info.hidraw_node.is_empty() || check_node_writable(&info.hidraw_node);
    }

    if st.rule_installed && st.rule_content_valid {
        if info.present && !st.usbfs_writable {
            st.ready = false;
            st.summary = "Rule installed, but USB node is not writable yet (run `sudo udevadm trigger` or replug receiver)".to_string();
        } else {
            st.ready = true;
            st.summary = if info.present {
                "udev rule installed & USB/HID permissions verified".to_string()
            } else {
                "udev rule installed & ready".to_string()
            };
        }
    } else if info.present && st.usbfs_writable {
        st.ready = false;
        st.summary = format!(
            "Temporary session access active, but {} is missing",
            UDEV_RULE_PATH
        );
    } else {
        st.ready = false;
        st.summary = format!(
            "udev rule missing ({}) — required for non-root USB telemetry",
            UDEV_RULE_PATH
        );
    }

    st
}

pub fn install_udev_rule_as_root() -> (bool, String) {
    if unsafe { libc::geteuid() } != 0 {
        return (
            false,
            format!(
                "Root privileges required to write {}. Run:\n  sudo dji-mic udev --install\nor:\n  {}",
                UDEV_RULE_PATH, UDEV_INSTALL_ONELINER
            ),
        );
    }

    if let Err(e) = fs::write(UDEV_RULE_PATH, UDEV_RULE_CONTENT) {
        return (false, format!("Failed to write {}: {}", UDEV_RULE_PATH, e));
    }
    let _ = Command::new("chmod")
        .args(["0644", UDEV_RULE_PATH])
        .status();
    let _ = Command::new("udevadm")
        .args(["control", "--reload-rules"])
        .status();
    let _ = Command::new("udevadm")
        .args([
            "trigger",
            "--subsystem-match=usb",
            "--attr-match=idVendor=2ca3",
        ])
        .status();
    let _ = Command::new("udevadm")
        .args(["trigger", "--subsystem-match=hidraw"])
        .status();

    (
        true,
        format!(
            "Installed {} and reloaded udev rules successfully.",
            UDEV_RULE_PATH
        ),
    )
}

pub fn query_hid_service_info(hidraw_node: &str) -> BTreeMap<String, i32> {
    let mut result = BTreeMap::new();
    if hidraw_node.is_empty() || !Path::new(hidraw_node).exists() {
        return result;
    }

    let Ok(file) = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(hidraw_node)
    else {
        return result;
    };

    use std::os::unix::io::AsRawFd;
    let fd = file.as_raw_fd();

    let mut drain = [0u8; 64];
    unsafe {
        while libc::read(fd, drain.as_mut_ptr() as *mut c_void, 64) > 0 {}
    }

    for &cmd in &[0x59u8, 0x6Au8] {
        let mut req = [0u8; 64];
        req[0] = 0x55;
        req[1] = cmd;
        let written = unsafe { libc::write(fd, req.as_ptr() as *const c_void, 64) };
        if written < 0 {
            continue;
        }

        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let pr = unsafe { libc::poll(&mut pfd, 1, 80) };
        if pr > 0 && (pfd.revents & libc::POLLIN) != 0 {
            let mut resp = [0u8; 64];
            let n = unsafe { libc::read(fd, resp.as_mut_ptr() as *mut c_void, 64) };
            if n >= 4 && resp[0] == 0x55 && resp[1] == cmd {
                if cmd == 0x59 {
                    result.insert("paired_tx_count".to_string(), resp[3] as i32);
                } else if cmd == 0x6A {
                    result.insert("active_tx_index".to_string(), resp[2] as i32);
                }
            }
        }
    }

    result
}

// libusb-1.0 FFI bindings
#[allow(non_camel_case_types)]
type libusb_context = c_void;
#[allow(non_camel_case_types)]
type libusb_device_handle = c_void;

#[link(name = "usb-1.0")]
extern "C" {
    fn libusb_init(ctx: *mut *mut libusb_context) -> c_int;
    fn libusb_exit(ctx: *mut libusb_context);
    fn libusb_open_device_with_vid_pid(
        ctx: *mut libusb_context,
        vendor_id: u16,
        product_id: u16,
    ) -> *mut libusb_device_handle;
    fn libusb_close(dev_handle: *mut libusb_device_handle);
    fn libusb_set_auto_detach_kernel_driver(
        dev_handle: *mut libusb_device_handle,
        enable: c_int,
    ) -> c_int;
    fn libusb_claim_interface(
        dev_handle: *mut libusb_device_handle,
        interface_number: c_int,
    ) -> c_int;
    fn libusb_release_interface(
        dev_handle: *mut libusb_device_handle,
        interface_number: c_int,
    ) -> c_int;
    fn libusb_bulk_transfer(
        dev_handle: *mut libusb_device_handle,
        endpoint: c_uchar,
        data: *mut c_uchar,
        length: c_int,
        actual_length: *mut c_int,
        timeout: c_uint,
    ) -> c_int;
    fn libusb_strerror(errcode: c_int) -> *const libc::c_char;
}

const LIBUSB_ERROR_ACCESS: c_int = -3;
const LIBUSB_ERROR_NO_DEVICE: c_int = -4;
const LIBUSB_ERROR_TIMEOUT: c_int = -7;

fn usb_strerror(rc: c_int) -> String {
    unsafe {
        let p = libusb_strerror(rc);
        if p.is_null() {
            format!("libusb error {}", rc)
        } else {
            CStr::from_ptr(p).to_string_lossy().to_string()
        }
    }
}

pub struct DjiMicDevice {
    ctx: *mut libusb_context,
    handle: *mut libusb_device_handle,
    claimed: bool,
    rx_buf: Vec<u8>,
    seq: u16,
    pub status: DeviceStatus,
}

unsafe impl Send for DjiMicDevice {}

impl Default for DjiMicDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl DjiMicDevice {
    pub fn new() -> Self {
        Self {
            ctx: ptr::null_mut(),
            handle: ptr::null_mut(),
            claimed: false,
            rx_buf: Vec::new(),
            seq: 1,
            status: DeviceStatus::default(),
        }
    }

    pub fn is_open(&self) -> bool {
        !self.handle.is_null() && self.claimed
    }

    fn next_seq(&mut self) -> u16 {
        self.seq = (self.seq + 1) & 0xFFFF;
        if self.seq == 0 {
            self.seq = 1;
        }
        self.seq
    }

    pub fn open(&mut self) -> Result<(), String> {
        if self.is_open() {
            return Ok(());
        }

        unsafe {
            let mut ctx: *mut libusb_context = ptr::null_mut();
            let rc = libusb_init(&mut ctx);
            if rc != 0 {
                return Err(format!("libusb_init failed: {}", usb_strerror(rc)));
            }
            self.ctx = ctx;

            let handle = libusb_open_device_with_vid_pid(self.ctx, VENDOR_ID, PRODUCT_ID);
            if handle.is_null() {
                let sysfs = get_usb_sysfs_info();
                self.close();
                if sysfs.present {
                    return Err(format!(
                        "Permission denied opening DJI Mic Mini ({}) — install udev rule: {}",
                        sysfs.usbfs_node, UDEV_RULE_PATH
                    ));
                }
                return Err("DJI Mic Mini USB-C Receiver (2ca3:4011) not found.".to_string());
            }
            self.handle = handle;
            let _ = libusb_set_auto_detach_kernel_driver(self.handle, 1);

            let rc = libusb_claim_interface(self.handle, INTERFACE_NUM);
            if rc != 0 {
                let msg = usb_strerror(rc);
                self.close();
                if rc == LIBUSB_ERROR_ACCESS {
                    return Err(format!(
                        "Access denied claiming USB Interface {}: {}",
                        INTERFACE_NUM, msg
                    ));
                }
                return Err(format!(
                    "Failed to claim USB Interface {}: {}",
                    INTERFACE_NUM, msg
                ));
            }
            self.claimed = true;
            self.status.connected = true;
        }

        let _ = self.poll_frames(80);
        let _ = self.query_rx_version();
        Ok(())
    }

    pub fn close(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                if self.claimed {
                    let _ = libusb_release_interface(self.handle, INTERFACE_NUM);
                    self.claimed = false;
                }
                libusb_close(self.handle);
                self.handle = ptr::null_mut();
            }
            if !self.ctx.is_null() {
                libusb_exit(self.ctx);
                self.ctx = ptr::null_mut();
            }
        }
        self.status.connected = false;
    }

    pub fn write(&mut self, data: &[u8], timeout_ms: u32) -> Result<usize, String> {
        if !self.is_open() {
            return Err("DJI Mic Mini is not open".to_string());
        }
        let mut buf = data.to_vec();
        let mut transferred: c_int = 0;
        let rc = unsafe {
            libusb_bulk_transfer(
                self.handle,
                EP_OUT,
                buf.as_mut_ptr(),
                buf.len() as c_int,
                &mut transferred,
                timeout_ms,
            )
        };
        if rc == LIBUSB_ERROR_NO_DEVICE {
            self.close();
            return Err("DJI Mic Mini disconnected during write".to_string());
        }
        if rc != 0 {
            return Err(format!("Bulk write failed: {}", usb_strerror(rc)));
        }
        Ok(transferred as usize)
    }

    pub fn poll_frames(&mut self, timeout_ms: u32) -> Result<Vec<DumlFrame>, String> {
        if !self.is_open() {
            return Err("DJI Mic Mini is not open".to_string());
        }
        let mut buf = [0u8; 512];
        let mut transferred: c_int = 0;
        let rc = unsafe {
            libusb_bulk_transfer(
                self.handle,
                EP_IN,
                buf.as_mut_ptr(),
                buf.len() as c_int,
                &mut transferred,
                timeout_ms,
            )
        };
        if rc == LIBUSB_ERROR_NO_DEVICE {
            self.close();
            return Err("DJI Mic Mini disconnected".to_string());
        }
        if rc != 0 && rc != LIBUSB_ERROR_TIMEOUT {
            return Ok(Vec::new());
        }
        if transferred > 0 {
            self.rx_buf
                .extend_from_slice(&buf[..(transferred as usize)]);
        }
        let frames = take_frames(&mut self.rx_buf);
        for f in &frames {
            apply_frame_to_status(f, &mut self.status);
        }
        Ok(frames)
    }

    pub fn query_rx_version(&mut self) -> bool {
        let dialect = self.status.dialect.clone();
        let dialects = if dialect == "v1" {
            ["v1", "v2"]
        } else {
            ["v2", "v1"]
        };
        for d in dialects {
            let seq = self.next_seq();
            let pkt = build_get_version_packet(seq, d);
            if self.write(&pkt, 150).is_err() {
                continue;
            }
            let deadline = Instant::now() + Duration::from_millis(250);
            while Instant::now() < deadline {
                if let Ok(frames) = self.poll_frames(50) {
                    for f in frames {
                        if f.cmd_set == CMD_SET_GENERAL && f.cmd_id == CMD_ID_GET_VERSION {
                            return true;
                        }
                    }
                } else {
                    break;
                }
            }
        }
        false
    }

    pub fn collect_snapshot(&mut self, duration_ms: u64) -> DeviceStatus {
        let deadline = Instant::now() + Duration::from_millis(duration_ms);
        while Instant::now() < deadline {
            let _ = self.poll_frames(40);
            if self.status.heartbeat_count >= 1
                && self
                    .status
                    .rx
                    .as_ref()
                    .and_then(|r| r.serial.as_ref())
                    .is_some()
            {
                break;
            }
        }
        self.status.clone()
    }

    pub fn set_setting(
        &mut self,
        setting: &str,
        value: &str,
        tx_unit: Option<i32>,
    ) -> Result<bool, String> {
        if self.status.heartbeat_count == 0 {
            let _ = self.poll_frames(150);
        }
        let primary = self.status.dialect.clone();
        let dialects = if primary == "v1" {
            ["v1", "v2"]
        } else {
            ["v2", "v1"]
        };

        let mut acked = false;
        for d in dialects {
            let seq = self.next_seq();
            let pkt = build_command_packet(setting, value, seq, d, tx_unit)?;
            self.write(&pkt, 200)?;

            let deadline = Instant::now() + Duration::from_millis(280);
            while Instant::now() < deadline {
                let frames = self.poll_frames(40)?;
                for f in frames {
                    if f.cmd_set == CMD_SET_MIC && f.seq == seq {
                        acked = true;
                        break;
                    }
                }
                if acked {
                    break;
                }
            }
            if acked {
                break;
            }
        }

        let settle_deadline = Instant::now() + Duration::from_millis(320);
        while Instant::now() < settle_deadline {
            let _ = self.poll_frames(45);
        }
        Ok(acked)
    }

    pub fn send_raw_command(
        &mut self,
        cmd_set: u8,
        cmd_id: u8,
        payload: &[u8],
        dialect: Option<&str>,
        sender: u8,
        receiver: u8,
        pack_type: u8,
    ) -> Result<Vec<DumlFrame>, String> {
        let d = dialect.unwrap_or(&self.status.dialect).to_string();
        let seq = self.next_seq();
        let pkt = crate::protocol::build_frame(
            cmd_set, cmd_id, payload, seq, &d, sender, receiver, pack_type,
        );
        self.write(&pkt, 200)?;
        let mut replies = Vec::new();
        let deadline = Instant::now() + Duration::from_millis(350);
        while Instant::now() < deadline {
            let frames = self.poll_frames(50)?;
            replies.extend(frames);
            thread::sleep(Duration::from_millis(10));
        }
        Ok(replies)
    }
}

impl Drop for DjiMicDevice {
    fn drop(&mut self) {
        self.close();
    }
}
