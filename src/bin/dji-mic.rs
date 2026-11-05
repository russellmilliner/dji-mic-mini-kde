use dji_mic_mini_kde::audio::{
    get_alsa_card_info, get_alsa_volume, get_pipewire_stream_details, set_alsa_mute,
    set_alsa_volume, toggle_alsa_mute,
};
use dji_mic_mini_kde::controller::socket_path;
use dji_mic_mini_kde::device::{
    check_udev_status, get_usb_sysfs_info, install_udev_rule_as_root, query_hid_service_info,
    DjiMicDevice, UDEV_INSTALL_ONELINER, UDEV_RULE_CONTENT, UDEV_RULE_PATH,
};
use dji_mic_mini_kde::protocol::{find_setting_spec, hex_encode, DeviceStatus, SETTINGS_SPEC};
use serde_json::json;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

fn query_tray_socket(req: &serde_json::Value) -> Option<serde_json::Value> {
    let sp = socket_path();
    if !sp.exists() {
        return None;
    }
    let mut stream = UnixStream::connect(&sp).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .ok()?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1200)))
        .ok()?;
    let payload = format!("{}\n", serde_json::to_string(req).ok()?);
    stream.write_all(payload.as_bytes()).ok()?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).ok()?;
    serde_json::from_slice(&resp).ok()
}

fn build_direct_snapshot() -> serde_json::Value {
    if let Some(tray_snap) = query_tray_socket(&json!({"action": "status"})) {
        if tray_snap.get("usb").is_some() {
            return tray_snap;
        }
    }

    let usb_info = get_usb_sysfs_info();
    let udev_status = check_udev_status(Some(&usb_info));
    let alsa_card = get_alsa_card_info();
    let vol = get_alsa_volume();
    let pw = get_pipewire_stream_details();

    let mut status = DeviceStatus::default();
    if usb_info.present {
        let mut dev = DjiMicDevice::new();
        if dev.open().is_ok() {
            status = dev.collect_snapshot(550);
            if !usb_info.hidraw_node.is_empty() {
                let hid_info = query_hid_service_info(&usb_info.hidraw_node);
                status.paired_tx_count_hid = hid_info.get("paired_tx_count").copied();
                status.active_tx_index_hid = hid_info.get("active_tx_index").copied();
            }
            dev.close();
        }
    }

    json!({
        "usb": usb_info,
        "udev": udev_status,
        "duml": status,
        "audio": {
            "alsa_card": alsa_card,
            "volume": vol,
            "pipewire": pw,
        }
    })
}

