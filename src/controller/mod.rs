pub mod alerts;
pub mod config;
pub mod desktop;
pub mod ffi;
pub mod ipc;
pub mod shortcuts;
pub mod worker;

pub use crate::audio::AiNcMode;
pub use crate::util::socket_path;
pub use config::read_start_minimized_config;
pub use desktop::NotificationEvent;
pub use ffi::*;

use crate::audio::{
    get_noise_learn_live_state, AlsaCardInfo, AlsaVolumeInfo, PipewireStreamInfo,
};
use crate::device::{UdevStatus, UsbSysfsInfo};
use crate::protocol::DeviceStatus;
use crate::util::lock_unpoisoned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::VecDeque;
use std::ffi::{c_char, c_int, CString};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

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
    #[serde(default)]
    pub ai_nc_mode: AiNcMode,
    #[serde(default = "default_ai_nc_strength")]
    pub ai_nc_strength_pct: i32,
    #[serde(default)]
    pub ai_nc_virtual_mic: bool,
}

fn default_true() -> bool {
    true
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
            ai_nc_mode: AiNcMode::LearnedAi,
            ai_nc_strength_pct: 80,
            ai_nc_virtual_mic: false,
        }
    }
}

pub struct BackendState {
    pub ui_test_mode: bool,
    pub quitting: bool,
    pub window_visible: bool,
    pub active_tab_index: i32,
    pub last_action_message: String,
    pub event_log: VecDeque<String>,
    pub config: AppConfig,
    pub usb_info: UsbSysfsInfo,
    pub udev_status: UdevStatus,
    pub status: DeviceStatus,
    pub alsa_card: AlsaCardInfo,
    pub alsa_vol: AlsaVolumeInfo,
    pub pw_info: PipewireStreamInfo,
    pub alsa_capturing: bool,
    pub last_slow_poll: Instant,
    pub last_status_write: Option<Instant>,
    pub last_state_digest: u64,
    pub last_tray_digest: u64,
    pub last_json_digest: u64,
    pub initial_check_done: bool,
    pub prev_rx_connected: Option<bool>,
    pub prev_tx_count: Option<i32>,
    pub low_bat_alerted: [bool; 3],
}

pub static STATE: OnceLock<Arc<Mutex<BackendState>>> = OnceLock::new();

pub fn get_state() -> &'static Arc<Mutex<BackendState>> {
    STATE.get_or_init(|| {
        let cfg = config::load_initial_config();
        Arc::new(Mutex::new(BackendState {
            ui_test_mode: false,
            quitting: false,
            window_visible: true,
            active_tab_index: 0,
            last_action_message: String::new(),
            event_log: VecDeque::with_capacity(40),
            config: cfg,
            usb_info: UsbSysfsInfo::default(),
            udev_status: UdevStatus::default(),
            status: DeviceStatus::default(),
            alsa_card: AlsaCardInfo::default(),
            alsa_vol: AlsaVolumeInfo::default(),
            pw_info: PipewireStreamInfo::default(),
            alsa_capturing: false,
            last_slow_poll: Instant::now() - Duration::from_secs(10),
            last_status_write: None,
            last_state_digest: 0,
            last_tray_digest: 0,
            last_json_digest: 0,
            initial_check_done: false,
            prev_rx_connected: None,
            prev_tx_count: None,
            low_bat_alerted: [false, false, false],
        }))
    })
}

pub type VoidCallback = unsafe extern "C" fn();
pub type NotifyCallback = unsafe extern "C" fn(
    event_id: *const c_char,
    title: *const c_char,
    body: *const c_char,
    icon: *const c_char,
    urgency: c_int,
);
pub type StrCallback = unsafe extern "C" fn(*const c_char);

pub static CB_STATE_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
pub static CB_TRAY_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
pub static CB_WINDOW_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
pub static CB_CONFIG_CHANGED: OnceLock<VoidCallback> = OnceLock::new();
pub static CB_NOTIFY: OnceLock<NotifyCallback> = OnceLock::new();
pub static CB_COLOR_SCHEME: OnceLock<StrCallback> = OnceLock::new();

pub fn emit_state_changed() {
    let should_emit = lock_unpoisoned(get_state()).window_visible;
    if should_emit {
        if let Some(cb) = CB_STATE_CHANGED.get() {
            unsafe { cb() };
        }
    }
}

pub fn emit_tray_changed() {
    if let Some(cb) = CB_TRAY_CHANGED.get() {
        unsafe { cb() };
    }
}

pub fn emit_window_changed() {
    if let Some(cb) = CB_WINDOW_CHANGED.get() {
        unsafe { cb() };
    }
}

pub fn emit_config_changed() {
    if let Some(cb) = CB_CONFIG_CHANGED.get() {
        unsafe { cb() };
    }
}

pub fn emit_color_scheme(scheme: &str) {
    if let Some(cb) = CB_COLOR_SCHEME.get() {
        if let Ok(cs) = CString::new(scheme) {
            unsafe { cb(cs.as_ptr()) };
        }
    }
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
            "ai_nc_mode": st.config.ai_nc_mode.as_str(),
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
