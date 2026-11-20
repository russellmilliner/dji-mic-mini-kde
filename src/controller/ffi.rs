use super::alerts::{log_event, record_notification};
use super::config::save_config;
use super::desktop::{
    dispatch_notifications, ensure_kde_notifyrc_installed, NotificationEvent,
};
use super::ipc::start_socket_server;
use super::shortcuts::execute_shortcut_action;
use super::worker::{send_worker_cmd, start_worker_thread, WorkerCmd};
use super::{
    build_full_snapshot_from, emit_color_scheme, emit_config_changed, emit_state_changed,
    emit_tray_changed, emit_window_changed, get_state, AiNcMode, BackendState, NotifyCallback,
    StrCallback, VoidCallback, CB_COLOR_SCHEME, CB_CONFIG_CHANGED, CB_NOTIFY, CB_STATE_CHANGED,
    CB_TRAY_CHANGED, CB_WINDOW_CHANGED,
};
use crate::audio::{
    get_noise_learn_live_state, reset_learned_noise_profile, trigger_noise_learning_async,
    NoiseLearnLiveState,
};
use crate::device::{check_udev_status, UsbSysfsInfo, UDEV_INSTALL_ONELINER};
use crate::protocol::{find_setting_spec, DeviceStatus, DumlDialect, RxInfo, TxInfo};
use crate::util::{lock_unpoisoned, socket_path};
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TxUiSnapshot {
    pub connected: bool,
    pub battery_pct: i32,
    pub battery_code: i32,
    pub charging: bool,
    pub vu_level: i32,
    pub serial: String,
    pub firmware: String,
    pub product: String,
    pub tone: String,
    pub nc_summary: String,
    pub raw_flags: String,
}

#[derive(Debug, Clone, Serialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiSnapshot {
    pub window_visible: bool,
    pub quitting: bool,
    pub active_tab_index: i32,

    pub ui_zoom_percent: i32,
    pub color_scheme: String,
    pub start_on_login: bool,
    pub start_minimized: bool,
    pub alert_on_rx_disconnect: bool,
    pub alert_on_tx_disconnect: bool,
    pub alert_on_low_battery: bool,
    pub low_battery_threshold_pct: i32,

    pub device_in_use: bool,
    pub min_battery_pct: i32,
    pub max_vu_level: i32,

    pub udev_ready: bool,
    pub udev_summary: String,
    pub udev_rule_file_status: String,
    pub udev_device_access_status: String,

    pub usb_present: bool,
    pub rx_connected: bool,
    pub tx_connected_count: i32,
    pub overall_status_state: String,
    pub overall_status_badge: String,
    pub header_subtitle: String,
    pub protocol_dialect: String,
    pub last_sequence: i32,
    pub heartbeat_count: i32,
    pub rx_hardware_model: String,
    pub rx_serial: String,
    pub rx_firmware: String,
    pub rx_firmware_and_rev: String,
    pub rx_gain_summary: String,
    pub rx_subtitle: String,

    pub tx1: TxUiSnapshot,
    pub tx2: TxUiSnapshot,

    pub setting_nc_mode: String,
    pub setting_nc_power: String,
    pub nc_overall_summary: String,
    pub setting_stereo: String,
    pub setting_low_cut: String,
    pub setting_safety_track: String,
    pub setting_clip_limiter: String,
    pub setting_leds: String,
    pub setting_rx_auto_off: String,
    pub setting_tx_auto_off: String,
    pub setting_camera_power: String,
    pub setting_plug_free: String,

    pub alsa_volume_pct: i32,
    pub alsa_muted: bool,
    pub ai_nc_mode: String,
    pub ai_nc_strength_pct: i32,
    pub ai_nc_virtual_mic: bool,
    pub ai_nc_virtual_mic_active: bool,
    pub noise_learn_active: bool,
    pub noise_learn_progress_pct: i32,
    pub noise_learn_status: String,
    pub noise_profile_learned: bool,
    pub noise_profile_learned_at: String,
    pub noise_profile_source: String,
    pub noise_signature: String,
    pub noise_floor_summary: String,
    pub learned_filter_summary: String,
    pub band_low_rumble_str: String,
    pub band_fan_hum_str: String,
    pub band_mid_room_str: String,
    pub band_high_hiss_str: String,
    pub adaptive_passes: i32,
    pub linux_audio_summary: String,

    pub usb_vid_pid: String,
    pub usb_manufacturer_product: String,
    pub usb_descriptor_serial: String,
    pub usb_topology: String,
    pub usb_speed_and_power: String,
    pub usb_fs_node: String,
    pub usb_sysfs_path: String,
    pub hid_node_and_phys: String,
    pub hid_service_summary: String,
    pub alsa_card_details: String,
    pub pipewire_node_details: String,
    pub pipewire_format_state: String,
    pub last_heartbeat_hex: String,
    pub aux5b05_hex: String,
    pub aux0474_hex: String,
    pub last_action_message: String,
    pub event_log_text: String,
    pub status_bar_summary: String,
}

