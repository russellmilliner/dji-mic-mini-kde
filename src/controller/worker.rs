use super::alerts::{evaluate_alerts, log_event};
use super::config::{apply_start_on_login, save_config};
use super::desktop::{dispatch_notifications, spawn_pkexec_udev_install};
use super::ffi::{build_tray_digest, build_ui_snapshot_locked, hash_value};
use super::{
    build_full_snapshot_from, emit_config_changed, emit_state_changed, emit_tray_changed,
    get_state, AiNcMode,
};
use crate::audio::{
    get_alsa_card_info, get_alsa_volume, get_noise_learn_live_state, get_pipewire_stream_details,
    is_alsa_card_capturing, maybe_run_adaptive_micro_learn, recompute_profile_for_strength,
    set_alsa_mute, set_alsa_volume, sync_pipewire_ai_virtual_mic,
};
use crate::device::{
    check_udev_status, get_usb_sysfs_info, query_hid_service_info, DjiMicDevice, UDEV_INSTALL_ONELINER,
};
use crate::protocol::{find_setting_spec, DeviceStatus};
use crate::util::{lock_unpoisoned, status_json_path, write_file_atomic_mode};
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub enum WorkerCmd {
    Wake,
    RefreshNow,
    RecheckUdev,
    ForceStateEmit,
    ApplyHardwareSetting {
        setting: String,
        value: String,
        tx_unit: Option<i32>,
        reply: Option<mpsc::Sender<Result<bool, String>>>,
    },
    SetSystemMute {
        muted: bool,
        send_popup: bool,
        reply: Option<mpsc::Sender<bool>>,
    },
    SetSystemVolume {
        pct: i32,
        reply: Option<mpsc::Sender<bool>>,
    },
    SetAiNcMode(AiNcMode),
    SetAiNcStrength(i32),
    SetAiNcVirtualMic(bool),
    InstallUdevRule,
    SyncStartOnLogin {
        enabled: bool,
        start_minimized: bool,
    },
    Shutdown,
}

static WORKER_TX: OnceLock<mpsc::Sender<WorkerCmd>> = OnceLock::new();

pub fn send_worker_cmd(cmd: WorkerCmd) {
    if let Some(tx) = WORKER_TX.get() {
        let _ = tx.send(cmd);
    }
}

pub fn apply_setting_blocking_via_worker(
    setting: String,
    value: String,
    tx_unit: Option<i32>,
    timeout: Duration,
) -> Result<bool, String> {
    if find_setting_spec(&setting).is_none() {
        return Err(format!(
            "Setting '{}' is not in the verified allowlist.",
            setting
        ));
    }
    let (tx, rx) = mpsc::channel();
    send_worker_cmd(WorkerCmd::ApplyHardwareSetting {
        setting,
        value,
        tx_unit,
        reply: Some(tx),
    });
    match rx.recv_timeout(timeout) {
        Ok(res) => res,
        Err(_) => Err("Timed out waiting for USB hardware worker".to_string()),
    }
}

fn apply_setting_on_device(
    dev_slot: &mut Option<DjiMicDevice>,
    setting: &str,
    value: &str,
    tx_unit: Option<i32>,
) -> Result<(bool, bool), String> {
    if find_setting_spec(setting).is_none() {
        return Err(format!(
            "Setting '{}' is not in the verified allowlist.",
            setting
        ));
    }
    let Some(ref mut dev) = *dev_slot else {
        return Err("DJI Mic Mini Receiver is not connected.".to_string());
    };
    if !dev.is_open() {
        return Err("DJI Mic Mini Receiver is not connected.".to_string());
    }
    let ack = dev.set_setting(setting, value, tx_unit)?;
    let updated_status = dev.status.clone();
    let matched = updated_status
        .settings
        .get(setting)
        .map(|v| v == value)
        .unwrap_or(false);

    {
        let mut st = lock_unpoisoned(get_state());
        st.status = updated_status;
        st.last_action_message = format!("Applied {} = {}", setting, value);
        log_event(
            &mut st,
            &format!("Hardware setting updated: {} -> {}", setting, value),
        );
    }
    Ok((ack || matched, matched))
}

pub fn start_worker_thread() {
    let (tx, rx) = mpsc::channel::<WorkerCmd>();
    if WORKER_TX.set(tx).is_err() {
        return;
    }

    let _ = thread::Builder::new()
        .name("dji-usb-worker".to_string())
        .spawn(move || worker_loop(rx));
}

