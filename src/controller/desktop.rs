use super::CB_NOTIFY;
use crate::device::{UDEV_INSTALL_ONELINER, UDEV_RULE_CONTENT, UDEV_RULE_PATH};
use crate::util::{
    lock_unpoisoned, spawn_reaped, write_file_atomic_mode, xdg_config_home, xdg_data_home,
};
use std::ffi::CString;
use std::fs;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

pub const KDE_NOTIFYRC_COMPONENT: &str = "dji-mic-tray";
pub const KDE_NOTIFYRC_CONTENT: &str = include_str!("../../packaging/dji-mic-tray.notifyrc");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationEvent {
    RxDisconnected,
    RxConnected,
    TxDisconnected,
    TxConnected,
    LowBattery,
    MicMuteChanged,
    NoiseProfileLearned,
    TelemetrySummary,
    AlertTest,
}

impl NotificationEvent {
    pub fn event_id(self) -> &'static str {
        match self {
            Self::RxDisconnected => "rxDisconnected",
            Self::RxConnected => "rxConnected",
            Self::TxDisconnected => "txDisconnected",
            Self::TxConnected => "txConnected",
            Self::LowBattery => "lowBattery",
            Self::MicMuteChanged => "micMuteChanged",
            Self::NoiseProfileLearned => "noiseProfileLearned",
            Self::TelemetrySummary => "telemetrySummary",
            Self::AlertTest => "alertTest",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingNotification {
    pub event: NotificationEvent,
    pub title: String,
    pub body: String,
    pub urgency: &'static str,
}

pub fn ensure_kde_notifyrc_installed() {
    let dir = xdg_data_home().join("knotifications6");
    let path = dir.join(format!("{KDE_NOTIFYRC_COMPONENT}.notifyrc"));
    let needs_write = match fs::read_to_string(&path) {
        Ok(existing) => existing != KDE_NOTIFYRC_CONTENT,
        Err(_) => true,
    };
    if needs_write {
        let _ = write_file_atomic_mode(&path, KDE_NOTIFYRC_CONTENT.as_bytes(), 0o644);
    }
}

static NOTIFYRC_CACHE: LazyLock<Mutex<(Option<SystemTime>, String)>> =
    LazyLock::new(|| Mutex::new((None, String::new())));

pub fn is_kde_notification_event_enabled(event: NotificationEvent) -> bool {
    let cfg_path = xdg_config_home().join(format!("{KDE_NOTIFYRC_COMPONENT}.notifyrc"));
    let mtime = fs::metadata(&cfg_path).and_then(|m| m.modified()).ok();

    let content = {
        let mut cache = lock_unpoisoned(&NOTIFYRC_CACHE);
        match mtime {
            Some(mt) => {
                if cache.0 != Some(mt) {
                    if let Ok(txt) = fs::read_to_string(&cfg_path) {
                        *cache = (Some(mt), txt);
                    }
                }
                cache.1.clone()
            }
            None => return true,
        }
    };

    let target_header = format!("[Event/{}]", event.event_id());
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

pub fn dispatch_notification(ui_test_mode: bool, notif: &PendingNotification) {
    if ui_test_mode || !is_kde_notification_event_enabled(notif.event) {
        return;
    }

    let icon_candidate = xdg_data_home().join("icons/hicolor/256x256/apps/dji-mic-tray.png");
    let icon_arg = if icon_candidate.exists() {
        icon_candidate.to_string_lossy().to_string()
    } else {
        "audio-input-microphone".to_string()
    };

    if let Some(cb) = CB_NOTIFY.get() {
        let urgency_int = match notif.urgency {
            "low" => 0,
            "critical" => 2,
            _ => 1,
        };
        if let (Ok(eid), Ok(title), Ok(body), Ok(icon)) = (
            CString::new(notif.event.event_id()),
            CString::new(notif.title.as_str()),
            CString::new(notif.body.as_str()),
            CString::new(icon_arg.as_str()),
        ) {
            unsafe { cb(eid.as_ptr(), title.as_ptr(), body.as_ptr(), icon.as_ptr(), urgency_int) };
            return;
        }
    }

    // Headless / non-Qt fallback via `notify-send` with `--` argument terminator and background reaping.
    let desk_hint = format!("string:desktop-entry:{KDE_NOTIFYRC_COMPONENT}");
    let app_hint = format!("string:x-kde-appname:{KDE_NOTIFYRC_COMPONENT}");
    let ev_hint = format!("string:x-kde-eventId:{}", notif.event.event_id());
    let _ = spawn_reaped(
        "notify-send",
        &[
            "-u",
            notif.urgency,
            "-a",
            "KDE DJI Mic Mini Control",
            "-i",
            &icon_arg,
            "-h",
            &desk_hint,
            "-h",
            &app_hint,
            "-h",
            "string:x-kde-display-appname:KDE DJI Mic Mini Control",
            "-h",
            &ev_hint,
            "--",
            &notif.title,
            &notif.body,
        ],
    );
}

pub fn dispatch_notifications(ui_test_mode: bool, pending: &[PendingNotification]) {
    for n in pending {
        dispatch_notification(ui_test_mode, n);
    }
}

/// Install the hardened udev rules via `pkexec` without invoking any user-writable binary as root.
pub fn spawn_pkexec_udev_install<F>(on_finished: F) -> bool
where
    F: FnOnce(bool) + Send + 'static,
{
    let script = format!(
        "cat > {rule_path} <<'EOF'\n{content}EOF\nchmod 0644 {rule_path} && udevadm control --reload-rules && udevadm trigger --subsystem-match=usb --attr-match=idVendor=2ca3 && udevadm trigger --subsystem-match=hidraw",
        rule_path = UDEV_RULE_PATH,
        content = UDEV_RULE_CONTENT,
    );

    let Ok(mut child) = std::process::Command::new("pkexec")
        .args(["sh", "-c", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        let _ = UDEV_INSTALL_ONELINER;
        return false;
    };

    std::thread::spawn(move || {
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        on_finished(ok);
    });
    true
}