fn format_tx_slot(tx_opt: Option<&TxInfo>, rx_ok: bool) -> TxUiSnapshot {
    let Some(t) = tx_opt else {
        return TxUiSnapshot {
            connected: false,
            battery_pct: 0,
            battery_code: 0,
            charging: false,
            vu_level: 0,
            serial: "—".to_string(),
            firmware: "—".to_string(),
            product: String::new(),
            tone: "standard".to_string(),
            nc_summary: "—".to_string(),
            raw_flags: "—".to_string(),
        };
    };

    let nc_summary = format!(
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
    );

    let raw_flags = match (t.raw_state_byte, t.raw_flags_byte) {
        (Some(sb), Some(fb)) => format!(
            "state=0x{:02x} flags=0x{:02x} led=0x{:02x}",
            sb,
            fb,
            t.raw_led_byte.unwrap_or(0)
        ),
        _ => "—".to_string(),
    };

    TxUiSnapshot {
        connected: rx_ok && t.connected,
        battery_pct: t.battery_percent.unwrap_or(0),
        battery_code: t.battery.unwrap_or(0),
        charging: t.charging,
        vu_level: t.level.unwrap_or(0),
        serial: t.serial.clone().unwrap_or_else(|| "—".to_string()),
        firmware: t.firmware.clone().unwrap_or_else(|| "—".to_string()),
        product: t.product_name.clone().unwrap_or_default(),
        tone: t.voice_tone.clone().unwrap_or_else(|| "standard".to_string()),
        nc_summary,
        raw_flags,
    }
}

pub fn hash_value<T: Hash>(val: &T) -> u64 {
    let mut h = DefaultHasher::new();
    val.hash(&mut h);
    h.finish()
}

pub fn build_tray_digest(s: &UiSnapshot) -> u64 {
    let mut h = DefaultHasher::new();
    s.device_in_use.hash(&mut h);
    s.rx_connected.hash(&mut h);
    s.tx_connected_count.hash(&mut h);
    s.min_battery_pct.hash(&mut h);
    (s.rx_connected && s.tx_connected_count > 0 && !s.alsa_muted && s.max_vu_level > 40)
        .hash(&mut h);
    s.alsa_muted.hash(&mut h);
    s.overall_status_badge.hash(&mut h);
    s.rx_hardware_model.hash(&mut h);
    s.rx_firmware.hash(&mut h);
    s.rx_serial.hash(&mut h);
    s.rx_gain_summary.hash(&mut h);
    s.tx1.connected.hash(&mut h);
    s.tx1.battery_pct.hash(&mut h);
    s.tx1.battery_code.hash(&mut h);
    s.tx1.serial.hash(&mut h);
    s.tx1.firmware.hash(&mut h);
    s.tx2.connected.hash(&mut h);
    s.tx2.battery_pct.hash(&mut h);
    s.tx2.battery_code.hash(&mut h);
    s.tx2.serial.hash(&mut h);
    s.tx2.firmware.hash(&mut h);
    s.setting_nc_mode.hash(&mut h);
    s.setting_stereo.hash(&mut h);
    s.setting_low_cut.hash(&mut h);
    s.setting_safety_track.hash(&mut h);
    s.setting_clip_limiter.hash(&mut h);
    s.setting_leds.hash(&mut h);
    s.setting_rx_auto_off.hash(&mut h);
    s.setting_camera_power.hash(&mut h);
    s.setting_plug_free.hash(&mut h);
    s.usb_topology.hash(&mut h);
    s.usb_speed_and_power.hash(&mut h);
    s.udev_ready.hash(&mut h);
    s.linux_audio_summary.hash(&mut h);
    s.alsa_volume_pct.hash(&mut h);
    s.alert_on_rx_disconnect.hash(&mut h);
    s.alert_on_tx_disconnect.hash(&mut h);
    s.alert_on_low_battery.hash(&mut h);
    s.low_battery_threshold_pct.hash(&mut h);
    h.finish()
}

