use crate::audio::{
    get_alsa_card_info, get_alsa_volume, get_noise_learn_live_state, get_pipewire_stream_details,
    maybe_run_adaptive_micro_learn, recompute_profile_for_strength, reset_learned_noise_profile,
    set_alsa_mute, set_alsa_volume, sync_pipewire_ai_virtual_mic, trigger_noise_learning_async,
    AlsaCardInfo, AlsaVolumeInfo, PipewireStreamInfo,
};
use crate::device::{
    check_udev_status, get_usb_sysfs_info, query_hid_service_info, DjiMicDevice, UdevStatus,
    UsbSysfsInfo, UDEV_INSTALL_ONELINER,
};
use crate::protocol::{find_setting_spec, DeviceStatus, RxInfo, TxInfo};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ui_zoom_percent: i32,
    pub color_scheme: String,
    pub alert_on_rx_disconnect: bool,
    pub alert_on_tx_disconnect: bool,
    pub alert_on_low_battery: bool,
    pub low_battery_threshold_pct: i32,
    #[serde(default = "default_true")]
    pub start_on_login: bool,
    #[serde(default = "default_true")]
    pub start_minimized: bool,
    #[serde(default = "default_ai_nc_mode")]
    pub ai_nc_mode: String,
    #[serde(default = "default_ai_nc_strength")]
    pub ai_nc_strength_pct: i32,
    #[serde(default)]
    pub ai_nc_virtual_mic: bool,
}

fn default_true() -> bool {
    true
}

fn default_ai_nc_mode() -> String {
    "learned_ai".to_string()
}

fn default_ai_nc_strength() -> i32 {
    80
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui_zoom_percent: 100,
            color_scheme: "system".to_string(),
            alert_on_rx_disconnect: true,
            alert_on_tx_disconnect: true,
            alert_on_low_battery: true,
            low_battery_threshold_pct: 25,
            start_on_login: true,
            start_minimized: true,
            ai_nc_mode: "learned_ai".to_string(),
            ai_nc_strength_pct: 80,
            ai_nc_virtual_mic: false,
        }
    }
}

pub fn systemd_service_unit_content(start_minimized: bool) -> String {
    let flag = if start_minimized { " --minimized" } else { "" };
    format!(
        "[Unit]\n\
Description=KDE DJI Mic Mini Control (Rust) — System Tray Telemetry, Hardware Controller & Disconnection Alert Monitor\n\
After=graphical-session.target pipewire.service wireplumber.service\n\
PartOf=graphical-session.target\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart=%h/.local/bin/dji-mic-tray{flag}\n\
Restart=on-failure\n\
RestartSec=3\n\
Environment=QT_QUICK_CONTROLS_STYLE=org.kde.desktop\n\
\n\
[Install]\n\
WantedBy=default.target\n"
    )
}

pub fn read_start_minimized_config() -> bool {
    let cp = config_path();
    if let Ok(txt) = fs::read_to_string(&cp) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(v) = parsed.get("start_minimized").and_then(|v| v.as_bool()) {
                return v;
            }
        }
    }
    true
}

pub fn autostart_desktop_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("autostart")
        .join("dji-mic-tray.desktop")
}

pub fn systemd_user_unit_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("systemd")
        .join("user")
        .join("dji-mic-tray.service")
}

pub fn check_start_on_login_enabled() -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    let wants_link = PathBuf::from(&home)
        .join(".config/systemd/user/graphical-session.target.wants/dji-mic-tray.service");
    let default_wants_link = PathBuf::from(&home)
        .join(".config/systemd/user/default.target.wants/dji-mic-tray.service");
    if wants_link.exists() || default_wants_link.exists() || autostart_desktop_path().exists() {
        return true;
    }
    if let Ok(out) = Command::new("systemctl")
        .args(["--user", "is-enabled", "dji-mic-tray.service"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s == "enabled" || s == "enabled-runtime" {
            return true;
        }
    }
    false
}

pub fn apply_start_on_login(enabled: bool, start_minimized: bool) -> (bool, String) {
    let unit_path = systemd_user_unit_path();
    let auto_path = autostart_desktop_path();
    let min_flag = if start_minimized { " --minimized" } else { "" };
    if let Some(parent) = unit_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&unit_path, systemd_service_unit_content(start_minimized));
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    if enabled {
        let st = Command::new("systemctl")
            .args(["--user", "enable", "dji-mic-tray.service"])
            .status();
        if st.map(|s| s.success()).unwrap_or(false) {
            let _ = fs::remove_file(&auto_path);
            return (
                true,
                format!(
                    "Start on Login ENABLED (dji-mic-tray.service • Start Minimized: {})",
                    if start_minimized { "ON" } else { "OFF" }
                ),
            );
        }
        if let Some(parent) = auto_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
        let desktop_body = format!(
            "[Desktop Entry]\nType=Application\nName=KDE DJI Mic Mini Control\nComment=Start DJI Mic Mini System Tray Daemon on Login\nExec={home}/.local/bin/dji-mic-tray{min_flag}\nIcon=dji-mic-tray\nTerminal=false\nCategories=AudioVideo;Audio;Mixer;Utility;Qt;KDE;\nX-KDE-autostart-after=panel\n"
        );
        let _ = fs::write(&auto_path, desktop_body);
        (
            true,
            format!("Start on Login ENABLED ({})", auto_path.to_string_lossy()),
        )
    } else {
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "dji-mic-tray.service"])
            .status();
        let _ = fs::remove_file(&auto_path);
        (
            true,
            "Start on Login DISABLED (dji-mic-tray.service disabled)".to_string(),
        )
    }
}

pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("dji-mic-tray")
        .join("config.json")
}

pub fn status_json_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/run/user/{}/dji-mic-status.json", uid))
}

pub fn socket_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/run/user/{}/dji-mic-tray.sock", uid))
}

pub fn now_hms() -> String {
    if let Ok(out) = Command::new("date").arg("+%Y-%m-%d %H:%M:%S").output() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

type VoidCallback = extern "C" fn();

static CB_STATE_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
static CB_WINDOW_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
static CB_CONFIG_CHANGED: OnceLock<VoidCallback> = OnceLock::new();

fn emit_state_changed() {
    if let Some(cb) = CB_STATE_CHANGED.get() {
        cb();
    }
}
fn emit_window_changed() {
    if let Some(cb) = CB_WINDOW_CHANGED.get() {
        cb();
    }
}
fn emit_config_changed() {
    if let Some(cb) = CB_CONFIG_CHANGED.get() {
        cb();
    }
}

pub struct BackendState {
    pub ui_test_mode: bool,
    pub quitting: bool,
    pub window_visible: bool,
    pub active_tab_index: i32,
    pub last_action_message: String,
    pub event_log: Vec<String>,
    pub config: AppConfig,
    pub usb_info: UsbSysfsInfo,
    pub udev_status: UdevStatus,
    pub status: DeviceStatus,
    pub alsa_card: AlsaCardInfo,
    pub alsa_vol: AlsaVolumeInfo,
    pub pw_info: PipewireStreamInfo,
    pub last_slow_poll: Instant,
    pub initial_check_done: bool,
    pub prev_rx_connected: Option<bool>,
    pub prev_tx_count: Option<i32>,
    pub low_bat_alerted: [bool; 3],
}

pub static STATE: OnceLock<Arc<Mutex<BackendState>>> = OnceLock::new();
pub static USB_DEV: OnceLock<Arc<Mutex<Option<DjiMicDevice>>>> = OnceLock::new();

pub fn get_state() -> &'static Arc<Mutex<BackendState>> {
    STATE.get_or_init(|| {
        let mut cfg = AppConfig::default();
        let cp = config_path();
        if cp.exists() {
            if let Ok(txt) = fs::read_to_string(&cp) {
                if let Ok(loaded) = serde_json::from_str::<AppConfig>(&txt) {
                    cfg = loaded;
                }
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                    if val.get("start_on_login").is_none() {
                        cfg.start_on_login = check_start_on_login_enabled();
                    }
                }
            }
        } else {
            cfg.start_on_login = check_start_on_login_enabled();
        }
        cfg.ui_zoom_percent = cfg.ui_zoom_percent.clamp(75, 175);

        Arc::new(Mutex::new(BackendState {
            ui_test_mode: false,
            quitting: false,
            window_visible: true,
            active_tab_index: 0,
            last_action_message: String::new(),
            event_log: Vec::new(),
            config: cfg,
            usb_info: UsbSysfsInfo::default(),
            udev_status: UdevStatus::default(),
            status: DeviceStatus::default(),
            alsa_card: AlsaCardInfo::default(),
            alsa_vol: AlsaVolumeInfo::default(),
            pw_info: PipewireStreamInfo::default(),
            last_slow_poll: Instant::now() - Duration::from_secs(10),
            initial_check_done: false,
            prev_rx_connected: None,
            prev_tx_count: None,
            low_bat_alerted: [false, false, false],
        }))
    })
}

