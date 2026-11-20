use super::{AppConfig, BackendState};
use crate::util::{
    config_path, home_dir, run_command_with_timeout, write_file_atomic_mode, xdg_config_home,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

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
    xdg_config_home()
        .join("autostart")
        .join("dji-mic-tray.desktop")
}

pub fn systemd_user_unit_path() -> PathBuf {
    xdg_config_home()
        .join("systemd")
        .join("user")
        .join("dji-mic-tray.service")
}

pub fn check_start_on_login_enabled() -> bool {
    let cfg_dir = xdg_config_home();
    let wants_link = cfg_dir.join("systemd/user/graphical-session.target.wants/dji-mic-tray.service");
    let default_wants_link = cfg_dir.join("systemd/user/default.target.wants/dji-mic-tray.service");
    if wants_link.exists() || default_wants_link.exists() || autostart_desktop_path().exists() {
        return true;
    }
    if let Ok(out) = run_command_with_timeout(
        "systemctl",
        &["--user", "is-enabled", "dji-mic-tray.service"],
        Duration::from_secs(2),
    ) {
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

    let unit_body = systemd_service_unit_content(start_minimized);
    let _ = write_file_atomic_mode(&unit_path, unit_body.as_bytes(), 0o644);
    let timeout = Duration::from_secs(3);
    let _ = run_command_with_timeout("systemctl", &["--user", "daemon-reload"], timeout);

    if enabled {
        if let Ok(out) = run_command_with_timeout(
            "systemctl",
            &["--user", "enable", "dji-mic-tray.service"],
            timeout,
        ) {
            if out.status.success() {
                let _ = fs::remove_file(&auto_path);
                return (
                    true,
                    format!(
                        "Start on Login ENABLED (dji-mic-tray.service • Start Minimized: {})",
                        if start_minimized { "ON" } else { "OFF" }
                    ),
                );
            }
        }

        let exec_bin = home_dir().join(".local/bin/dji-mic-tray");
        let desktop_body = format!(
            "[Desktop Entry]\nType=Application\nName=KDE DJI Mic Mini Control\nComment=Start DJI Mic Mini System Tray Daemon on Login\nExec={}{min_flag}\nIcon=dji-mic-tray\nTerminal=false\nCategories=AudioVideo;Audio;Mixer;Utility;Qt;KDE;\nX-KDE-autostart-after=panel\n",
            exec_bin.to_string_lossy()
        );
        let _ = write_file_atomic_mode(&auto_path, desktop_body.as_bytes(), 0o644);
        (
            true,
            format!("Start on Login ENABLED ({})", auto_path.to_string_lossy()),
        )
    } else {
        let _ = run_command_with_timeout(
            "systemctl",
            &["--user", "disable", "dji-mic-tray.service"],
            timeout,
        );
        let _ = fs::remove_file(&auto_path);
        (
            true,
            "Start on Login DISABLED (dji-mic-tray.service disabled)".to_string(),
        )
    }
}

pub fn load_initial_config() -> AppConfig {
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
    if !matches!(cfg.color_scheme.as_str(), "system" | "dark" | "light") {
        cfg.color_scheme = "system".to_string();
    }
    cfg
}

pub fn save_config(st: &BackendState) {
    if st.ui_test_mode {
        return;
    }
    let cp = config_path();
    if let Ok(js) = serde_json::to_string_pretty(&st.config) {
        let _ = write_file_atomic_mode(&cp, js.as_bytes(), 0o600);
    }
}