pub fn build_ui_snapshot_locked(st: &BackendState, nl: &NoiseLearnLiveState) -> UiSnapshot {
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

    let mut min_b: Option<i32> = None;
    for t in st.status.tx.iter().flatten() {
        if t.connected {
            if let Some(p) = t.battery_percent {
                min_b = Some(min_b.map(|m| m.min(p)).unwrap_or(p));
            }
        }
    }
    let max_vu = st
        .status
        .tx
        .iter()
        .flatten()
        .filter(|t| t.connected)
        .filter_map(|t| t.level)
        .max()
        .unwrap_or(0);

    let overall_status_state = if !rx_ok {
        "error"
    } else if tx_cnt == 0 {
        "warn"
    } else {
        "ok"
    }
    .to_string();

    let overall_status_badge = if !rx_ok {
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
    };

    let pw_id_str = st
        .pw_info
        .wpctl_id
        .map(|i| i.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let setting_stereo = st
        .status
        .settings
        .get("stereo")
        .cloned()
        .unwrap_or_else(|| "mono".to_string());

    let header_subtitle = if !rx_ok {
        "DJI Mic Mini USB-C Receiver (2ca3:4011) is not plugged in.".to_string()
    } else {
        format!(
            "{} (S/N {}, FW {}) • {} Wireless Transmitter(s) Linked • PipeWire #{} ({})",
            rx_model,
            rx_sn,
            rx_fw,
            tx_cnt,
            pw_id_str,
            setting_stereo.to_uppercase()
        )
    };

    let rx_firmware_and_rev = match st.status.rx.as_ref() {
        None => "—".to_string(),
        Some(rx) => {
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
    };

    let rx_gain_summary = match st.status.gain_dial_db {
        None => "0 dB".to_string(),
        Some(db) => {
            let step_part = st
                .status
                .rx
                .as_ref()
                .and_then(|r| r.gain_step_code)
                .map(|s| format!(" (Dial Step {}/5)", s))
                .unwrap_or_default();
            format!("{:+} dB{}", db, step_part)
        }
    };

    let setting_nc_mode = st
        .status
        .settings
        .get("noise-cancel")
        .cloned()
        .unwrap_or_else(|| "basic".to_string());
    let setting_nc_power = st
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
        });
    let nc_overall_summary = format!(
        "{} (Strength: {})",
        setting_nc_power.to_uppercase(),
        setting_nc_mode.to_uppercase()
    );

    let card_idx_str = st
        .alsa_card
        .card_index
        .map(|i| i.to_string())
        .unwrap_or_else(|| "?".to_string());
    let pw_any_id = st
        .pw_info
        .wpctl_id
        .or(st.pw_info.pactl_id)
        .map(|i| i.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let linux_audio_summary = format!(
        "ALSA Card #{} [{}] • PipeWire #{} ({}) • Vol {}% ({}) [{}]",
        card_idx_str,
        st.alsa_card.card_id.as_deref().unwrap_or("MINI"),
        pw_any_id,
        st.pw_info.stream_state.as_deref().unwrap_or("IDLE"),
        st.alsa_vol.percent,
        st.alsa_vol.db.as_deref().unwrap_or("0dB"),
        if st.alsa_vol.muted { "MUTED" } else { "UNMUTED" }
    );

    let status_bar_summary = if !rx_ok {
        "USB Receiver (2ca3:4011): Disconnected • Desktop Alerts Active".to_string()
    } else {
        format!(
            "RX S/N {} (FW {}) • Active TX: {}/2 • DUML Seq #{} ({} frames) • PipeWire #{}",
            rx_sn, rx_fw, tx_cnt, st.status.last_sequence, st.status.heartbeat_count, pw_id_str
        )
    };

    let p = &nl.profile;
    let noise_floor_summary = format!(
        "Raw {:+.1} dBFS  →  RNNoise {:+.1} dBFS  (-{:.1} dB Neural Suppression)",
        p.noise_floor_dbfs, p.denoised_floor_dbfs, p.rnnoise_attenuation_db
    );
    let learned_filter_summary = format!(
        "HP: {} Hz • Hum Notch: {} Hz ({:+.1} dB) • VAD Gate: {}% • Rec HW: NC={}, Low-Cut={}",
        p.learned_highpass_hz,
        p.learned_notch_hz,
        p.learned_notch_db,
        p.learned_vad_threshold_pct,
        p.recommended_hw_nc_mode.to_uppercase(),
        p.recommended_hw_low_cut.to_uppercase()
    );

    let event_log_text = st.event_log.iter().cloned().collect::<Vec<_>>().join("\n");

    UiSnapshot {
        window_visible: st.window_visible,
        quitting: st.quitting,
        active_tab_index: st.active_tab_index,
        ui_zoom_percent: st.config.ui_zoom_percent,
        color_scheme: st.config.color_scheme.clone(),
        start_on_login: st.config.start_on_login,
        start_minimized: st.config.start_minimized,
        alert_on_rx_disconnect: st.config.alert_on_rx_disconnect,
        alert_on_tx_disconnect: st.config.alert_on_tx_disconnect,
        alert_on_low_battery: st.config.alert_on_low_battery,
        low_battery_threshold_pct: st.config.low_battery_threshold_pct,
        device_in_use: st.window_visible || (rx_ok && (st.alsa_capturing || nl.learning_active)),
        min_battery_pct: min_b.unwrap_or(-1),
        max_vu_level: max_vu,
        udev_ready: st.udev_status.ready,
        udev_summary: st.udev_status.summary.clone(),
        udev_rule_file_status: format!(
            "{} • {}",
            st.udev_status.rule_path,
            if st.udev_status.rule_installed && st.udev_status.rule_content_valid {
                "Installed & Valid (0660)"
            } else {
                "Missing / Needs Update"
            }
        ),
        udev_device_access_status: if !st.usb_info.present {
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
        },
        usb_present: st.usb_info.present,
        rx_connected: rx_ok,
        tx_connected_count: tx_cnt,
        overall_status_state,
        overall_status_badge,
        header_subtitle,
        protocol_dialect: st.status.dialect.as_str().to_string(),
        last_sequence: st.status.last_sequence as i32,
        heartbeat_count: st.status.heartbeat_count as i32,
        rx_hardware_model: rx_model.to_string(),
        rx_serial: rx_sn.to_string(),
        rx_firmware: rx_fw.to_string(),
        rx_firmware_and_rev,
        rx_gain_summary,
        rx_subtitle: if !rx_ok {
            "Receiver offline".to_string()
        } else {
            format!("Model {} • S/N {} • FW {}", rx_model, rx_sn, rx_fw)
        },
        tx1: format_tx_slot(st.status.tx.first().and_then(|t| t.as_ref()), rx_ok),
        tx2: format_tx_slot(st.status.tx.get(1).and_then(|t| t.as_ref()), rx_ok),
        setting_nc_mode,
        setting_nc_power,
        nc_overall_summary,
        setting_stereo,
        setting_low_cut: st
            .status
            .settings
            .get("low-cut")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        setting_safety_track: st
            .status
            .settings
            .get("safety-track")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        setting_clip_limiter: st
            .status
            .settings
            .get("clip-limiter")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        setting_leds: st
            .status
            .settings
            .get("mic-leds")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        setting_rx_auto_off: st
            .status
            .settings
            .get("auto-off-15m")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        setting_tx_auto_off: st
            .status
            .settings
            .get("tx-auto-off-15m")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        setting_camera_power: st
            .status
            .settings
            .get("camera-power")
            .cloned()
            .unwrap_or_else(|| "off".to_string()),
        setting_plug_free: st
            .status
            .settings
            .get("plug-free")
            .cloned()
            .unwrap_or_else(|| "on".to_string()),
        alsa_volume_pct: st.alsa_vol.percent,
        alsa_muted: st.alsa_vol.muted,
        ai_nc_mode: st.config.ai_nc_mode.as_str().to_string(),
        ai_nc_strength_pct: st.config.ai_nc_strength_pct,
        ai_nc_virtual_mic: st.config.ai_nc_virtual_mic,
        ai_nc_virtual_mic_active: nl.virtual_mic_active,
        noise_learn_active: nl.learning_active,
        noise_learn_progress_pct: nl.progress_pct,
        noise_learn_status: nl.status_text.clone(),
        noise_profile_learned: p.learned,
        noise_profile_learned_at: p.learned_at.clone(),
        noise_profile_source: p.capture_source.clone(),
        noise_signature: p.noise_signature.clone(),
        noise_floor_summary,
        learned_filter_summary,
        band_low_rumble_str: format!("{:+.1}", p.band_low_rumble_db),
        band_fan_hum_str: format!("{:+.1}", p.band_fan_hum_db),
        band_mid_room_str: format!("{:+.1}", p.band_mid_room_db),
        band_high_hiss_str: format!("{:+.1}", p.band_high_hiss_db),
        adaptive_passes: p.adaptive_passes as i32,
        linux_audio_summary,
        usb_vid_pid: st.usb_info.vid_pid.clone(),
        usb_manufacturer_product: if !st.usb_info.present {
            "Disconnected".to_string()
        } else {
            format!("{} — {}", st.usb_info.manufacturer, st.usb_info.product)
        },
        usb_descriptor_serial: if st.usb_info.usb_serial.is_empty() {
            "—".to_string()
        } else {
            st.usb_info.usb_serial.clone()
        },
        usb_topology: if !st.usb_info.present {
            "—".to_string()
        } else {
            format!(
                "Bus {:03} Device {:03} (Port {})",
                st.usb_info.bus_num, st.usb_info.dev_num, st.usb_info.port_path
            )
        },
        usb_speed_and_power: if !st.usb_info.present {
            "—".to_string()
        } else {
            format!(
                "USB {} • {} • MaxPower {}",
                st.usb_info.usb_version, st.usb_info.speed_mbps, st.usb_info.max_power
            )
        },
        usb_fs_node: if st.usb_info.usbfs_node.is_empty() {
            "—".to_string()
        } else {
            st.usb_info.usbfs_node.clone()
        },
        usb_sysfs_path: if st.usb_info.sysfs_path.is_empty() {
            "—".to_string()
        } else {
            st.usb_info.sysfs_path.clone()
        },
        hid_node_and_phys: if st.usb_info.hidraw_node.is_empty() {
            "—".to_string()
        } else {
            format!("{} ({})", st.usb_info.hidraw_node, st.usb_info.hid_phys)
        },
        hid_service_summary: format!(
            "Report 0x55/0x59 Paired TX Count: {} • Report 0x55/0x6a Index: 0x{:02x}",
            st.status.paired_tx_count_hid.unwrap_or(1),
            st.status.active_tx_index_hid.unwrap_or(0xFF)
        ),
        alsa_card_details: match st.alsa_card.card_index {
            None => "—".to_string(),
            Some(idx) => format!(
                "Card #{} [{}] — {} ({})",
                idx,
                st.alsa_card.card_id.as_deref().unwrap_or(""),
                st.alsa_card.card_name.as_deref().unwrap_or(""),
                st.alsa_card.usb_bus_path.as_deref().unwrap_or("")
            ),
        },
        pipewire_node_details: format!(
            "Node #{} • {}",
            pw_any_id,
            st.pw_info.node_name.as_deref().unwrap_or("—")
        ),
        pipewire_format_state: format!(
            "{} • Stream State: {}",
            st.pw_info
                .sample_spec
                .as_deref()
                .unwrap_or("s24le 2ch 48000Hz"),
            st.pw_info.stream_state.as_deref().unwrap_or("SUSPENDED")
        ),
        last_heartbeat_hex: st
            .status
            .last_heartbeat_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        aux5b05_hex: st
            .status
            .aux_5b05_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        aux0474_hex: st
            .status
            .aux_0474_hex
            .clone()
            .unwrap_or_else(|| "—".to_string()),
        last_action_message: st.last_action_message.clone(),
        event_log_text,
        status_bar_summary,
    }
}

unsafe fn c_str_to_rust(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().to_string()
}

fn rust_to_c_str(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn catch_ffi<T>(default: T, f: impl FnOnce() -> T) -> T {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or(default)
}

/// # Safety
/// `ptr` must be either null or a pointer previously allocated by `CString::into_raw` in this crate.
#[no_mangle]
pub unsafe extern "C" fn rust_free_str(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = CString::from_raw(ptr);
        }));
    }
}