pub fn get_usb_dev() -> &'static Arc<Mutex<Option<DjiMicDevice>>> {
    USB_DEV.get_or_init(|| Arc::new(Mutex::new(None)))
}

fn save_config(st: &BackendState) {
    if st.ui_test_mode {
        return;
    }
    let cp = config_path();
    if let Some(parent) = cp.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(js) = serde_json::to_string_pretty(&st.config) {
        if fs::write(&cp, js).is_ok() {
            let _ = fs::set_permissions(&cp, fs::Permissions::from_mode(0o600));
        }
    }
}

fn log_event(st: &mut BackendState, msg: &str) {
    let line = format!("[{}] {}", now_hms(), msg);
    st.event_log.insert(0, line);
    if st.event_log.len() > 40 {
        st.event_log.truncate(40);
    }
}

pub const KDE_NOTIFYRC_COMPONENT: &str = "dji-mic-tray";

pub const KDE_NOTIFYRC_CONTENT: &str = "\
[Global]
IconName=dji-mic-tray
Comment=KDE DJI Mic Mini Control — System Tray Telemetry, Hardware Controller & Disconnection Alerts
Name=KDE DJI Mic Mini Control
DesktopEntry=dji-mic-tray

[Event/rxDisconnected]
Name=USB Receiver Disconnected Alert
Comment=Triggered when the DJI Mic Mini USB-C Receiver (2ca3:4011) is unplugged or disconnected
Action=Popup
Urgency=Critical

[Event/rxConnected]
Name=USB Receiver Connected
Comment=Triggered when the DJI Mic Mini USB-C Receiver connects on USB
Action=Popup
Urgency=Normal

[Event/txDisconnected]
Name=Wireless Transmitter Disconnected Alert
Comment=Triggered when all wireless microphone transmitters (TX1/TX2) power off or lose link
Action=Popup
Urgency=Critical

[Event/txConnected]
Name=Wireless Transmitter Connected
Comment=Triggered when a wireless microphone transmitter links with the receiver
Action=Popup
Urgency=Normal

[Event/lowBattery]
Name=Transmitter Low Battery Alert
Comment=Triggered when TX1 or TX2 battery drops at or below the configured threshold
Action=Popup
Urgency=Critical

[Event/micMuteChanged]
Name=Microphone Mute & Hardware Setting Changed
Comment=Triggered when microphone mute or hardware settings are toggled via global shortcut
Action=Popup
Urgency=Low

[Event/noiseProfileLearned]
Name=AI Room Noise Profile Learned
Comment=Triggered when 3-second RNNoise + 4-band spectral room noise profiling finishes
Action=Popup
Urgency=Low

[Event/telemetrySummary]
Name=On-Demand Telemetry & Battery Summary
Comment=Triggered by the Show Receiver & Transmitter Battery Notification global shortcut
Action=Popup
Urgency=Normal

[Event/alertTest]
Name=Manual Alert Test
Comment=Triggered when testing desktop notifications from Settings or System Tray
Action=Popup
Urgency=Normal
";

pub fn ensure_kde_notifyrc_installed() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    let dir = PathBuf::from(&home).join(".local/share/knotifications6");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(format!("{KDE_NOTIFYRC_COMPONENT}.notifyrc"));
    let needs_write = match fs::read_to_string(&path) {
        Ok(existing) => existing != KDE_NOTIFYRC_CONTENT,
        Err(_) => true,
    };
    if needs_write {
        let _ = fs::write(&path, KDE_NOTIFYRC_CONTENT);
    }
}

pub fn is_kde_notification_event_enabled(event_id: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    let cfg_path = PathBuf::from(home)
        .join(".config")
        .join(format!("{KDE_NOTIFYRC_COMPONENT}.notifyrc"));
    let Ok(content) = fs::read_to_string(&cfg_path) else {
        return true;
    };
    let target_header = format!("[Event/{event_id}]");
    let mut in_section = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line.eq_ignore_ascii_case(&target_header);
            continue;
        }
        if in_section {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("Action") {
                    let val = v.trim();
                    if val.is_empty() || val.eq_ignore_ascii_case("None") {
                        return false;
                    }
                    return val
                        .split('|')
                        .chain(val.split(','))
                        .any(|part| part.trim().eq_ignore_ascii_case("Popup"));
                }
            }
        }
    }
    true
}