fn print_human_status(snap: &serde_json::Value) {
    let usb = &snap["usb"];
    let udev = &snap["udev"];
    let duml = &snap["duml"];
    let aud = &snap["audio"];

    println!("========================================================================");
    println!("  KDE DJI Mic Mini Control (Rust) — Full Hardware & Telemetry Report");
    println!("========================================================================");

    let udev_ready = udev["ready"].as_bool().unwrap_or(false);
    println!(
        "  udev Rule Status: {} ({})",
        if udev_ready { "READY" } else { "NEEDS SETUP" },
        udev["summary"].as_str().unwrap_or("")
    );

    if !usb["present"].as_bool().unwrap_or(false) {
        println!("  USB Status      : DISCONNECTED (2ca3:4011 not present on USB bus)");
        if !udev_ready {
            println!("  Install Rule    : sudo dji-mic udev --install");
        }
        println!("========================================================================");
        return;
    }

    println!(
        "  USB Device      : {} — {} ({})",
        usb["manufacturer"].as_str().unwrap_or(""),
        usb["product"].as_str().unwrap_or(""),
        usb["vid_pid"].as_str().unwrap_or("2ca3:4011")
    );
    println!(
        "  USB Topology    : Bus {:03} Dev {:03} (Port {}) • USB {} @ {} (MaxPower {})",
        usb["bus_num"].as_i64().unwrap_or(0),
        usb["dev_num"].as_i64().unwrap_or(0),
        usb["port_path"].as_str().unwrap_or(""),
        usb["usb_version"].as_str().unwrap_or(""),
        usb["speed_mbps"].as_str().unwrap_or(""),
        usb["max_power"].as_str().unwrap_or("")
    );
    println!(
        "  Device Nodes    : USBFS {} • HID {} ({})",
        usb["usbfs_node"].as_str().unwrap_or("—"),
        usb["hidraw_node"].as_str().unwrap_or("—"),
        usb["hid_phys"].as_str().unwrap_or("—")
    );
    println!(
        "  USB String S/N  : {}",
        usb["usb_serial"].as_str().unwrap_or("—")
    );
    println!("------------------------------------------------------------------------");

    if !duml["connected"].as_bool().unwrap_or(false) {
        println!("  DUML Interface 6: DISCONNECTED / PERMISSION DENIED");
        println!("  Fix Permissions : sudo dji-mic udev --install");
        println!("========================================================================");
        return;
    }

    let rx = &duml["rx"];
    println!(
        "  Receiver (RX)   : {} • S/N {} • FW {} (HW Rev {})",
        rx["hardware_model"].as_str().unwrap_or("DJI MIC MINI RX"),
        rx["serial"].as_str().unwrap_or("—"),
        rx["firmware"].as_str().unwrap_or("—"),
        rx["hardware_revision"].as_str().unwrap_or("—")
    );
    let gain_db = duml["gain_dial_db"].as_i64().unwrap_or(0);
    println!(
        "  Protocol / Gain : DUML {} (Seq #{}, {} frames) • RX Gain Dial: {:+} dB (Step {}/5)",
        duml["dialect"].as_str().unwrap_or("v1"),
        duml["last_sequence"].as_i64().unwrap_or(0),
        duml["heartbeat_count"].as_i64().unwrap_or(0),
        gain_db,
        rx["gain_step_code"]
            .as_i64()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
    );
    println!("------------------------------------------------------------------------");

    if let Some(tx_list) = duml["tx"].as_array() {
        for (idx, t) in tx_list.iter().enumerate() {
            let unit_num = idx + 1;
            if t.is_null() || !t["connected"].as_bool().unwrap_or(false) {
                println!(
                    "  Transmitter TX{}: DISCONNECTED / POWERED OFF",
                    unit_num
                );
            } else {
                println!(
                    "  Transmitter TX{}: CONNECTED ({}) • S/N {} • FW {}",
                    unit_num,
                    t["product_name"].as_str().unwrap_or("DJI Mic Mini TX"),
                    t["serial"].as_str().unwrap_or("—"),
                    t["firmware"].as_str().unwrap_or("—")
                );
                println!(
                    "    Battery & VU  : {}% (Gauge {}/7, Charging: {}) • Live VU Meter: {}/255",
                    t["battery_percent"].as_i64().unwrap_or(100),
                    t["battery"].as_i64().unwrap_or(1),
                    if t["charging"].as_bool().unwrap_or(false) {
                        "YES"
                    } else {
                        "NO"
                    },
                    t["level"].as_i64().unwrap_or(0)
                );
                println!(
                    "    TX Features   : NC={} ({}) • Low-Cut={} • Auto-Off 15m={} • LEDs={} • Tone={}",
                    if t["nc_enabled"].as_bool().unwrap_or(false) {
                        "ON"
                    } else {
                        "OFF"
                    },
                    t["nc_mode"].as_str().unwrap_or("basic").to_uppercase(),
                    if t["low_cut"].as_bool().unwrap_or(false) {
                        "ON"
                    } else {
                        "OFF"
                    },
                    if t["auto_off_15m"].as_bool().unwrap_or(false) {
                        "ON"
                    } else {
                        "OFF"
                    },
                    if t["leds_enabled"].as_bool().unwrap_or(true) {
                        "ON"
                    } else {
                        "OFF"
                    },
                    t["voice_tone"].as_str().unwrap_or("standard")
                );
            }
        }
    }

    println!("------------------------------------------------------------------------");
    let st = &duml["settings"];
    println!(
        "  Active Settings : Channels={} • NC={} ({}) • Low-Cut={} • Safety-Track={}",
        st["stereo"].as_str().unwrap_or("mono").to_uppercase(),
        st["noise-cancel-power"]
            .as_str()
            .unwrap_or("off")
            .to_uppercase(),
        st["noise-cancel"]
            .as_str()
            .unwrap_or("basic")
            .to_uppercase(),
        st["low-cut"].as_str().unwrap_or("off").to_uppercase(),
        st["safety-track"].as_str().unwrap_or("off").to_uppercase()
    );
    println!(
        "                    Limiter={} • LEDs={} • RX-AutoOff={} • Camera-Power={} • Plug-Free={}",
        st["clip-limiter"].as_str().unwrap_or("on").to_uppercase(),
        st["mic-leds"].as_str().unwrap_or("on").to_uppercase(),
        st["auto-off-15m"].as_str().unwrap_or("on").to_uppercase(),
        st["camera-power"].as_str().unwrap_or("off").to_uppercase(),
        st["plug-free"].as_str().unwrap_or("on").to_uppercase()
    );
    println!("------------------------------------------------------------------------");

    let ac = &aud["alsa_card"];
    let av = &aud["volume"];
    let pw = &aud["pipewire"];
    println!(
        "  Linux Audio     : ALSA Card #{} [{}] • PipeWire Node #{} ({}) • Vol {}% ({}) [{}]",
        ac["card_index"]
            .as_i64()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string()),
        ac["card_id"].as_str().unwrap_or("MINI"),
        pw["wpctl_id"]
            .as_i64()
            .or_else(|| pw["pactl_id"].as_i64())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        pw["stream_state"].as_str().unwrap_or("IDLE"),
        av["percent"].as_i64().unwrap_or(100),
        av["db"].as_str().unwrap_or("0dB"),
        if av["muted"].as_bool().unwrap_or(false) {
            "MUTED"
        } else {
            "UNMUTED"
        }
    );
    println!("========================================================================");
}

