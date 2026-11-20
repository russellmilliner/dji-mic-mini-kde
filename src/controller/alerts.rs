use super::desktop::{NotificationEvent, PendingNotification};
use super::BackendState;
use crate::util::now_hms;

pub fn log_event(st: &mut BackendState, msg: &str) {
    let line = format!("[{}] {}", now_hms(), msg);
    st.event_log.push_front(line);
    if st.event_log.len() > 40 {
        st.event_log.truncate(40);
    }
}

pub fn record_notification(
    st: &mut BackendState,
    event: NotificationEvent,
    title: impl Into<String>,
    body: impl Into<String>,
    urgency: &'static str,
) -> PendingNotification {
    let title = title.into();
    let body = body.into();
    log_event(st, &format!("{title}: {body}"));
    PendingNotification {
        event,
        title,
        body,
        urgency,
    }
}

pub fn evaluate_alerts(st: &mut BackendState) -> Vec<PendingNotification> {
    let mut pending = Vec::new();
    let rx_ok = st.usb_info.present && st.status.connected;
    let tx_cnt = if rx_ok { st.status.tx_count() } else { 0 };

    if st.usb_info.present && !st.status.connected {
        return pending;
    }

    if !st.initial_check_done {
        st.initial_check_done = true;
        st.prev_rx_connected = Some(rx_ok);
        st.prev_tx_count = Some(tx_cnt);
        if !rx_ok && st.config.alert_on_rx_disconnect {
            pending.push(record_notification(
                st,
                NotificationEvent::RxDisconnected,
                "DJI Mic Mini Disconnected",
                "The DJI Mic Mini USB-C Receiver (2ca3:4011) is not connected to this PC.",
                "critical",
            ));
        } else if rx_ok && tx_cnt == 0 && st.config.alert_on_tx_disconnect {
            pending.push(record_notification(
                st,
                NotificationEvent::TxDisconnected,
                "DJI Mic Mini Transmitter Disconnected",
                "The USB-C Receiver is plugged in, but no wireless microphone transmitter (TX1/TX2) is powered on or linked.",
                "critical",
            ));
        } else {
            log_event(
                st,
                &format!(
                    "Monitoring started — Receiver connected, {} transmitter(s) active.",
                    tx_cnt
                ),
            );
        }
        return pending;
    }

    if st.prev_rx_connected == Some(true) && !rx_ok {
        if st.config.alert_on_rx_disconnect {
            pending.push(record_notification(
                st,
                NotificationEvent::RxDisconnected,
                "DJI Mic Mini USB Receiver Disconnected",
                "The DJI Mic Mini USB-C Receiver (2ca3:4011) was unplugged.",
                "critical",
            ));
        }
    } else if st.prev_rx_connected == Some(false) && rx_ok {
        let rx_sn = st
            .status
            .rx
            .as_ref()
            .and_then(|r| r.serial.as_deref())
            .unwrap_or("—")
            .to_string();
        pending.push(record_notification(
            st,
            NotificationEvent::RxConnected,
            "DJI Mic Mini Receiver Connected",
            format!(
                "USB-C Receiver (S/N {}) connected ({} transmitter(s) linked).",
                rx_sn, tx_cnt
            ),
            "normal",
        ));
    }

    if rx_ok && st.prev_rx_connected == Some(true) {
        let prev_tx = st.prev_tx_count.unwrap_or(0);
        if prev_tx > 0 && tx_cnt == 0 {
            if st.config.alert_on_tx_disconnect {
                pending.push(record_notification(
                    st,
                    NotificationEvent::TxDisconnected,
                    "DJI Mic Mini Transmitter Disconnected",
                    "All DJI Mic Mini wireless transmitters (TX1/TX2) powered off or lost link.",
                    "critical",
                ));
            }
        } else if prev_tx == 0 && tx_cnt > 0 {
            let mut summary = Vec::new();
            for t in st.status.tx.iter().flatten() {
                if t.connected {
                    summary.push(format!("TX{}: {}%", t.unit, t.battery_percent.unwrap_or(100)));
                }
            }
            pending.push(record_notification(
                st,
                NotificationEvent::TxConnected,
                "DJI Mic Mini Transmitter Connected",
                format!("Wireless link active ({}).", summary.join(", ")),
                "normal",
            ));
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
                        pending.push(record_notification(
                            st,
                            NotificationEvent::LowBattery,
                            format!("DJI Mic Mini TX{} Low Battery ({}%)", t.unit, pct),
                            format!(
                                "Transmitter TX{} (S/N {}) battery is at {}% (gauge {}/7).",
                                t.unit,
                                t.serial.as_deref().unwrap_or("—"),
                                pct,
                                t.battery.unwrap_or(1)
                            ),
                            "critical",
                        ));
                    } else if pct > thresh {
                        st.low_bat_alerted[unit_idx] = false;
                    }
                }
            }
        }
    }

    st.prev_rx_connected = Some(rx_ok);
    st.prev_tx_count = Some(tx_cnt);
    pending
}