fn send_desktop_popup_event(
    st: &mut BackendState,
    event_id: &str,
    title: &str,
    body: &str,
    urgency: &str,
) {
    log_event(st, &format!("{}: {}", title, body));
    if st.ui_test_mode || !is_kde_notification_event_enabled(event_id) {
        return;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/millinr".to_string());
    let icon_candidate =
        PathBuf::from(&home).join(".local/share/icons/hicolor/256x256/apps/dji-mic-tray.png");
    let icon_arg = if icon_candidate.exists() {
        icon_candidate.to_string_lossy().to_string()
    } else {
        "audio-input-microphone".to_string()
    };
    let _ = Command::new("notify-send")
        .args([
            "-u",
            urgency,
            "-a",
            "KDE DJI Mic Mini Control",
            "-i",
            &icon_arg,
            "-h",
            &format!("string:desktop-entry:{KDE_NOTIFYRC_COMPONENT}"),
            "-h",
            &format!("string:x-kde-appname:{KDE_NOTIFYRC_COMPONENT}"),
            "-h",
            "string:x-kde-display-appname:KDE DJI Mic Mini Control",
            "-h",
            &format!("string:x-kde-eventId:{event_id}"),
            title,
            body,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn send_desktop_popup(st: &mut BackendState, title: &str, body: &str, urgency: &str) {
    let event_id = if title.contains("Test") || title.contains("Status Popup") {
        "alertTest"
    } else if title.contains("Low Battery") {
        "lowBattery"
    } else if title.contains("Transmitter Disconnected") || title.contains("No Transmitter Linked") {
        "txDisconnected"
    } else if title.contains("Transmitter Connected") {
        "txConnected"
    } else if title.contains("Receiver Connected") {
        "rxConnected"
    } else if title.contains("Disconnected") {
        "rxDisconnected"
    } else {
        "telemetrySummary"
    };
    send_desktop_popup_event(st, event_id, title, body, urgency);
}

pub fn build_full_snapshot_from(st: &BackendState) -> serde_json::Value {
    let nl = get_noise_learn_live_state();
    json!({
        "usb": st.usb_info,
        "udev": st.udev_status,
        "duml": st.status,
        "audio": {
            "alsa_card": st.alsa_card,
            "volume": st.alsa_vol,
            "pipewire": st.pw_info,
            "ai_nc_mode": st.config.ai_nc_mode,
            "ai_nc_strength_pct": st.config.ai_nc_strength_pct,
            "ai_nc_virtual_mic": st.config.ai_nc_virtual_mic,
            "ai_nc_virtual_mic_active": nl.virtual_mic_active,
            "noise_learning": {
                "active": nl.learning_active,
                "progress_pct": nl.progress_pct,
                "status": nl.status_text,
                "profile": nl.profile,
            }
        }
    })
}

fn write_runtime_status(st: &BackendState) {
    if st.ui_test_mode {
        return;
    }
    let sp = status_json_path();
    if let Ok(js) = serde_json::to_string_pretty(&build_full_snapshot_from(st)) {
        if fs::write(&sp, js).is_ok() {
            let _ = fs::set_permissions(&sp, fs::Permissions::from_mode(0o600));
        }
    }
}

fn evaluate_alerts(st: &mut BackendState) {
    let rx_ok = st.usb_info.present && st.status.connected;
    let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };

    if st.usb_info.present && !st.status.connected {
        return;
    }

    if !st.initial_check_done {
        st.initial_check_done = true;
        st.prev_rx_connected = Some(rx_ok);
        st.prev_tx_count = Some(tx_cnt);
        if !rx_ok && st.config.alert_on_rx_disconnect {
            send_desktop_popup(
                st,
                "DJI Mic Mini Disconnected",
                "The DJI Mic Mini USB-C Receiver (2ca3:4011) is not connected to this PC.",
                "critical",
            );
        } else if rx_ok && tx_cnt == 0 && st.config.alert_on_tx_disconnect {
            send_desktop_popup(
                st,
                "DJI Mic Mini Transmitter Disconnected",
                "The USB-C Receiver is plugged in, but no wireless microphone transmitter (TX1/TX2) is powered on or linked.",
                "critical",
            );
        } else {
            log_event(
                st,
                &format!(
                    "Monitoring started — Receiver connected, {} transmitter(s) active.",
                    tx_cnt
                ),
            );
        }
        return;
    }

    if st.prev_rx_connected == Some(true) && !rx_ok {
        if st.config.alert_on_rx_disconnect {
            send_desktop_popup(
                st,
                "DJI Mic Mini USB Receiver Disconnected",
                "The DJI Mic Mini USB-C Receiver (2ca3:4011) was unplugged.",
                "critical",
            );
        }
    } else if st.prev_rx_connected == Some(false) && rx_ok {
        let rx_sn = st
            .status
            .rx
            .as_ref()
            .and_then(|r| r.serial.as_deref())
            .unwrap_or("—")
            .to_string();
        send_desktop_popup(
            st,
            "DJI Mic Mini Receiver Connected",
            &format!(
                "USB-C Receiver (S/N {}) connected ({} transmitter(s) linked).",
                rx_sn, tx_cnt
            ),
            "normal",
        );
    }

    if rx_ok && st.prev_rx_connected == Some(true) {
        let prev_tx = st.prev_tx_count.unwrap_or(0);
        if prev_tx > 0 && tx_cnt == 0 {
            if st.config.alert_on_tx_disconnect {
                send_desktop_popup(
                    st,
                    "DJI Mic Mini Transmitter Disconnected",
                    "All DJI Mic Mini wireless transmitters (TX1/TX2) powered off or lost link.",
                    "critical",
                );
            }
        } else if prev_tx == 0 && tx_cnt > 0 {
            let mut summary = Vec::new();
            for t in st.status.tx.iter().flatten() {
                if t.connected {
                    summary.push(format!("TX{}: {}%", t.unit, t.battery_percent.unwrap_or(100)));
                }
            }
            send_desktop_popup(
                st,
                "DJI Mic Mini Transmitter Connected",
                &format!("Wireless link active ({}).", summary.join(", ")),
                "normal",
            );
        }
    }

    if rx_ok && st.config.alert_on_low_battery {
        let thresh = st.config.low_battery_threshold_pct;
        let tx_clone = st.status.tx.clone();
        for t in tx_clone.into_iter().flatten() {
            let unit_idx = t.unit.clamp(1, 2) as usize;
            if t.connected {
                if let Some(pct) = t.battery_percent {
                    if pct <= thresh && !st.low_bat_alerted[unit_idx] {
                        st.low_bat_alerted[unit_idx] = true;
                        send_desktop_popup(
                            st,
                            &format!("DJI Mic Mini TX{} Low Battery ({}%)", t.unit, pct),
                            &format!(
                                "Transmitter TX{} (S/N {}) battery is at {}% (gauge {}/7).",
                                t.unit,
                                t.serial.as_deref().unwrap_or("—"),
                                pct,
                                t.battery.unwrap_or(1)
                            ),
                            "critical",
                        );
                    } else if pct > thresh {
                        st.low_bat_alerted[unit_idx] = false;
                    }
                }
            }
        }
    }

    st.prev_rx_connected = Some(rx_ok);
    st.prev_tx_count = Some(tx_cnt);
}

pub fn poll_tick_internal(force_slow: bool) {
    let now = Instant::now();
    let slow_due = {
        let st = get_state().lock().unwrap();
        if st.ui_test_mode || st.quitting {
            return;
        }
        force_slow || now.duration_since(st.last_slow_poll) >= Duration::from_millis(2500)
    };

    if slow_due {
        let usb_info = get_usb_sysfs_info();
        let udev_status = check_udev_status(Some(&usb_info));
        let alsa_card = get_alsa_card_info();
        let alsa_vol = get_alsa_volume();
        let pw_info = get_pipewire_stream_details();

        let mut st = get_state().lock().unwrap();
        st.last_slow_poll = now;
        st.usb_info = usb_info;
        st.udev_status = udev_status;
        st.alsa_card = alsa_card;
        if let Some(v) = alsa_vol {
            st.alsa_vol = v;
        }
        st.pw_info = pw_info;
        if st.config.ai_nc_mode == "adaptive" {
            maybe_run_adaptive_micro_learn(
                st.config.ai_nc_strength_pct,
                st.config.ai_nc_virtual_mic,
            );
        }
    }

    let (present, hidraw_node) = {
        let st = get_state().lock().unwrap();
        (st.usb_info.present, st.usb_info.hidraw_node.clone())
    };

    if !present {
        let mut dev_guard = get_usb_dev().lock().unwrap();
        if let Some(mut dev) = dev_guard.take() {
            dev.close();
        }
        let mut st = get_state().lock().unwrap();
        st.status = DeviceStatus::default();
    } else {
        let mut dev_guard = get_usb_dev().lock().unwrap();
        if dev_guard.as_ref().map(|d| !d.is_open()).unwrap_or(true) {
            let mut dev = DjiMicDevice::new();
            if dev.open().is_ok() {
                if !hidraw_node.is_empty() {
                    let hid_info = query_hid_service_info(&hidraw_node);
                    dev.status.paired_tx_count_hid = hid_info.get("paired_tx_count").copied();
                    dev.status.active_tx_index_hid = hid_info.get("active_tx_index").copied();
                }
                *dev_guard = Some(dev);
            } else {
                dev.close();
                *dev_guard = None;
                let mut st = get_state().lock().unwrap();
                st.status = DeviceStatus::default();
            }
        }

        if let Some(ref mut dev) = *dev_guard {
            let mut disconnected = false;
            for _ in 0..3 {
                match dev.poll_frames(15) {
                    Ok(frames) => {
                        if frames.is_empty() {
                            break;
                        }
                    }
                    Err(_) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            if disconnected {
                dev.close();
                *dev_guard = None;
                let usb_info = get_usb_sysfs_info();
                let udev_status = check_udev_status(Some(&usb_info));
                let mut st = get_state().lock().unwrap();
                st.usb_info = usb_info;
                st.udev_status = udev_status;
                st.status = DeviceStatus::default();
            } else {
                let mut st = get_state().lock().unwrap();
                st.status = dev.status.clone();
            }
        }
    }

    {
        let mut st = get_state().lock().unwrap();
        evaluate_alerts(&mut st);
        write_runtime_status(&st);
    }
    emit_state_changed();
}

pub fn apply_setting_sync(
    setting: &str,
    value: &str,
    tx_unit: Option<i32>,
) -> Result<bool, String> {
    if find_setting_spec(setting).is_none() {
        return Err(format!(
            "Setting '{}' is not in the verified allowlist.",
            setting
        ));
    }

    let (ok, updated_status) = {
        let mut dev_guard = get_usb_dev().lock().unwrap();
        let Some(ref mut dev) = *dev_guard else {
            return Err("DJI Mic Mini Receiver is not connected.".to_string());
        };
        if !dev.is_open() {
            return Err("DJI Mic Mini Receiver is not connected.".to_string());
        }
        let ack = dev.set_setting(setting, value, tx_unit)?;
        (ack, dev.status.clone())
    };

    let matched = updated_status
        .settings
        .get(setting)
        .map(|v| v == value)
        .unwrap_or(false);
    {
        let mut st = get_state().lock().unwrap();
        st.status = updated_status;
        st.last_action_message = format!("Applied {} = {}", setting, value);
        log_event(&mut st, &format!("Hardware setting updated: {} -> {}", setting, value));
    }
    emit_state_changed();
    Ok(ok || matched)
}

fn start_socket_server() {
    let sp = socket_path();
    let _ = fs::remove_file(&sp);

    thread::spawn(move || {
        let Ok(listener) = UnixListener::bind(&sp) else {
            return;
        };
        let _ = fs::set_permissions(&sp, fs::Permissions::from_mode(0o600));
        let my_uid = unsafe { libc::getuid() };

        for stream_res in listener.incoming() {
            let Ok(mut stream) = stream_res else {
                break;
            };
            if get_state().lock().unwrap().quitting {
                break;
            }
            let fd = stream.as_raw_fd();
            let mut cred = libc::ucred {
                pid: 0,
                uid: u32::MAX,
                gid: u32::MAX,
            };
            let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
            let rc = unsafe {
                libc::getsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_PEERCRED,
                    &mut cred as *mut _ as *mut libc::c_void,
                    &mut len,
                )
            };
            if rc != 0 || cred.uid != my_uid {
                let _ = stream.write_all(b"{\"ok\":false,\"error\":\"Unauthorized peer UID\"}");
                continue;
            }

            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]).trim().to_string();
            let req: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
            let act = req.get("action").and_then(|v| v.as_str()).unwrap_or("");

            let resp = match act {
                "status" => {
                    let st = get_state().lock().unwrap();
                    build_full_snapshot_from(&st)
                }
                "show" => {
                    {
                        let mut st = get_state().lock().unwrap();
                        st.window_visible = true;
                    }
                    emit_window_changed();
                    json!({"ok": true})
                }
                "toggle" => {
                    {
                        let mut st = get_state().lock().unwrap();
                        st.window_visible = !st.window_visible;
                    }
                    emit_window_changed();
                    json!({"ok": true})
                }
                "set" => {
                    let setting = req.get("setting").and_then(|v| v.as_str()).unwrap_or("");
                    let value = req.get("value").and_then(|v| v.as_str()).unwrap_or("on");
                    let tx_u = req.get("tx").and_then(|v| v.as_i64()).map(|x| x as i32);
                    match apply_setting_sync(setting, value, tx_u) {
                        Ok(ok) => json!({"ok": ok}),
                        Err(e) => json!({"ok": false, "error": e}),
                    }
                }
                other => execute_shortcut_action(other),
            };

            if let Ok(out) = serde_json::to_vec(&resp) {
                let _ = stream.write_all(&out);
            }
        }
    });
}

fn c_str_to_rust(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().to_string() }
}

fn rust_to_c_str(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

#[no_mangle]
pub extern "C" fn rust_free_str(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_register_callbacks(
    on_state: VoidCallback,
    on_window: VoidCallback,
    on_config: VoidCallback,
) {
    let _ = CB_STATE_CHANGED.set(on_state);
    let _ = CB_WINDOW_CHANGED.set(on_window);
    let _ = CB_CONFIG_CHANGED.set(on_config);
}

#[no_mangle]
pub extern "C" fn rust_init_backend(start_minimized: bool, ui_test_mode: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.ui_test_mode = ui_test_mode;
        st.window_visible = !start_minimized;
    }

    if ui_test_mode {
        let mut st = get_state().lock().unwrap();
        st.usb_info = UsbSysfsInfo {
            present: true,
            vid_pid: "2ca3:4011".to_string(),
            sysfs_path: "/sys/devices/pci0000:00/0000:00:08.1/0000:11:00.4/usb3/3-1/3-1.2"
                .to_string(),
            bus_num: 3,
            dev_num: 23,
            port_path: "3-1.2".to_string(),
            usb_version: "2.00".to_string(),
            speed_mbps: "12 Mbps".to_string(),
            max_power: "100mA".to_string(),
            manufacturer: "DJI Technology Co., Ltd.".to_string(),
            product: "DJI MIC MINI".to_string(),
            usb_serial: "XSP12345678B".to_string(),
            hidraw_node: "/dev/hidraw10".to_string(),
            hid_phys: "usb-0000:11:00.4-1.2/input0".to_string(),
            usbfs_node: "/dev/bus/usb/003/023".to_string(),
        };
        st.udev_status = check_udev_status(Some(&st.usb_info));
        let mut settings = BTreeMap::new();
        settings.insert("noise-cancel".to_string(), "basic".to_string());
        settings.insert("noise-cancel-power".to_string(), "on".to_string());
        settings.insert("stereo".to_string(), "mono".to_string());
        settings.insert("low-cut".to_string(), "off".to_string());
        settings.insert("safety-track".to_string(), "off".to_string());
        settings.insert("clip-limiter".to_string(), "on".to_string());
        settings.insert("mic-leds".to_string(), "on".to_string());
        settings.insert("auto-off-15m".to_string(), "on".to_string());
        settings.insert("tx-auto-off-15m".to_string(), "on".to_string());
        settings.insert("camera-power".to_string(), "off".to_string());
        settings.insert("plug-free".to_string(), "on".to_string());

        st.status = DeviceStatus {
            connected: true,
            dialect: "v1".to_string(),
            gain_dial_db: Some(0),
            nc_enabled: Some(true),
            rx: Some(RxInfo {
                serial: Some("8PCXN5H02429TH".to_string()),
                firmware: Some("01.01.00.56".to_string()),
                hardware_model: Some("DJI MIC MINI RX".to_string()),
                hardware_revision: Some("01.00.00.00".to_string()),
                gain_step_code: Some(3),
                raw_flags0: Some(0x32),
                raw_flags1: Some(0x20),
                raw_plug_free_byte: Some(0x01),
                ..Default::default()
            }),
            tx: vec![
                Some(TxInfo {
                    unit: 1,
                    connected: true,
                    level: Some(54),
                    serial: Some("8PDXN5E035JPX5".to_string()),
                    firmware: Some("01.01.00.56".to_string()),
                    product_name: Some("DJI Mic Mini TX".to_string()),
                    voice_tone: Some("standard".to_string()),
                    charging: false,
                    battery: Some(1),
                    battery_percent: Some(100),
                    nc_enabled: Some(true),
                    nc_mode: Some("basic".to_string()),
                    low_cut: Some(false),
                    auto_off_15m: Some(true),
                    nc_button_enabled: Some(true),
                    leds_enabled: Some(true),
                    raw_state_byte: Some(0x90),
                    raw_flags_byte: Some(0x26),
                    raw_led_byte: Some(0x00),
                    ..Default::default()
                }),
                None,
            ],
            settings,
            paired_tx_count_hid: Some(1),
            active_tx_index_hid: Some(0xFF),
            last_sequence: 15751,
            heartbeat_count: 420,
            last_heartbeat_hex: Some(
                "5546048b5a0229ae005b0300390090263400010100380e385044584e35453033354a505835"
                    .to_string(),
            ),
            aux_5b05_hex: Some("0000000004000d000101".to_string()),
            aux_0474_hex: Some("04".to_string()),
        };
    } else {
        ensure_kde_notifyrc_installed();
        start_socket_server();
        poll_tick_internal(true);
    }
}

#[no_mangle]
pub extern "C" fn rust_poll_tick() {
    poll_tick_internal(false);
}

fn is_alsa_card_capturing(card_index: Option<i32>) -> bool {
    let Some(idx) = card_index else {
        return false;
    };
    let status_path = format!("/proc/asound/card{idx}/pcm0c/sub0/status");
    if let Ok(txt) = fs::read_to_string(status_path) {
        return txt.contains("RUNNING");
    }
    false
}

#[no_mangle]
pub extern "C" fn rust_get_bool(key_ptr: *const c_char) -> bool {
    let key = c_str_to_rust(key_ptr);
    let st = get_state().lock().unwrap();
    let rx_ok = st.usb_info.present && st.status.connected;
    match key.as_str() {
        "deviceInUse" => {
            st.window_visible
                || (rx_ok
                    && (is_alsa_card_capturing(st.alsa_card.card_index)
                        || get_noise_learn_live_state().learning_active))
        }
        "windowVisible" => st.window_visible,
        "quitting" => st.quitting,
        "startOnLogin" => st.config.start_on_login,
        "startMinimized" => st.config.start_minimized,
        "alertOnRxDisconnect" => st.config.alert_on_rx_disconnect,
        "alertOnTxDisconnect" => st.config.alert_on_tx_disconnect,
        "alertOnLowBattery" => st.config.alert_on_low_battery,
        "udevReady" => st.udev_status.ready,
        "usbPresent" => st.usb_info.present,
        "rxConnected" => rx_ok,
        "tx1Connected" => {
            rx_ok
                && st
                    .status
                    .tx
                    .first()
                    .and_then(|t| t.as_ref())
                    .map(|t| t.connected)
                    .unwrap_or(false)
        }
        "tx1Charging" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .map(|t| t.charging)
            .unwrap_or(false),
        "tx2Connected" => {
            rx_ok
                && st
                    .status
                    .tx
                    .get(1)
                    .and_then(|t| t.as_ref())
                    .map(|t| t.connected)
                    .unwrap_or(false)
        }
        "tx2Charging" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .map(|t| t.charging)
            .unwrap_or(false),
        "alsaMuted" => st.alsa_vol.muted,
        "aiNcVirtualMic" => st.config.ai_nc_virtual_mic,
        "aiNcVirtualMicActive" => get_noise_learn_live_state().virtual_mic_active,
        "noiseLearnActive" => get_noise_learn_live_state().learning_active,
        "noiseProfileLearned" => get_noise_learn_live_state().profile.learned,
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn rust_get_int(key_ptr: *const c_char) -> i32 {
    let key = c_str_to_rust(key_ptr);
    let st = get_state().lock().unwrap();
    let rx_ok = st.usb_info.present && st.status.connected;
    match key.as_str() {
        "activeTabIndex" => st.active_tab_index,
        "uiZoomPercent" => st.config.ui_zoom_percent,
        "lowBatteryThresholdPct" => st.config.low_battery_threshold_pct,
        "txConnectedCount" => {
            if rx_ok {
                st.status.tx_count()
            } else {
                0
            }
        }
        "lastSequence" => st.status.last_sequence as i32,
        "heartbeatCount" => st.status.heartbeat_count as i32,
        "tx1BatteryPct" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.battery_percent)
            .unwrap_or(0),
        "tx1BatteryCode" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.battery)
            .unwrap_or(0),
        "tx1VuLevel" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.level)
            .unwrap_or(0),
        "tx2BatteryPct" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.battery_percent)
            .unwrap_or(0),
        "tx2BatteryCode" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.battery)
            .unwrap_or(0),
        "tx2VuLevel" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.level)
            .unwrap_or(0),
        "alsaVolumePct" => st.alsa_vol.percent,
        "aiNcStrengthPct" => st.config.ai_nc_strength_pct,
        "noiseLearnProgressPct" => get_noise_learn_live_state().progress_pct,
        "dominantHumHz" => get_noise_learn_live_state().profile.dominant_hum_hz,
        "learnedVadThresholdPct" => get_noise_learn_live_state().profile.learned_vad_threshold_pct,
        "learnedHighpassHz" => get_noise_learn_live_state().profile.learned_highpass_hz,
        "adaptivePasses" => get_noise_learn_live_state().profile.adaptive_passes as i32,
        "minBatteryPct" => {
            let mut min_b: Option<i32> = None;
            for t in st.status.tx.iter().flatten() {
                if t.connected {
                    if let Some(p) = t.battery_percent {
                        min_b = Some(min_b.map(|m| m.min(p)).unwrap_or(p));
                    }
                }
            }
            min_b.unwrap_or(-1)
        }
        "maxVuLevel" => st
            .status
            .tx
            .iter()
            .flatten()
            .filter(|t| t.connected)
            .filter_map(|t| t.level)
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn rust_get_str(key_ptr: *const c_char) -> *mut c_char {
    let key = c_str_to_rust(key_ptr);
    let st = get_state().lock().unwrap();
    let rx_ok = st.usb_info.present && st.status.connected;
    let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };
    let rx_model = st
        .status
        .rx
        .as_ref()
        .and_then(|r| r.hardware_model.as_deref())
        .unwrap_or("DJI MIC MINI RX");
    let rx_sn = st
        .status
        .rx
        .as_ref()
        .and_then(|r| r.serial.as_deref())
        .unwrap_or("—");
    let rx_fw = st
        .status
        .rx
        .as_ref()
        .and_then(|r| r.firmware.as_deref())
        .unwrap_or("—");

    let s: String = match key.as_str() {
        "colorScheme" => st.config.color_scheme.clone(),
        "udevSummary" => st.udev_status.summary.clone(),
        "udevRuleFileStatus" => format!(
            "{} • {}",
            st.udev_status.rule_path,
            if st.udev_status.rule_installed && st.udev_status.rule_content_valid {
                "Installed & Valid"
            } else {
                "Missing / Incomplete"
            }
        ),
        "udevDeviceAccessStatus" => {
            if !st.usb_info.present {
                "USB Receiver not currently plugged in".to_string()
            } else {
                format!(
                    "USBFS {}: {} • HID {}: {}",
                    st.udev_status.usbfs_node,
                    if st.udev_status.usbfs_writable {
                        "WRITABLE (OK)"
                    } else {
                        "READ-ONLY (Blocked)"
                    },
                    st.udev_status.hidraw_node,
                    if st.udev_status.hidraw_writable {
                        "WRITABLE (OK)"
                    } else {
                        "READ-ONLY"
                    }
                )
            }
        }
        "overallStatusState" => {
            if !rx_ok {
                "error".to_string()
            } else if tx_cnt == 0 {
                "warn".to_string()
            } else {
                "ok".to_string()
            }
        }
        "overallStatusBadge" => {
            if !rx_ok {
                "USB RECEIVER DISCONNECTED".to_string()
            } else if tx_cnt == 0 {
                "RECEIVER ONLY • NO TX LINKED".to_string()
            } else {
                let mut parts = Vec::new();
                for t in st.status.tx.iter().flatten() {
                    if t.connected {
                        parts.push(format!("TX{}: {}%", t.unit, t.battery_percent.unwrap_or(100)));
                    }
                }
                format!("CONNECTED • {}", parts.join(" & "))
            }
        }
        "headerSubtitle" => {
            if !rx_ok {
                "DJI Mic Mini USB-C Receiver (2ca3:4011) is not plugged in.".to_string()
            } else {
                let pw_id = st
                    .pw_info
                    .wpctl_id
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let stereo = st
                    .status
                    .settings
                    .get("stereo")
                    .map(|s| s.as_str())
                    .unwrap_or("mono")
                    .to_uppercase();
                format!(
                    "{} (S/N {}, FW {}) • {} Wireless Transmitter(s) Linked • PipeWire #{} ({})",
                    rx_model, rx_sn, rx_fw, tx_cnt, pw_id, stereo
                )
            }
        }
        "protocolDialect" => st.status.dialect.clone(),
        "rxHardwareModel" => rx_model.to_string(),
        "rxSerial" => rx_sn.to_string(),
        "rxFirmware" => rx_fw.to_string(),
        "rxFirmwareAndRev" => {
            let Some(ref rx) = st.status.rx else {
                return rust_to_c_str("—");
            };
            let fw = rx.firmware.as_deref().unwrap_or("—");
            let rev_part = rx
                .hardware_revision
                .as_ref()
                .map(|r| format!(" • HW Rev {}", r))
                .unwrap_or_default();
            let flags_part = match (rx.raw_flags0, rx.raw_flags1) {
                (Some(f0), Some(f1)) => format!(" (flags0=0x{:02x}, flags1=0x{:02x})", f0, f1),
                _ => String::new(),
            };
            format!("FW {}{}{}", fw, rev_part, flags_part)
        }
        "rxGainSummary" => match st.status.gain_dial_db {
            None => "0 dB".to_string(),
            Some(db) => {
                let step = st.status.rx.as_ref().and_then(|r| r.gain_step_code);
                let step_part = step
                    .map(|s| format!(" (Dial Step {}/5)", s))
                    .unwrap_or_default();
                format!("{:+} dB{}", db, step_part)
            }
        },
        "rxSubtitle" => {
            if !rx_ok {
                "Receiver offline".to_string()
            } else {
                format!("Model {} • S/N {} • FW {}", rx_model, rx_sn, rx_fw)
            }
        }
        "tx1Serial" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.serial.clone())
            .unwrap_or_else(|| "—".to_string()),
        "tx1Firmware" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.firmware.clone())
            .unwrap_or_else(|| "—".to_string()),
        "tx1Product" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.product_name.clone())
            .unwrap_or_default(),
        "tx1Tone" => st
            .status
            .tx
            .first()
            .and_then(|t| t.as_ref())
            .and_then(|t| t.voice_tone.clone())
            .unwrap_or_else(|| "standard".to_string()),
        "tx1NcSummary" => match st.status.tx.first().and_then(|t| t.as_ref()) {
            None => "—".to_string(),
            Some(t) => format!(
                "{} (Mode: {} • TX Button Toggle: {})",
                if t.nc_enabled.unwrap_or(false) {
                    "ACTIVE"
                } else {
                    "OFF"
                },
                t.nc_mode.as_deref().unwrap_or("basic").to_uppercase(),
                if t.nc_button_enabled.unwrap_or(true) {
                    "ON"
                } else {
                    "OFF"
                }
            ),
        },
        "tx1RawFlags" => match st.status.tx.first().and_then(|t| t.as_ref()) {
            Some(t) if t.raw_state_byte.is_some() && t.raw_flags_byte.is_some() => format!(
                "state=0x{:02x} flags=0x{:02x} led=0x{:02x}",
                t.raw_state_byte.unwrap(),
                t.raw_flags_byte.unwrap(),
                t.raw_led_byte.unwrap_or(0)
            ),
            _ => "—".to_string(),
        },
        "tx2Serial" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.serial.clone())
            .unwrap_or_else(|| "—".to_string()),
        "tx2Firmware" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.firmware.clone())
            .unwrap_or_else(|| "—".to_string()),
        "tx2Product" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.product_name.clone())
            .unwrap_or_default(),
        "tx2Tone" => st
            .status
            .tx
            .get(1)
            .and_then(|t| t.as_ref())
            .and_then(|t| t.voice_tone.clone())
            .unwrap_or_else(|| "standard".to_string()),
        "tx2NcSummary" => match st.status.tx.get(1).and_then(|t| t.as_ref()) {
            None => "—".to_string(),
            Some(t) => format!(
                "{} (Mode: {} • TX Button Toggle: {})",
                if t.nc_enabled.unwrap_or(false) {
                    "ACTIVE"
                } else {
                    "OFF"
                },
                t.nc_mode.as_deref().unwrap_or("basic").to_uppercase(),
                if t.nc_button_enabled.unwrap_or(true) {
                    "ON"
                } else {
                    "OFF"
                }
            ),
        },
        "tx2RawFlags" => match st.status.tx.get(1).and_then(|t| t.as_ref()) {
            Some(t) if t.raw_state_byte.is_some() && t.raw_flags_byte.is_some() => format!(
                "state=0x{:02x} flags=0x{:02x} led=0x{:02x}",
                t.raw_state_byte.unwrap(),
                t.raw_flags_byte.unwrap(),
                t.raw_led_byte.unwrap_or(0)
            ),
            _ => "—".to_string(),
        },
        "settingNcMode" => st
            .status
            .settings
            .get("noise-cancel")
            .cloned()
            .unwrap_or_else(|| "basic".to_string()),
        "settingNcPower" => st
            .status
            .settings
            .get("noise-cancel-power")
            .cloned()
            .unwrap_or_else(|| {
                if st.status.nc_enabled.unwrap_or(false) {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }),
        "ncOverallSummary" => {
            let pwr = st
                .status
                .settings
                .get("noise-cancel-power")
                .map(|s| s.as_str())
                .unwrap_or(if st.status.nc_enabled.unwrap_or(false) {
                    "on"
                } else {
                    "off"
                })
                .to_uppercase();
            let mode = st
                .status
                .settings
                .get("noise-cancel")
                .map(|s| s.as_str())
                .unwrap_or("basic")
                .to_uppercase();
            format!("{} (Strength: {})", pwr, mode)
        }
        "settingStereo" => st
            .status
            .settings
            .get("stereo")
            .cloned()
            .unwrap_or_else(|| "mono".to_string()),
        "settingLowCut" => st
            .status
            .settings
            .get("low-cut")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        "settingSafetyTrack" => st
            .status
            .settings
            .get("safety-track")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        "settingClipLimiter" => st
            .status
            .settings
            .get("clip-limiter")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        "settingLeds" => st
            .status
            .settings
            .get("mic-leds")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        "settingRxAutoOff" => st
            .status
            .settings
            .get("auto-off-15m")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        "settingTxAutoOff" => st
            .status
            .settings
            .get("tx-auto-off-15m")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        "settingCameraPower" => st
            .status
            .settings
            .get("camera-power")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        "settingPlugFree" => st
            .status
            .settings
            .get("plug-free")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        "linuxAudioSummary" => {
            let card_idx = st
                .alsa_card
                .card_index
                .map(|i| i.to_string())
                .unwrap_or_else(|| "?".to_string());
            let card_id = st.alsa_card.card_id.as_deref().unwrap_or("MINI");
            let pw_id = st
                .pw_info
                .wpctl_id
                .or(st.pw_info.pactl_id)
                .map(|i| i.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let vol = st.alsa_vol.percent;
            let db = st.alsa_vol.db.as_deref().unwrap_or("0dB");
            let m_str = if st.alsa_vol.muted { "MUTED" } else { "UNMUTED" };
            let st_str = st.pw_info.stream_state.as_deref().unwrap_or("IDLE");
            format!(
                "ALSA Card #{} [{}] • PipeWire #{} ({}) • Vol {}% ({}) [{}]",
                card_idx, card_id, pw_id, st_str, vol, db, m_str
            )
        }
        "usbVidPid" => st.usb_info.vid_pid.clone(),
        "usbManufacturerProduct" => {
            if !st.usb_info.present {
                "Disconnected".to_string()
            } else {
                format!("{} — {}", st.usb_info.manufacturer, st.usb_info.product)
            }
        }
        "usbDescriptorSerial" => {
            if st.usb_info.usb_serial.is_empty() {
                "—".to_string()
            } else {
                st.usb_info.usb_serial.clone()
            }
        }
        "usbTopology" => {
            if !st.usb_info.present {
                "—".to_string()
            } else {
                format!(
                    "Bus {:03} Device {:03} (Port {})",
                    st.usb_info.bus_num, st.usb_info.dev_num, st.usb_info.port_path
                )
            }
        }
        "usbSpeedAndPower" => {
            if !st.usb_info.present {
                "—".to_string()
            } else {
                format!(
                    "USB {} • {} • MaxPower {}",
                    st.usb_info.usb_version, st.usb_info.speed_mbps, st.usb_info.max_power
                )
            }
        }
        "usbFsNode" => {
            if st.usb_info.usbfs_node.is_empty() {
                "—".to_string()
            } else {
                st.usb_info.usbfs_node.clone()
            }
        }
        "usbSysfsPath" => {
            if st.usb_info.sysfs_path.is_empty() {
                "—".to_string()
            } else {
                st.usb_info.sysfs_path.clone()
            }
        }
        "hidNodeAndPhys" => {
            if st.usb_info.hidraw_node.is_empty() {
                "—".to_string()
            } else {
                format!("{} ({})", st.usb_info.hidraw_node, st.usb_info.hid_phys)
            }
        }
        "hidServiceSummary" => {
            let cnt = st.status.paired_tx_count_hid.unwrap_or(1);
            let idx = st.status.active_tx_index_hid.unwrap_or(0xFF);
            format!(
                "Report 0x55/0x59 Paired TX Count: {} • Report 0x55/0x6a Index: 0x{:02x}",
                cnt, idx
            )
        }
        "alsaCardDetails" => match st.alsa_card.card_index {
            None => "—".to_string(),
            Some(idx) => format!(
                "Card #{} [{}] — {} ({})",
                idx,
                st.alsa_card.card_id.as_deref().unwrap_or(""),
                st.alsa_card.card_name.as_deref().unwrap_or(""),
                st.alsa_card.usb_bus_path.as_deref().unwrap_or("")
            ),
        },
        "pipewireNodeDetails" => {
            let nid = st
                .pw_info
                .wpctl_id
                .or(st.pw_info.pactl_id)
                .map(|i| i.to_string())
                .unwrap_or_else(|| "—".to_string());
            let nname = st.pw_info.node_name.as_deref().unwrap_or("—");
            format!("Node #{} • {}", nid, nname)
        }
        "pipewireFormatState" => {
            let spec = st
                .pw_info
                .sample_spec
                .as_deref()
                .unwrap_or("s24le 2ch 48000Hz");
            let state = st.pw_info.stream_state.as_deref().unwrap_or("SUSPENDED");
            format!("{} • Stream State: {}", spec, state)
        }
        "lastHeartbeatHex" => st
            .status
            .last_heartbeat_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        "aux5b05Hex" => st
            .status
            .aux_5b05_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        "aux0474Hex" => st
            .status
            .aux_0474_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        "lastActionMessage" => st.last_action_message.clone(),
        "eventLogText" => st.event_log.join("\n"),
        "statusBarSummary" => {
            if !rx_ok {
                "USB Receiver (2ca3:4011): Disconnected • Desktop Alerts Active".to_string()
            } else {
                let pw_id = st
                    .pw_info
                    .wpctl_id
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                format!(
                    "RX S/N {} (FW {}) • Active TX: {}/2 • DUML Seq #{} ({} frames) • PipeWire #{}",
                    rx_sn, rx_fw, tx_cnt, st.status.last_sequence, st.status.heartbeat_count, pw_id
                )
            }
        }
        "udevInstallOneliner" => UDEV_INSTALL_ONELINER.to_string(),
        "aiNcMode" => st.config.ai_nc_mode.clone(),
        "noiseLearnStatus" => get_noise_learn_live_state().status_text,
        "noiseProfileLearnedAt" => get_noise_learn_live_state().profile.learned_at,
        "noiseProfileSource" => get_noise_learn_live_state().profile.capture_source,
        "noiseSignature" => get_noise_learn_live_state().profile.noise_signature,
        "noiseFloorSummary" => {
            let p = get_noise_learn_live_state().profile;
            format!(
                "Raw {:+.1} dBFS  →  RNNoise {:+.1} dBFS  (-{:.1} dB Neural Suppression)",
                p.noise_floor_dbfs, p.denoised_floor_dbfs, p.rnnoise_attenuation_db
            )
        }
        "learnedFilterSummary" => {
            let p = get_noise_learn_live_state().profile;
            format!(
                "HP: {} Hz • Hum Notch: {} Hz ({:+.1} dB) • VAD Gate: {}% • Rec HW: NC={}, Low-Cut={}",
                p.learned_highpass_hz,
                p.learned_notch_hz,
                p.learned_notch_db,
                p.learned_vad_threshold_pct,
                p.recommended_hw_nc_mode.to_uppercase(),
                p.recommended_hw_low_cut.to_uppercase()
            )
        }
        "bandLowRumbleStr" => format!("{:+.1}", get_noise_learn_live_state().profile.band_low_rumble_db),
        "bandFanHumStr" => format!("{:+.1}", get_noise_learn_live_state().profile.band_fan_hum_db),
        "bandMidRoomStr" => format!("{:+.1}", get_noise_learn_live_state().profile.band_mid_room_db),
        "bandHighHissStr" => format!("{:+.1}", get_noise_learn_live_state().profile.band_high_hiss_db),
        "fullSnapshotJson" => {
            serde_json::to_string_pretty(&build_full_snapshot_from(&st)).unwrap_or_default()
        }
        _ => String::new(),
    };
    rust_to_c_str(&s)
}