fn print_usage() {
    println!("Usage: dji-mic <command> [options]");
    println!("Commands:");
    println!("  status [--json]                     Show complete hardware, battery, VU & audio status");
    println!("  watch [--interval SEC] [--json]     Continuously stream live telemetry");
    println!("  set <setting> [value] [--tx 1|2]    Change a verified DJI Mic Mini hardware setting");
    println!("  mute [on|off|toggle]                Mute or unmute Linux ALSA/PipeWire capture");
    println!("  volume [0-100%|+5%|-5%]             Get or set Linux ALSA capture volume");
    println!("  udev [--check|--print-rule|--install] Check or install /etc/udev/rules.d/99-dji-mic.rules");
    println!("  raw --cmd-id HEX [--cmd-set HEX] [--payload HEX] Send raw DUML frame");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("status");

    match cmd {
        "-h" | "--help" | "help" => {
            print_usage();
        }
        "status" => {
            let as_json = args.iter().any(|a| a == "--json");
            let snap = build_direct_snapshot();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&snap).unwrap_or_default()
                );
            } else {
                print_human_status(&snap);
            }
        }
        "watch" => {
            let as_json = args.iter().any(|a| a == "--json");
            let mut interval = 0.25f64;
            for i in 0..args.len() {
                if args[i] == "--interval" && i + 1 < args.len() {
                    if let Ok(v) = args[i + 1].parse::<f64>() {
                        interval = v.max(0.05);
                    }
                }
            }
            let mut dev = DjiMicDevice::new();
            if let Err(e) = dev.open() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            loop {
                let deadline =
                    Instant::now() + Duration::from_secs_f64(interval);
                while Instant::now() < deadline {
                    if dev.poll_frames(40).is_err() {
                        break;
                    }
                }
                if as_json {
                    println!("{}", serde_json::to_string(&dev.status).unwrap_or_default());
                } else {
                    let s = &dev.status;
                    let rx_sn = s
                        .rx
                        .as_ref()
                        .and_then(|r| r.serial.as_deref())
                        .unwrap_or("—");
                    let mut parts = vec![format!(
                        "[Seq {:5}] RX({}) Gain {:+}dB",
                        s.last_sequence,
                        rx_sn,
                        s.gain_dial_db.unwrap_or(0)
                    )];
                    for (idx, t_opt) in s.tx.iter().enumerate() {
                        if let Some(t) = t_opt {
                            if t.connected {
                                parts.push(format!(
                                    "TX{}: Bat {:3}% ({}/7) VU {:3}/255 NC={}",
                                    idx + 1,
                                    t.battery_percent.unwrap_or(100),
                                    t.battery.unwrap_or(1),
                                    t.level.unwrap_or(0),
                                    if t.nc_enabled.unwrap_or(false) {
                                        t.nc_mode.as_deref().unwrap_or("on")
                                    } else {
                                        "off"
                                    }
                                ));
                            } else {
                                parts.push(format!("TX{}: OFF", idx + 1));
                            }
                        } else {
                            parts.push(format!("TX{}: OFF", idx + 1));
                        }
                    }
                    println!("{}", parts.join(" | "));
                }
            }
        }
        "set" => {
            if args.len() < 2 {
                eprintln!("Available settings:");
                for s in SETTINGS_SPEC {
                    eprintln!("  {:16} — {}", s.name, s.description);
                }
                std::process::exit(1);
            }
            let setting = &args[1];
            if find_setting_spec(setting).is_none() {
                eprintln!("Unknown setting '{}'", setting);
                std::process::exit(1);
            }
            let mut value = "on".to_string();
            let mut tx_unit: Option<i32> = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--tx" && i + 1 < args.len() {
                    tx_unit = args[i + 1].parse().ok();
                    i += 2;
                } else if !args[i].starts_with("--") {
                    value = args[i].clone();
                    i += 1;
                } else {
                    i += 1;
                }
            }

            if let Some(resp) = query_tray_socket(&json!({
                "action": "set",
                "setting": setting,
                "value": value,
                "tx": tx_unit,
            })) {
                if resp["ok"].as_bool().unwrap_or(false) {
                    println!("Applied {} = {} (via tray daemon)", setting, value);
                    return;
                } else if let Some(err) = resp["error"].as_str() {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            }

            let mut dev = DjiMicDevice::new();
            if let Err(e) = dev.open() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            match dev.set_setting(setting, &value, tx_unit) {
                Ok(_) => println!("Applied {} = {}", setting, value),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "mute" => {
            let state = args.get(1).map(|s| s.as_str()).unwrap_or("toggle");
            let res = match state {
                "toggle" => toggle_alsa_mute(),
                "on" => {
                    set_alsa_mute(true);
                    Some(true)
                }
                "off" => {
                    set_alsa_mute(false);
                    Some(false)
                }
                _ => None,
            };
            if let Some(m) = res {
                println!(
                    "DJI Mic Mini Capture: {}",
                    if m { "MUTED" } else { "UNMUTED" }
                );
            } else {
                eprintln!("DJI Mic Mini ALSA card not found.");
                std::process::exit(1);
            }
        }
        "volume" => {
            if let Some(level) = args.get(1) {
                if !set_alsa_volume(level) {
                    eprintln!("Failed to set volume to {}", level);
                    std::process::exit(1);
                }
            }
            if let Some(v) = get_alsa_volume() {
                println!(
                    "DJI Mic Mini Volume: {}% ({}) [{}]",
                    v.percent,
                    v.db.unwrap_or_else(|| "0dB".to_string()),
                    if v.muted { "MUTED" } else { "UNMUTED" }
                );
            } else {
                eprintln!("DJI Mic Mini ALSA card not found.");
                std::process::exit(1);
            }
        }
        "udev" => {
            let as_json = args.iter().any(|a| a == "--json");
            if args.iter().any(|a| a == "--print-rule") {
                print!("{}", UDEV_RULE_CONTENT);
                return;
            }
            if args.iter().any(|a| a == "--install") {
                let (ok, msg) = install_udev_rule_as_root();
                println!("{}", msg);
                std::process::exit(if ok { 0 } else { 1 });
            }
            let st = check_udev_status(None);
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&st).unwrap_or_default()
                );
            } else {
                println!("udev Rule File  : {}", UDEV_RULE_PATH);
                println!(
                    "Rule Installed  : {}",
                    if st.rule_installed && st.rule_content_valid {
                        "YES"
                    } else {
                        "NO"
                    }
                );
                println!(
                    "USBFS Node      : {} (Writable: {})",
                    if st.usbfs_node.is_empty() {
                        "—"
                    } else {
                        &st.usbfs_node
                    },
                    if st.usbfs_writable { "YES" } else { "NO" }
                );
                println!(
                    "HID Node        : {} (Writable: {})",
                    if st.hidraw_node.is_empty() {
                        "—"
                    } else {
                        &st.hidraw_node
                    },
                    if st.hidraw_writable { "YES" } else { "NO" }
                );
                println!("Summary         : {}", st.summary);
                if !st.ready {
                    println!("\nTo install the udev rule, run:");
                    println!("  sudo dji-mic udev --install");
                    println!("Or:");
                    println!("  {}", UDEV_INSTALL_ONELINER);
                }
            }
        }
        "raw" => {
            let mut cmd_set = 0x5Bu8;
            let mut cmd_id = 0x01u8;
            let mut payload = Vec::new();
            let mut dialect: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--cmd-set" if i + 1 < args.len() => {
                        let s = args[i + 1].trim_start_matches("0x");
                        cmd_set = u8::from_str_radix(s, 16).unwrap_or(0x5B);
                        i += 2;
                    }
                    "--cmd-id" if i + 1 < args.len() => {
                        let s = args[i + 1].trim_start_matches("0x");
                        cmd_id = u8::from_str_radix(s, 16).unwrap_or(0x01);
                        i += 2;
                    }
                    "--payload" if i + 1 < args.len() => {
                        let clean: String = args[i + 1]
                            .chars()
                            .filter(|c| !c.is_whitespace())
                            .collect();
                        for chunk in clean.as_bytes().chunks(2) {
                            if chunk.len() == 2 {
                                if let Ok(b) =
                                    u8::from_str_radix(&String::from_utf8_lossy(chunk), 16)
                                {
                                    payload.push(b);
                                }
                            }
                        }
                        i += 2;
                    }
                    "--dialect" if i + 1 < args.len() => {
                        dialect = Some(args[i + 1].clone());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            let mut dev = DjiMicDevice::new();
            if let Err(e) = dev.open() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            let _ = thread::sleep(Duration::from_millis(10));
            match dev.send_raw_command(
                cmd_set,
                cmd_id,
                &payload,
                dialect.as_deref(),
                0x02,
                0x5A,
                0x40,
            ) {
                Ok(replies) => {
                    for f in replies {
                        println!(
                            "[{}] set=0x{:02x} id=0x{:02x} seq={} payload={}",
                            f.dialect,
                            f.cmd_set,
                            f.cmd_id,
                            f.seq,
                            hex_encode(&f.payload)
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}