#[no_mangle]
pub extern "C" fn rust_register_callbacks(
    on_state: VoidCallback,
    on_tray: VoidCallback,
    on_window: VoidCallback,
    on_config: VoidCallback,
    on_notify: NotifyCallback,
    on_color_scheme: StrCallback,
) {
    catch_ffi((), || {
        let _ = CB_STATE_CHANGED.set(on_state);
        let _ = CB_TRAY_CHANGED.set(on_tray);
        let _ = CB_WINDOW_CHANGED.set(on_window);
        let _ = CB_CONFIG_CHANGED.set(on_config);
        let _ = CB_NOTIFY.set(on_notify);
        let _ = CB_COLOR_SCHEME.set(on_color_scheme);
    });
}

#[no_mangle]
pub extern "C" fn rust_init_backend(start_minimized: bool, ui_test_mode: bool) {
    catch_ffi((), || {
        let scheme = {
            let mut st = lock_unpoisoned(get_state());
            st.ui_test_mode = ui_test_mode;
            st.window_visible = !start_minimized;
            st.config.color_scheme.clone()
        };

        if ui_test_mode {
            let mut st = lock_unpoisoned(get_state());
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
                dialect: DumlDialect::V1,
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
            start_worker_thread();
        }
        emit_color_scheme(&scheme);
    });
}