#[no_mangle]
pub extern "C" fn rust_set_window_visible(vis: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.window_visible = vis;
    }
    emit_window_changed();
}

#[no_mangle]
pub extern "C" fn rust_toggle_window() {
    let next = {
        let mut st = get_state().lock().unwrap();
        st.window_visible = !st.window_visible;
        st.window_visible
    };
    let _ = next;
    emit_window_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_active_tab_index(idx: i32) {
    {
        let mut st = get_state().lock().unwrap();
        st.active_tab_index = idx.clamp(0, 4);
    }
    emit_window_changed();
}

#[no_mangle]
pub extern "C" fn rust_refresh_now() {
    poll_tick_internal(true);
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = "Refreshed live USB, udev & DUML telemetry.".to_string();
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_recheck_udev_status() {
    let usb_info = get_usb_sysfs_info();
    let udev_status = check_udev_status(Some(&usb_info));
    {
        let mut st = get_state().lock().unwrap();
        let sum = udev_status.summary.clone();
        st.usb_info = usb_info;
        st.udev_status = udev_status;
        st.last_action_message = format!("udev check: {}", sum);
        log_event(&mut st, &format!("Checked udev status: {}", sum));
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_notify_udev_copied() {
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = "Copied udev rule install command to clipboard.".to_string();
        log_event(&mut st, "Copied udev setup command to clipboard.");
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_install_udev_rule_interactive() {
    let cli_path = "/home/millinr/.local/bin/dji-mic";
    let launched = Command::new("konsole")
        .args(["--hold", "-e", "sudo", cli_path, "udev", "--install"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok();
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = if launched {
            "Launched Konsole to install /etc/udev/rules.d/99-dji-mic.rules.".to_string()
        } else {
            "Command copied to clipboard: sudo dji-mic udev --install".to_string()
        };
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_apply_hardware_setting(
    setting_ptr: *const c_char,
    value_ptr: *const c_char,
) {
    let setting = c_str_to_rust(setting_ptr);
    let value = c_str_to_rust(value_ptr);
    if let Err(e) = apply_setting_sync(&setting, &value, None) {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = format!("Error: {}", e);
        drop(st);
        emit_state_changed();
    }
}

#[no_mangle]
pub extern "C" fn rust_set_system_mic_mute(muted: bool) {
    set_alsa_mute(muted);
    poll_tick_internal(true);
}

#[no_mangle]
pub extern "C" fn rust_set_system_mic_volume(pct: i32) {
    let clamped = pct.clamp(0, 100);
    set_alsa_volume(&format!("{}%", clamped));
    poll_tick_internal(true);
}

#[no_mangle]
pub extern "C" fn rust_learn_noise_profile() {
    let (ui_test, strength, vmic, ai_mode) = {
        let st = get_state().lock().unwrap();
        (
            st.ui_test_mode,
            st.config.ai_nc_strength_pct,
            st.config.ai_nc_virtual_mic,
            st.config.ai_nc_mode.clone(),
        )
    };
    if ui_test {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = "Simulated AI Room Noise Learning (UI Test)".to_string();
        drop(st);
        emit_state_changed();
        return;
    }
    let started = trigger_noise_learning_async(strength, vmic, move |prof| {
        if ai_mode == "learned_ai" || ai_mode == "adaptive" {
            let _ = apply_setting_sync("noise-cancel", &prof.recommended_hw_nc_mode, None);
            let _ = apply_setting_sync("low-cut", &prof.recommended_hw_low_cut, None);
        }
        {
            let mut st = get_state().lock().unwrap();
            st.last_action_message = format!(
                "Learned Room Profile: {} -> Applied HW NC={} & Low-Cut={}",
                prof.noise_signature,
                prof.recommended_hw_nc_mode.to_uppercase(),
                prof.recommended_hw_low_cut.to_uppercase()
            );
            log_event(
                &mut st,
                &format!(
                    "AI Room Noise Learned: {} (RNNoise -{:.1}dB, HW NC={}, Low-Cut={})",
                    prof.noise_signature,
                    prof.rnnoise_attenuation_db,
                    prof.recommended_hw_nc_mode,
                    prof.recommended_hw_low_cut
                ),
            );
            send_desktop_popup_event(
                &mut st,
                "noiseProfileLearned",
                "DJI Mic Mini — AI Room Noise Learned",
                &format!(
                    "{} (RNNoise -{:.1} dB • HW NC: {}, Low-Cut: {})",
                    prof.noise_signature,
                    prof.rnnoise_attenuation_db,
                    prof.recommended_hw_nc_mode.to_uppercase(),
                    prof.recommended_hw_low_cut.to_uppercase()
                ),
                "low",
            );
        }
        emit_state_changed();
    });
    if started {
        let mut st = get_state().lock().unwrap();
        st.last_action_message =
            "Sampling room acoustics (3s) via RNNoise neural network & spectral analyzer..."
                .to_string();
        drop(st);
        emit_state_changed();
    }
}

#[no_mangle]
pub extern "C" fn rust_reset_noise_profile() {
    let _ = reset_learned_noise_profile();
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message =
            "Reset learned AI room noise profile to baseline defaults.".to_string();
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_ai_nc_mode(mode_ptr: *const c_char) {
    let raw = c_str_to_rust(mode_ptr);
    let valid = match raw.as_str() {
        "off" | "hardware" | "learned_ai" | "adaptive" => raw,
        _ => "learned_ai".to_string(),
    };
    let (ui_test, strength, vmic) = {
        let mut st = get_state().lock().unwrap();
        st.config.ai_nc_mode = valid.clone();
        save_config(&st);
        (
            st.ui_test_mode,
            st.config.ai_nc_strength_pct,
            st.config.ai_nc_virtual_mic,
        )
    };
    if !ui_test {
        if valid == "off" && vmic {
            let _ = sync_pipewire_ai_virtual_mic(false, false);
        } else if (valid == "learned_ai" || valid == "adaptive") && vmic {
            let _ = sync_pipewire_ai_virtual_mic(true, false);
        }
        if valid == "learned_ai" || valid == "adaptive" {
            let prof = get_noise_learn_live_state().profile;
            if prof.learned {
                let _ = apply_setting_sync("noise-cancel", &prof.recommended_hw_nc_mode, None);
                let _ = apply_setting_sync("low-cut", &prof.recommended_hw_low_cut, None);
            }
        }
        if valid == "adaptive" {
            maybe_run_adaptive_micro_learn(strength, vmic);
        }
    }
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = format!("AI Noise Cancellation Mode -> {}", valid.to_uppercase());
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_ai_nc_strength(pct: i32) {
    let clamped = pct.clamp(0, 100);
    let (ui_test, vmic, mode) = {
        let mut st = get_state().lock().unwrap();
        st.config.ai_nc_strength_pct = clamped;
        save_config(&st);
        (
            st.ui_test_mode,
            st.config.ai_nc_virtual_mic,
            st.config.ai_nc_mode.clone(),
        )
    };
    if !ui_test {
        recompute_profile_for_strength(clamped, vmic);
        if mode == "learned_ai" || mode == "adaptive" {
            let prof = get_noise_learn_live_state().profile;
            let _ = apply_setting_sync("noise-cancel", &prof.recommended_hw_nc_mode, None);
        }
    }
    {
        let mut st = get_state().lock().unwrap();
        let prof = get_noise_learn_live_state().profile;
        st.last_action_message = format!(
            "AI Suppression Strength -> {}% (HP {}Hz, VAD {}%, Rec HW NC={})",
            clamped,
            prof.learned_highpass_hz,
            prof.learned_vad_threshold_pct,
            prof.recommended_hw_nc_mode.to_uppercase()
        );
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_ai_nc_virtual_mic(enabled: bool) {
    let ui_test = {
        let mut st = get_state().lock().unwrap();
        st.config.ai_nc_virtual_mic = enabled;
        save_config(&st);
        st.ui_test_mode
    };
    let msg = if !ui_test {
        sync_pipewire_ai_virtual_mic(enabled, false).1
    } else {
        format!(
            "PipeWire AI Virtual Mic -> {}",
            if enabled { "ON" } else { "OFF" }
        )
    };
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = msg;
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_notify_json_copied() {
    {
        let mut st = get_state().lock().unwrap();
        st.last_action_message = "Copied full JSON telemetry snapshot to clipboard!".to_string();
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_start_on_login(enabled: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.start_on_login = enabled;
        if !st.ui_test_mode {
            let (_, msg) = apply_start_on_login(enabled, st.config.start_minimized);
            st.last_action_message = msg;
        }
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_start_minimized(enabled: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.start_minimized = enabled;
        if !st.ui_test_mode {
            let _ = apply_start_on_login(st.config.start_on_login, enabled);
            st.last_action_message = format!(
                "Start Minimized to System Tray -> {}",
                if enabled { "ENABLED" } else { "DISABLED" }
            );
        }
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_rx_disconnect(enabled: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.alert_on_rx_disconnect = enabled;
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_tx_disconnect(enabled: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.alert_on_tx_disconnect = enabled;
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_low_battery(enabled: bool) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.alert_on_low_battery = enabled;
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_low_battery_threshold_pct(val: i32) {
    {
        let mut st = get_state().lock().unwrap();
        st.config.low_battery_threshold_pct = val.clamp(5, 70);
        save_config(&st);
    }
    emit_config_changed();
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_send_test_alert() {
    {
        let mut st = get_state().lock().unwrap();
        let rx_ok = st.usb_info.present && st.status.connected;
        let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };
        if !rx_ok {
            send_desktop_popup(
                &mut st,
                "DJI Mic Mini — Disconnected Alert (Test)",
                "The DJI Mic Mini USB-C Receiver (2ca3:4011) is NOT connected.",
                "critical",
            );
        } else if tx_cnt == 0 {
            send_desktop_popup(
                &mut st,
                "DJI Mic Mini — No Transmitter Linked (Test)",
                "Receiver RX is connected, but no wireless transmitter (TX1/TX2) is turned on.",
                "critical",
            );
        } else {
            let rx_sn = st
                .status
                .rx
                .as_ref()
                .and_then(|r| r.serial.as_deref())
                .unwrap_or("—")
                .to_string();
            let tx1_bat = st
                .status
                .tx
                .first()
                .and_then(|t| t.as_ref())
                .and_then(|t| t.battery_percent)
                .unwrap_or(100);
            let tx1_code = st
                .status
                .tx
                .first()
                .and_then(|t| t.as_ref())
                .and_then(|t| t.battery)
                .unwrap_or(1);
            let stereo = st
                .status
                .settings
                .get("stereo")
                .map(|s| s.as_str())
                .unwrap_or("mono")
                .to_uppercase();
            send_desktop_popup(
                &mut st,
                "DJI Mic Mini — Telemetry Status Popup",
                &format!(
                    "Receiver S/N {} • TX1 Battery {}% (Code {}/7) • Mode {}",
                    rx_sn, tx1_bat, tx1_code, stereo
                ),
                "normal",
            );
        }
    }
    emit_state_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_color_scheme(scheme_ptr: *const c_char) {
    let mut s = c_str_to_rust(scheme_ptr);
    if !matches!(s.as_str(), "system" | "dark" | "light") {
        s = "system".to_string();
    }
    {
        let mut st = get_state().lock().unwrap();
        st.config.color_scheme = s;
        save_config(&st);
    }
    emit_config_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_ui_zoom_percent(pct: i32) {
    let rounded = ((pct as f64 / 5.0).round() as i32 * 5).clamp(75, 175);
    {
        let mut st = get_state().lock().unwrap();
        st.config.ui_zoom_percent = rounded;
        save_config(&st);
    }
    emit_config_changed();
}

#[no_mangle]
pub extern "C" fn rust_quit_app() {
    {
        let mut st = get_state().lock().unwrap();
        st.quitting = true;
    }
    emit_window_changed();
    {
        let mut dev_guard = get_usb_dev().lock().unwrap();
        if let Some(mut dev) = dev_guard.take() {
            dev.close();
        }
    }
    let _ = fs::remove_file(socket_path());
}

pub fn execute_shortcut_action(action: &str) -> serde_json::Value {
    match action {
        "_launch" | "show" => {
            {
                let mut st = get_state().lock().unwrap();
                st.window_visible = true;
            }
            emit_window_changed();
            json!({"ok": true})
        }
        "ToggleWindow" | "toggle" => {
            rust_toggle_window();
            json!({"ok": true})
        }
        "ShowBatteryPopup" | "popup-status" => {
            {
                let mut st = get_state().lock().unwrap();
                let rx_ok = st.usb_info.present && st.status.connected;
                let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };
                if !rx_ok {
                    send_desktop_popup_event(
                        &mut st,
                        "telemetrySummary",
                        "DJI Mic Mini — Receiver Disconnected",
                        "The DJI Mic Mini USB-C Receiver (2ca3:4011) is not connected.",
                        "normal",
                    );
                } else {
                    let rx_sn = st
                        .status
                        .rx
                        .as_ref()
                        .and_then(|r| r.serial.as_deref())
                        .unwrap_or("—")
                        .to_string();
                    let mut tx_parts = Vec::new();
                    for t in st.status.tx.iter().flatten() {
                        if t.connected {
                            tx_parts.push(format!(
                                "TX{}: {}% ({}/7)",
                                t.unit,
                                t.battery_percent.unwrap_or(100),
                                t.battery.unwrap_or(1)
                            ));
                        }
                    }
                    let tx_str = if tx_parts.is_empty() {
                        "No TX Linked".to_string()
                    } else {
                        tx_parts.join(" & ")
                    };
                    let nc_mode = st
                        .status
                        .settings
                        .get("noise-cancel")
                        .cloned()
                        .unwrap_or_else(|| "basic".to_string())
                        .to_uppercase();
                    let muted = if st.alsa_vol.muted { "MUTED" } else { "LIVE" };
                    let ai_mode = st.config.ai_nc_mode.to_uppercase();
                    let title = format!("DJI Mic Mini — {} ({})", tx_str, muted);
                    let body = format!(
                        "RX S/N {} • Active TX: {}/2 • HW NC: {} • AI Mode: {}",
                        rx_sn, tx_cnt, nc_mode, ai_mode
                    );
                    send_desktop_popup_event(&mut st, "telemetrySummary", &title, &body, "normal");
                }
            }
            emit_state_changed();
            json!({"ok": true})
        }
        "ToggleMicMute" | "toggle-mute" => {
            let next_mute = {
                let st = get_state().lock().unwrap();
                !st.alsa_vol.muted
            };
            rust_set_system_mic_mute(next_mute);
            {
                let mut st = get_state().lock().unwrap();
                st.last_action_message = format!(
                    "System Microphone -> {}",
                    if next_mute { "MUTED" } else { "UNMUTED" }
                );
                send_desktop_popup_event(
                    &mut st,
                    "micMuteChanged",
                    &format!(
                        "DJI Mic Mini — Microphone {}",
                        if next_mute { "MUTED" } else { "UNMUTED" }
                    ),
                    &format!(
                        "Linux PipeWire/ALSA capture is now {}.",
                        if next_mute { "MUTED" } else { "LIVE (Unmuted)" }
                    ),
                    "low",
                );
            }
            emit_state_changed();
            json!({"ok": true, "muted": next_mute})
        }
        "ToggleNoiseCancel" | "toggle-nc" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("noise-cancel-power")
                    .map(|s| s.as_str())
                    .unwrap_or(if st.status.nc_enabled.unwrap_or(true) {
                        "on"
                    } else {
                        "off"
                    });
                if cur == "on" { "off" } else { "on" }
            };
            let _ = apply_setting_sync("noise-cancel-power", next_val, None);
            json!({"ok": true, "noise_cancel_power": next_val})
        }
        "NoiseCancelBasic" | "nc-basic" => {
            let _ = apply_setting_sync("noise-cancel", "basic", None);
            json!({"ok": true, "noise_cancel": "basic"})
        }
        "NoiseCancelStrong" | "nc-strong" => {
            let _ = apply_setting_sync("noise-cancel", "strong", None);
            json!({"ok": true, "noise_cancel": "strong"})
        }
        "CycleAiNoiseCancel" | "cycle-ai-nc" => {
            let next_mode = {
                let st = get_state().lock().unwrap();
                let order = ["off", "hardware", "learned_ai", "adaptive"];
                let idx = order
                    .iter()
                    .position(|m| *m == st.config.ai_nc_mode)
                    .unwrap_or(2);
                order[(idx + 1) % order.len()].to_string()
            };
            if let Ok(cs) = CString::new(next_mode.clone()) {
                rust_set_ai_nc_mode(cs.as_ptr());
            }
            json!({"ok": true, "ai_nc_mode": next_mode})
        }
        "ToggleAiVirtualMic" | "toggle-virtual-mic" => {
            let next_en = {
                let st = get_state().lock().unwrap();
                !st.config.ai_nc_virtual_mic
            };
            rust_set_ai_nc_virtual_mic(next_en);
            json!({"ok": true, "ai_nc_virtual_mic": next_en})
        }
        "LearnRoomNoise" | "learn-noise" => {
            rust_learn_noise_profile();
            json!({"ok": true})
        }
        "ToggleLowCut" | "toggle-low-cut" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("low-cut")
                    .map(|s| s.as_str())
                    .unwrap_or("off");
                if cur == "on" { "off" } else { "on" }
            };
            let _ = apply_setting_sync("low-cut", next_val, None);
            json!({"ok": true, "low_cut": next_val})
        }
        "ToggleClipLimiter" | "toggle-limiter" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("clip-limiter")
                    .map(|s| s.as_str())
                    .unwrap_or("on");
                if cur == "on" { "off" } else { "on" }
            };
            let _ = apply_setting_sync("clip-limiter", next_val, None);
            json!({"ok": true, "clip_limiter": next_val})
        }
        "ToggleTrackMode" | "toggle-stereo" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("stereo")
                    .map(|s| s.as_str())
                    .unwrap_or("mono");
                if cur == "stereo" { "mono" } else { "stereo" }
            };
            let _ = apply_setting_sync("stereo", next_val, None);
            json!({"ok": true, "stereo": next_val})
        }
        "ToggleSafetyTrack" | "toggle-safety" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("safety-track")
                    .map(|s| s.as_str())
                    .unwrap_or("off");
                if cur == "on" { "off" } else { "on" }
            };
            let _ = apply_setting_sync("safety-track", next_val, None);
            json!({"ok": true, "safety_track": next_val})
        }
        "ToggleTransmitterLeds" | "toggle-leds" => {
            let next_val = {
                let st = get_state().lock().unwrap();
                let cur = st
                    .status
                    .settings
                    .get("mic-leds")
                    .map(|s| s.as_str())
                    .unwrap_or("on");
                if cur == "on" { "off" } else { "on" }
            };
            let _ = apply_setting_sync("mic-leds", next_val, None);
            json!({"ok": true, "mic_leds": next_val})
        }
        _ => json!({"error": format!("Unknown action: {}", action)}),
    }
}

#[no_mangle]
pub extern "C" fn rust_handle_global_shortcut(shortcut_ptr: *const c_char) {
    let shortcut = c_str_to_rust(shortcut_ptr);
    let _ = execute_shortcut_action(&shortcut);
}
