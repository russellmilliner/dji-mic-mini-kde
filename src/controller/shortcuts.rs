use super::alerts::record_notification;
use super::desktop::{dispatch_notifications, NotificationEvent};
use super::ffi::{
    rust_learn_noise_profile, rust_set_ai_nc_virtual_mic, rust_toggle_window, set_ai_nc_mode_inner,
};
use super::worker::{apply_setting_blocking_via_worker, send_worker_cmd, WorkerCmd};
use super::{emit_state_changed, emit_window_changed, get_state};
use crate::util::lock_unpoisoned;
use serde_json::json;
use std::sync::mpsc;
use std::time::Duration;

fn toggle_hw_setting(
    setting: &str,
    default_cur: &str,
    val_a: &str,
    val_b: &str,
    wait_reply: bool,
) -> &'static str {
    let next_val = {
        let st = lock_unpoisoned(get_state());
        let cur = st
            .status
            .settings
            .get(setting)
            .map(|s| s.as_str())
            .unwrap_or(default_cur);
        if cur == val_a {
            val_b
        } else {
            val_a
        }
    };
    let next_static = if next_val == val_a {
        // Store static strings via match for clean return
        val_a.to_string()
    } else {
        val_b.to_string()
    };

    if wait_reply {
        let _ = apply_setting_blocking_via_worker(
            setting.to_string(),
            next_static,
            None,
            Duration::from_secs(3),
        );
    } else {
        send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
            setting: setting.to_string(),
            value: next_static,
            tx_unit: None,
            reply: None,
        });
    }
    if next_val == "on" {
        "on"
    } else if next_val == "off" {
        "off"
    } else if next_val == "stereo" {
        "stereo"
    } else {
        "mono"
    }
}

pub fn execute_shortcut_action(action: &str, wait_reply: bool) -> serde_json::Value {
    match action {
        "_launch" | "show" => {
            {
                let mut st = lock_unpoisoned(get_state());
                st.window_visible = true;
            }
            send_worker_cmd(WorkerCmd::ForceStateEmit);
            emit_window_changed();
            json!({"ok": true})
        }
        "ToggleWindow" | "toggle" => {
            rust_toggle_window();
            json!({"ok": true})
        }
        "ShowBatteryPopup" | "popup-status" => {
            let (ui_test, notif) = {
                let mut st = lock_unpoisoned(get_state());
                let rx_ok = st.usb_info.present && st.status.connected;
                let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };
                let n = if !rx_ok {
                    record_notification(
                        &mut st,
                        NotificationEvent::TelemetrySummary,
                        "DJI Mic Mini — Receiver Disconnected",
                        "The DJI Mic Mini USB-C Receiver (2ca3:4011) is not connected.",
                        "normal",
                    )
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
                    let ai_mode = st.config.ai_nc_mode.as_str().to_uppercase();
                    record_notification(
                        &mut st,
                        NotificationEvent::TelemetrySummary,
                        format!("DJI Mic Mini — {} ({})", tx_str, muted),
                        format!(
                            "RX S/N {} • Active TX: {}/2 • HW NC: {} • AI Mode: {}",
                            rx_sn, tx_cnt, nc_mode, ai_mode
                        ),
                        "normal",
                    )
                };
                (st.ui_test_mode, n)
            };
            dispatch_notifications(ui_test, &[notif]);
            emit_state_changed();
            json!({"ok": true})
        }
        "ToggleMicMute" | "toggle-mute" => {
            let next_mute = !lock_unpoisoned(get_state()).alsa_vol.muted;
            if wait_reply {
                let (tx, rx) = mpsc::channel();
                send_worker_cmd(WorkerCmd::SetSystemMute {
                    muted: next_mute,
                    send_popup: true,
                    reply: Some(tx),
                });
                let _ = rx.recv_timeout(Duration::from_secs(3));
            } else {
                send_worker_cmd(WorkerCmd::SetSystemMute {
                    muted: next_mute,
                    send_popup: true,
                    reply: None,
                });
            }
            json!({"ok": true, "muted": next_mute})
        }
        "ToggleNoiseCancel" | "toggle-nc" => {
            let def = {
                let st = lock_unpoisoned(get_state());
                if st.status.nc_enabled.unwrap_or(true) {
                    "on"
                } else {
                    "off"
                }
            };
            let v = toggle_hw_setting("noise-cancel-power", def, "on", "off", wait_reply);
            json!({"ok": true, "noise_cancel_power": v})
        }
        "NoiseCancelBasic" | "nc-basic" => {
            if wait_reply {
                let _ = apply_setting_blocking_via_worker(
                    "noise-cancel".to_string(),
                    "basic".to_string(),
                    None,
                    Duration::from_secs(3),
                );
            } else {
                send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
                    setting: "noise-cancel".to_string(),
                    value: "basic".to_string(),
                    tx_unit: None,
                    reply: None,
                });
            }
            json!({"ok": true, "noise_cancel": "basic"})
        }
        "NoiseCancelStrong" | "nc-strong" => {
            if wait_reply {
                let _ = apply_setting_blocking_via_worker(
                    "noise-cancel".to_string(),
                    "strong".to_string(),
                    None,
                    Duration::from_secs(3),
                );
            } else {
                send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
                    setting: "noise-cancel".to_string(),
                    value: "strong".to_string(),
                    tx_unit: None,
                    reply: None,
                });
            }
            json!({"ok": true, "noise_cancel": "strong"})
        }
        "CycleAiNoiseCancel" | "cycle-ai-nc" => {
            let next_mode = lock_unpoisoned(get_state()).config.ai_nc_mode.next_cycle();
            set_ai_nc_mode_inner(next_mode);
            json!({"ok": true, "ai_nc_mode": next_mode.as_str()})
        }
        "ToggleAiVirtualMic" | "toggle-virtual-mic" => {
            let next_en = !lock_unpoisoned(get_state()).config.ai_nc_virtual_mic;
            rust_set_ai_nc_virtual_mic(next_en);
            json!({"ok": true, "ai_nc_virtual_mic": next_en})
        }
        "LearnRoomNoise" | "learn-noise" => {
            rust_learn_noise_profile();
            json!({"ok": true})
        }
        "ToggleLowCut" | "toggle-low-cut" => {
            let v = toggle_hw_setting("low-cut", "off", "on", "off", wait_reply);
            json!({"ok": true, "low_cut": v})
        }
        "ToggleClipLimiter" | "toggle-limiter" => {
            let v = toggle_hw_setting("clip-limiter", "on", "on", "off", wait_reply);
            json!({"ok": true, "clip_limiter": v})
        }
        "ToggleTrackMode" | "toggle-stereo" => {
            let v = toggle_hw_setting("stereo", "mono", "stereo", "mono", wait_reply);
            json!({"ok": true, "stereo": v})
        }
        "ToggleSafetyTrack" | "toggle-safety" => {
            let v = toggle_hw_setting("safety-track", "off", "on", "off", wait_reply);
            json!({"ok": true, "safety_track": v})
        }
        "ToggleTransmitterLeds" | "toggle-leds" => {
            let v = toggle_hw_setting("mic-leds", "on", "on", "off", wait_reply);
            json!({"ok": true, "mic_leds": v})
        }
        _ => json!({"error": format!("Unknown action: {}", action)}),
    }
}