#[no_mangle]
pub extern "C" fn rust_get_ui_snapshot_json() -> *mut c_char {
    catch_ffi(std::ptr::null_mut(), || {
        let nl = get_noise_learn_live_state();
        let snap = {
            let st = lock_unpoisoned(get_state());
            build_ui_snapshot_locked(&st, &nl)
        };
        let js = serde_json::to_string(&snap).unwrap_or_else(|_| "{}".to_string());
        rust_to_c_str(&js)
    })
}

#[no_mangle]
pub extern "C" fn rust_get_udev_oneliner() -> *mut c_char {
    catch_ffi(std::ptr::null_mut(), || rust_to_c_str(UDEV_INSTALL_ONELINER))
}

#[no_mangle]
pub extern "C" fn rust_get_full_snapshot_json() -> *mut c_char {
    catch_ffi(std::ptr::null_mut(), || {
        let st = lock_unpoisoned(get_state());
        let js = serde_json::to_string_pretty(&build_full_snapshot_from(&st)).unwrap_or_default();
        rust_to_c_str(&js)
    })
}

#[no_mangle]
pub extern "C" fn rust_set_window_visible(vis: bool) {
    catch_ffi((), || {
        {
            let mut st = lock_unpoisoned(get_state());
            st.window_visible = vis;
        }
        if vis {
            send_worker_cmd(WorkerCmd::ForceStateEmit);
        }
        emit_window_changed();
        emit_tray_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_toggle_window() {
    catch_ffi((), || {
        let vis = {
            let mut st = lock_unpoisoned(get_state());
            st.window_visible = !st.window_visible;
            st.window_visible
        };
        if vis {
            send_worker_cmd(WorkerCmd::ForceStateEmit);
        }
        emit_window_changed();
        emit_tray_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_set_active_tab_index(idx: i32) {
    catch_ffi((), || {
        {
            let mut st = lock_unpoisoned(get_state());
            st.active_tab_index = idx.clamp(0, 4);
        }
        emit_window_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_refresh_now() {
    catch_ffi((), || {
        send_worker_cmd(WorkerCmd::RefreshNow);
    });
}

#[no_mangle]
pub extern "C" fn rust_recheck_udev_status() {
    catch_ffi((), || {
        send_worker_cmd(WorkerCmd::RecheckUdev);
    });
}

#[no_mangle]
pub extern "C" fn rust_notify_udev_copied() {
    catch_ffi((), || {
        {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message = "Copied udev rule install command to clipboard.".to_string();
            log_event(&mut st, "Copied udev setup command to clipboard.");
        }
        emit_state_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_install_udev_rule_interactive() {
    catch_ffi((), || {
        send_worker_cmd(WorkerCmd::InstallUdevRule);
    });
}

/// # Safety
/// `setting_ptr` and `value_ptr` must be either null or valid NUL-terminated UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn rust_apply_hardware_setting(
    setting_ptr: *const c_char,
    value_ptr: *const c_char,
) {
    catch_ffi((), || {
        let setting = c_str_to_rust(setting_ptr);
        let value = c_str_to_rust(value_ptr);
        if find_setting_spec(&setting).is_none() {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message = format!(
                "Error: Setting '{}' is not in the verified allowlist.",
                setting
            );
            drop(st);
            emit_state_changed();
            return;
        }
        send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
            setting,
            value,
            tx_unit: None,
            reply: None,
        });
    });
}

#[no_mangle]
pub extern "C" fn rust_set_system_mic_mute(muted: bool) {
    catch_ffi((), || {
        send_worker_cmd(WorkerCmd::SetSystemMute {
            muted,
            send_popup: false,
            reply: None,
        });
    });
}

#[no_mangle]
pub extern "C" fn rust_set_system_mic_volume(pct: i32) {
    catch_ffi((), || {
        send_worker_cmd(WorkerCmd::SetSystemVolume {
            pct: pct.clamp(0, 100),
            reply: None,
        });
    });
}

#[no_mangle]
pub extern "C" fn rust_learn_noise_profile() {
    catch_ffi((), || {
        let (ui_test, strength, vmic, ai_mode) = {
            let st = lock_unpoisoned(get_state());
            (
                st.ui_test_mode,
                st.config.ai_nc_strength_pct,
                st.config.ai_nc_virtual_mic,
                st.config.ai_nc_mode,
            )
        };
        if ui_test {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message = "Simulated AI Room Noise Learning (UI Test)".to_string();
            drop(st);
            emit_state_changed();
            return;
        }
        let started = trigger_noise_learning_async(strength, vmic, move |prof| {
            if matches!(ai_mode, AiNcMode::LearnedAi | AiNcMode::Adaptive) {
                send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
                    setting: "noise-cancel".to_string(),
                    value: prof.recommended_hw_nc_mode.clone(),
                    tx_unit: None,
                    reply: None,
                });
                send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
                    setting: "low-cut".to_string(),
                    value: prof.recommended_hw_low_cut.clone(),
                    tx_unit: None,
                    reply: None,
                });
            }
            let notif = {
                let mut st = lock_unpoisoned(get_state());
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
                record_notification(
                    &mut st,
                    NotificationEvent::NoiseProfileLearned,
                    "DJI Mic Mini — AI Room Noise Learned",
                    format!(
                        "{} (RNNoise -{:.1} dB • HW NC: {}, Low-Cut: {})",
                        prof.noise_signature,
                        prof.rnnoise_attenuation_db,
                        prof.recommended_hw_nc_mode.to_uppercase(),
                        prof.recommended_hw_low_cut.to_uppercase()
                    ),
                    "low",
                )
            };
            dispatch_notifications(false, &[notif]);
            send_worker_cmd(WorkerCmd::Wake);
        });
        if started {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message =
                "Sampling room acoustics (3s) via RNNoise neural network & spectral analyzer..."
                    .to_string();
            drop(st);
            emit_state_changed();
        }
    });
}

#[no_mangle]
pub extern "C" fn rust_reset_noise_profile() {
    catch_ffi((), || {
        let _ = reset_learned_noise_profile();
        {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message =
                "Reset learned AI room noise profile to baseline defaults.".to_string();
        }
        emit_state_changed();
    });
}

pub fn set_ai_nc_mode_inner(mode: AiNcMode) {
    let ui_test = {
        let mut st = lock_unpoisoned(get_state());
        st.config.ai_nc_mode = mode;
        save_config(&st);
        if st.ui_test_mode {
            st.last_action_message =
                format!("AI Noise Cancellation Mode -> {}", mode.as_str().to_uppercase());
        }
        st.ui_test_mode
    };
    if ui_test {
        emit_config_changed();
        emit_state_changed();
    } else {
        send_worker_cmd(WorkerCmd::SetAiNcMode(mode));
    }
}

/// # Safety
/// `mode_ptr` must be either null or a valid NUL-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn rust_set_ai_nc_mode(mode_ptr: *const c_char) {
    catch_ffi((), || {
        let raw = c_str_to_rust(mode_ptr);
        let valid = AiNcMode::from_str_lossy(&raw);
        set_ai_nc_mode_inner(valid);
    });
}

#[no_mangle]
pub extern "C" fn rust_set_ai_nc_strength(pct: i32) {
    catch_ffi((), || {
        let clamped = pct.clamp(0, 100);
        let ui_test = {
            let mut st = lock_unpoisoned(get_state());
            st.config.ai_nc_strength_pct = clamped;
            save_config(&st);
            st.ui_test_mode
        };
        if ui_test {
            emit_config_changed();
            emit_state_changed();
        } else {
            send_worker_cmd(WorkerCmd::SetAiNcStrength(clamped));
        }
    });
}

#[no_mangle]
pub extern "C" fn rust_set_ai_nc_virtual_mic(enabled: bool) {
    catch_ffi((), || {
        let ui_test = {
            let mut st = lock_unpoisoned(get_state());
            st.config.ai_nc_virtual_mic = enabled;
            save_config(&st);
            if st.ui_test_mode {
                st.last_action_message = format!(
                    "PipeWire AI Virtual Mic -> {}",
                    if enabled { "ON" } else { "OFF" }
                );
            }
            st.ui_test_mode
        };
        if ui_test {
            emit_config_changed();
            emit_state_changed();
        } else {
            send_worker_cmd(WorkerCmd::SetAiNcVirtualMic(enabled));
        }
    });
}

#[no_mangle]
pub extern "C" fn rust_notify_json_copied() {
    catch_ffi((), || {
        {
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message = "Copied full JSON telemetry snapshot to clipboard!".to_string();
        }
        emit_state_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_set_start_on_login(enabled: bool) {
    catch_ffi((), || {
        let (ui_test, start_min) = {
            let mut st = lock_unpoisoned(get_state());
            st.config.start_on_login = enabled;
            save_config(&st);
            (st.ui_test_mode, st.config.start_minimized)
        };
        if ui_test {
            emit_config_changed();
        } else {
            send_worker_cmd(WorkerCmd::SyncStartOnLogin {
                enabled,
                start_minimized: start_min,
            });
        }
    });
}

#[no_mangle]
pub extern "C" fn rust_set_start_minimized(enabled: bool) {
    catch_ffi((), || {
        let (ui_test, start_login) = {
            let mut st = lock_unpoisoned(get_state());
            st.config.start_minimized = enabled;
            save_config(&st);
            (st.ui_test_mode, st.config.start_on_login)
        };
        if ui_test {
            emit_config_changed();
        } else {
            send_worker_cmd(WorkerCmd::SyncStartOnLogin {
                enabled: start_login,
                start_minimized: enabled,
            });
        }
    });
}

fn update_alert_flag(f: impl FnOnce(&mut BackendState)) {
    {
        let mut st = lock_unpoisoned(get_state());
        f(&mut st);
        save_config(&st);
    }
    emit_config_changed();
    emit_tray_changed();
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_rx_disconnect(enabled: bool) {
    catch_ffi((), || update_alert_flag(|st| st.config.alert_on_rx_disconnect = enabled));
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_tx_disconnect(enabled: bool) {
    catch_ffi((), || update_alert_flag(|st| st.config.alert_on_tx_disconnect = enabled));
}

#[no_mangle]
pub extern "C" fn rust_set_alert_on_low_battery(enabled: bool) {
    catch_ffi((), || update_alert_flag(|st| st.config.alert_on_low_battery = enabled));
}

#[no_mangle]
pub extern "C" fn rust_set_low_battery_threshold_pct(val: i32) {
    catch_ffi((), || {
        update_alert_flag(|st| st.config.low_battery_threshold_pct = val.clamp(5, 70))
    });
}

#[no_mangle]
pub extern "C" fn rust_send_test_alert() {
    catch_ffi((), || {
        let (ui_test, notif) = {
            let mut st = lock_unpoisoned(get_state());
            let rx_ok = st.usb_info.present && st.status.connected;
            let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };
            let n = if !rx_ok {
                record_notification(
                    &mut st,
                    NotificationEvent::AlertTest,
                    "DJI Mic Mini — Disconnected Alert (Test)",
                    "The DJI Mic Mini USB-C Receiver (2ca3:4011) is NOT connected.",
                    "critical",
                )
            } else if tx_cnt == 0 {
                record_notification(
                    &mut st,
                    NotificationEvent::AlertTest,
                    "DJI Mic Mini — No Transmitter Linked (Test)",
                    "Receiver RX is connected, but no wireless transmitter (TX1/TX2) is turned on.",
                    "critical",
                )
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
                record_notification(
                    &mut st,
                    NotificationEvent::AlertTest,
                    "DJI Mic Mini — Telemetry Status Popup",
                    format!(
                        "Receiver S/N {} • TX1 Battery {}% (Code {}/7) • Mode {}",
                        rx_sn, tx1_bat, tx1_code, stereo
                    ),
                    "normal",
                )
            };
            (st.ui_test_mode, n)
        };
        dispatch_notifications(ui_test, &[notif]);
        emit_state_changed();
    });
}

/// # Safety
/// `scheme_ptr` must be either null or a valid NUL-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn rust_set_color_scheme(scheme_ptr: *const c_char) {
    catch_ffi((), || {
        let mut s = c_str_to_rust(scheme_ptr);
        if !matches!(s.as_str(), "system" | "dark" | "light") {
            s = "system".to_string();
        }
        {
            let mut st = lock_unpoisoned(get_state());
            st.config.color_scheme = s.clone();
            save_config(&st);
        }
        emit_color_scheme(&s);
        emit_config_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_set_ui_zoom_percent(pct: i32) {
    catch_ffi((), || {
        let rounded = ((pct as f64 / 5.0).round() as i32 * 5).clamp(75, 175);
        {
            let mut st = lock_unpoisoned(get_state());
            st.config.ui_zoom_percent = rounded;
            save_config(&st);
        }
        emit_config_changed();
    });
}

#[no_mangle]
pub extern "C" fn rust_quit_app() {
    catch_ffi((), || {
        {
            let mut st = lock_unpoisoned(get_state());
            st.quitting = true;
        }
        send_worker_cmd(WorkerCmd::Shutdown);
        emit_window_changed();
        let _ = fs::remove_file(socket_path());
    });
}

/// # Safety
/// `shortcut_ptr` must be either null or a valid NUL-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn rust_handle_global_shortcut(shortcut_ptr: *const c_char) {
    catch_ffi((), || {
        let shortcut = c_str_to_rust(shortcut_ptr);
        let _ = execute_shortcut_action(&shortcut, false);
    });
}