fn worker_loop(rx: mpsc::Receiver<WorkerCmd>) {
    let mut usb_dev: Option<DjiMicDevice> = None;
    let mut first_run = true;

    loop {
        let mut force_slow = first_run;
        let mut force_emit_ui = false;
        let mut force_emit_tray = false;
        let mut shutdown = false;

        first_run = false;

        while let Ok(cmd) = rx.try_recv() {
            handle_cmd(
                cmd,
                &mut usb_dev,
                &mut force_slow,
                &mut force_emit_ui,
                &mut force_emit_tray,
                &mut shutdown,
            );
            if shutdown {
                break;
            }
        }
        if shutdown || lock_unpoisoned(get_state()).quitting {
            if let Some(mut d) = usb_dev.take() {
                d.close();
            }
            break;
        }

        poll_once(&mut usb_dev, force_slow, force_emit_ui, force_emit_tray);

        let wait_dur = {
            let st = lock_unpoisoned(get_state());
            let active_tx = st.usb_info.present && st.status.connected && st.status.tx_count() > 0;
            if st.window_visible || active_tx {
                Duration::from_millis(250)
            } else {
                Duration::from_millis(750)
            }
        };

        match rx.recv_timeout(wait_dur) {
            Ok(cmd) => {
                let mut f_slow = false;
                let mut f_ui = false;
                let mut f_tray = false;
                handle_cmd(
                    cmd,
                    &mut usb_dev,
                    &mut f_slow,
                    &mut f_ui,
                    &mut f_tray,
                    &mut shutdown,
                );
                if shutdown {
                    if let Some(mut d) = usb_dev.take() {
                        d.close();
                    }
                    break;
                }
                if f_slow || f_ui || f_tray {
                    poll_once(&mut usb_dev, f_slow, f_ui, f_tray);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn handle_cmd(
    cmd: WorkerCmd,
    usb_dev: &mut Option<DjiMicDevice>,
    force_slow: &mut bool,
    force_emit_ui: &mut bool,
    force_emit_tray: &mut bool,
    shutdown: &mut bool,
) {
    match cmd {
        WorkerCmd::Wake => {
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::RefreshNow => {
            *force_slow = true;
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::RecheckUdev => {
            let usb_info = get_usb_sysfs_info();
            let udev_status = check_udev_status(Some(&usb_info));
            let mut st = lock_unpoisoned(get_state());
            st.usb_info = usb_info;
            st.udev_status = udev_status.clone();
            st.last_action_message = udev_status.summary.clone();
            log_event(&mut st, &format!("udev status check: {}", udev_status.summary));
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::ForceStateEmit => {
            *force_emit_ui = true;
        }
        WorkerCmd::ApplyHardwareSetting {
            setting,
            value,
            tx_unit,
            reply,
        } => {
            let res = apply_setting_on_device(usb_dev, &setting, &value, tx_unit);
            match &res {
                Ok((_, _)) => {}
                Err(e) => {
                    let mut st = lock_unpoisoned(get_state());
                    st.last_action_message = format!("Error: {e}");
                }
            }
            if let Some(ch) = reply {
                let _ = ch.send(res.map(|(ack_or_match, _)| ack_or_match));
            }
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::SetSystemMute {
            muted,
            send_popup,
            reply,
        } => {
            let ok = set_alsa_mute(muted);
            *force_slow = true;
            *force_emit_ui = true;
            *force_emit_tray = true;
            if let Some(ch) = reply {
                let _ = ch.send(ok);
            }
            if send_popup {
                let mut st = lock_unpoisoned(get_state());
                st.alsa_vol.muted = muted;
                st.last_action_message = format!(
                    "System Microphone -> {}",
                    if muted { "MUTED" } else { "UNMUTED" }
                );
                let notif = super::alerts::record_notification(
                    &mut st,
                    super::desktop::NotificationEvent::MicMuteChanged,
                    format!(
                        "DJI Mic Mini — Microphone {}",
                        if muted { "MUTED" } else { "UNMUTED" }
                    ),
                    format!(
                        "Linux PipeWire/ALSA capture is now {}.",
                        if muted { "MUTED" } else { "LIVE (Unmuted)" }
                    ),
                    "low",
                );
                let ui_test = st.ui_test_mode;
                drop(st);
                dispatch_notifications(ui_test, &[notif]);
            }
        }
        WorkerCmd::SetSystemVolume { pct, reply } => {
            let ok = set_alsa_volume(&format!("{}%", pct.clamp(0, 100)));
            *force_slow = true;
            *force_emit_ui = true;
            *force_emit_tray = true;
            if let Some(ch) = reply {
                let _ = ch.send(ok);
            }
        }
        WorkerCmd::SetAiNcMode(mode) => {
            let (strength, vmic) = {
                let st = lock_unpoisoned(get_state());
                (st.config.ai_nc_strength_pct, st.config.ai_nc_virtual_mic)
            };
            if mode == AiNcMode::Off && vmic {
                let _ = sync_pipewire_ai_virtual_mic(false, false);
            } else if matches!(mode, AiNcMode::LearnedAi | AiNcMode::Adaptive) && vmic {
                let _ = sync_pipewire_ai_virtual_mic(true, false);
            }
            if matches!(mode, AiNcMode::LearnedAi | AiNcMode::Adaptive) {
                let prof = get_noise_learn_live_state().profile;
                if prof.learned {
                    let _ = apply_setting_on_device(
                        usb_dev,
                        "noise-cancel",
                        &prof.recommended_hw_nc_mode,
                        None,
                    );
                    let _ = apply_setting_on_device(
                        usb_dev,
                        "low-cut",
                        &prof.recommended_hw_low_cut,
                        None,
                    );
                }
            }
            if mode == AiNcMode::Adaptive {
                maybe_run_adaptive_micro_learn(strength, vmic);
            }
            {
                let mut st = lock_unpoisoned(get_state());
                st.last_action_message =
                    format!("AI Noise Cancellation Mode -> {}", mode.as_str().to_uppercase());
            }
            emit_config_changed();
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::SetAiNcStrength(clamped) => {
            let (vmic, mode) = {
                let st = lock_unpoisoned(get_state());
                (st.config.ai_nc_virtual_mic, st.config.ai_nc_mode)
            };
            recompute_profile_for_strength(clamped, vmic);
            let prof = get_noise_learn_live_state().profile;
            if matches!(mode, AiNcMode::LearnedAi | AiNcMode::Adaptive) {
                let _ = apply_setting_on_device(
                    usb_dev,
                    "noise-cancel",
                    &prof.recommended_hw_nc_mode,
                    None,
                );
            }
            {
                let mut st = lock_unpoisoned(get_state());
                st.last_action_message = format!(
                    "AI Suppression Strength -> {}% (HP {}Hz, VAD {}%, Rec HW NC={})",
                    clamped,
                    prof.learned_highpass_hz,
                    prof.learned_vad_threshold_pct,
                    prof.recommended_hw_nc_mode.to_uppercase()
                );
            }
            emit_config_changed();
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::SetAiNcVirtualMic(enabled) => {
            let msg = sync_pipewire_ai_virtual_mic(enabled, false).1;
            {
                let mut st = lock_unpoisoned(get_state());
                st.last_action_message = msg;
            }
            emit_config_changed();
            *force_slow = true;
            *force_emit_ui = true;
            *force_emit_tray = true;
        }
        WorkerCmd::InstallUdevRule => {
            let started = spawn_pkexec_udev_install(|ok| {
                if ok {
                    send_worker_cmd(WorkerCmd::RecheckUdev);
                } else {
                    let mut st = lock_unpoisoned(get_state());
                    st.last_action_message =
                        "Polkit cancelled or unavailable — clipboard has manual command: sudo dji-mic udev --install"
                            .to_string();
                    drop(st);
                    send_worker_cmd(WorkerCmd::Wake);
                }
            });
            if !started {
                let _ = UDEV_INSTALL_ONELINER;
                let mut st = lock_unpoisoned(get_state());
                st.last_action_message =
                    "pkexec not found — clipboard has manual command: sudo dji-mic udev --install"
                        .to_string();
            } else {
                let mut st = lock_unpoisoned(get_state());
                st.last_action_message =
                    "Requested PolicyKit authorization to install hardened 0660 udev rules..."
                        .to_string();
            }
            *force_emit_ui = true;
        }
        WorkerCmd::SyncStartOnLogin {
            enabled,
            start_minimized,
        } => {
            let (_, msg) = apply_start_on_login(enabled, start_minimized);
            let mut st = lock_unpoisoned(get_state());
            st.last_action_message = msg;
            save_config(&st);
            drop(st);
            emit_config_changed();
            *force_emit_ui = true;
        }
        WorkerCmd::Shutdown => {
            *shutdown = true;
        }
    }
}

fn poll_once(
    usb_dev: &mut Option<DjiMicDevice>,
    force_slow: bool,
    force_emit_ui: bool,
    force_emit_tray: bool,
) {
    let now = Instant::now();
    let (ui_test_mode, quitting, slow_due) = {
        let st = lock_unpoisoned(get_state());
        let slow_interval = if st.window_visible || (st.usb_info.present && st.status.tx_count() > 0)
        {
            Duration::from_millis(3000)
        } else {
            Duration::from_millis(6000)
        };
        (
            st.ui_test_mode,
            st.quitting,
            force_slow || now.duration_since(st.last_slow_poll) >= slow_interval,
        )
    };
    if ui_test_mode || quitting {
        return;
    }

    if slow_due {
        let usb_info = get_usb_sysfs_info();
        let udev_status = check_udev_status(Some(&usb_info));
        let alsa_card = get_alsa_card_info();
        let alsa_capturing = is_alsa_card_capturing(alsa_card.card_index);
        let alsa_vol = get_alsa_volume();
        let pw_info = get_pipewire_stream_details();

        let (ai_mode, ai_strength, ai_vmic) = {
            let mut st = lock_unpoisoned(get_state());
            st.last_slow_poll = now;
            st.usb_info = usb_info;
            st.udev_status = udev_status;
            st.alsa_card = alsa_card;
            st.alsa_capturing = alsa_capturing;
            if let Some(v) = alsa_vol {
                st.alsa_vol = v;
            }
            st.pw_info = pw_info;
            (
                st.config.ai_nc_mode,
                st.config.ai_nc_strength_pct,
                st.config.ai_nc_virtual_mic,
            )
        };
        if ai_mode == AiNcMode::Adaptive {
            maybe_run_adaptive_micro_learn(ai_strength, ai_vmic);
        }
    } else {
        // Light /proc read for active mic capture indicator only when RX connected
        let card_idx_opt = {
            let st = lock_unpoisoned(get_state());
            if st.usb_info.present && st.status.connected {
                Some(st.alsa_card.card_index)
            } else {
                None
            }
        };
        if let Some(card_idx) = card_idx_opt {
            let cap = is_alsa_card_capturing(card_idx);
            let mut st = lock_unpoisoned(get_state());
            st.alsa_capturing = cap;
        }
    }

    let (present, hidraw_node) = {
        let st = lock_unpoisoned(get_state());
        (st.usb_info.present, st.usb_info.hidraw_node.clone())
    };

    if !present {
        if let Some(mut dev) = usb_dev.take() {
            dev.close();
        }
        let mut st = lock_unpoisoned(get_state());
        st.status = DeviceStatus::default();
    } else {
        if usb_dev.as_ref().map(|d| !d.is_open()).unwrap_or(true) {
            let mut dev = DjiMicDevice::new();
            if dev.open().is_ok() {
                if !hidraw_node.is_empty() {
                    let hid_info = query_hid_service_info(&hidraw_node);
                    dev.status.paired_tx_count_hid = hid_info.get("paired_tx_count").copied();
                    dev.status.active_tx_index_hid = hid_info.get("active_tx_index").copied();
                }
                *usb_dev = Some(dev);
            } else {
                dev.close();
                *usb_dev = None;
                let mut st = lock_unpoisoned(get_state());
                st.status = DeviceStatus::default();
            }
        }

        if let Some(ref mut dev) = *usb_dev {
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
                *usb_dev = None;
                let usb_info = get_usb_sysfs_info();
                let udev_status = check_udev_status(Some(&usb_info));
                let mut st = lock_unpoisoned(get_state());
                st.usb_info = usb_info;
                st.udev_status = udev_status;
                st.status = DeviceStatus::default();
            } else {
                let mut st = lock_unpoisoned(get_state());
                st.status = dev.status.clone();
            }
        }
    }

    let nl = get_noise_learn_live_state();
    let (pending_notifs, emit_tray, emit_ui, status_json_to_write) = {
        let mut st = lock_unpoisoned(get_state());
        let pending = evaluate_alerts(&mut st);

        let snap = build_ui_snapshot_locked(&st, &nl);
        let ui_digest = hash_value(&snap);
        let tray_digest = build_tray_digest(&snap);

        let tray_changed = force_emit_tray || tray_digest != st.last_tray_digest;
        if tray_changed {
            st.last_tray_digest = tray_digest;
        }

        let ui_changed = ui_digest != st.last_state_digest;
        if ui_changed {
            st.last_state_digest = ui_digest;
        }
        let should_emit_ui = st.window_visible && (force_emit_ui || ui_changed);

        let json_due = !st.ui_test_mode
            && (ui_digest != st.last_json_digest)
            && st
                .last_status_write
                .map(|t| t.elapsed() >= Duration::from_secs(1))
                .unwrap_or(true);
        let json_bytes = if json_due {
            st.last_json_digest = ui_digest;
            st.last_status_write = Some(Instant::now());
            serde_json::to_vec_pretty(&build_full_snapshot_from(&st)).ok()
        } else {
            None
        };

        (pending, tray_changed, should_emit_ui, json_bytes)
    };

    if let Some(bytes) = status_json_to_write {
        let _ = write_file_atomic_mode(&status_json_path(), &bytes, 0o600);
    }

    dispatch_notifications(ui_test_mode, &pending_notifs);
    if emit_tray {
        emit_tray_changed();
    }
    if emit_ui {
        emit_state_changed();
    }
}
