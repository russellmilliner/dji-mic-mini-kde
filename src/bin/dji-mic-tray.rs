use dji_mic_mini_kde::controller::read_start_minimized_config;
use dji_mic_mini_kde::util::{data_app_dir, send_ipc_request};
use serde_json::json;
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

extern "C" {
    fn cpp_run_dji_mic_tray(
        argc: c_int,
        argv: *mut *mut c_char,
        start_minimized: bool,
        ui_test_mode: bool,
        app_dir: *const c_char,
    ) -> c_int;
}

fn send_socket_command(cmd: &serde_json::Value) -> Option<serde_json::Value> {
    send_ipc_request(cmd, Duration::from_millis(1500))
}

fn locate_app_dir() -> String {
    if let Ok(env_dir) = std::env::var("DJI_MIC_APP_DIR") {
        let p = PathBuf::from(env_dir.trim());
        if p.join("qml/Main.qml").exists() {
            return p.to_string_lossy().to_string();
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("qml/Main.qml").exists() {
                return parent.to_string_lossy().to_string();
            }
            if let Some(gp) = parent.parent() {
                if gp.join("qml/Main.qml").exists() {
                    return gp.to_string_lossy().to_string();
                }
                let share = gp.join("share/dji-mic-mini-kde");
                if share.join("qml/Main.qml").exists() {
                    return share.to_string_lossy().to_string();
                }
            }
        }
    }

    let xdg_app = data_app_dir();
    if xdg_app.join("qml/Main.qml").exists() {
        return xdg_app.to_string_lossy().to_string();
    }

    let static_candidates = ["/app/share/dji-mic-mini-kde", "/usr/share/dji-mic-mini-kde"];
    for c in static_candidates {
        if PathBuf::from(c).join("qml/Main.qml").exists() {
            return c.to_string();
        }
    }

    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        if PathBuf::from(manifest_dir).join("qml/Main.qml").exists() {
            return manifest_dir.to_string();
        }
    }

    xdg_app.to_string_lossy().to_string()
}

fn find_cli_binary() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("dji-mic");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    PathBuf::from("dji-mic")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let minimized = args.iter().any(|a| a == "--minimized");
    let show = args.iter().any(|a| a == "--show");
    let toggle = args.iter().any(|a| a == "--toggle");
    let status = args.iter().any(|a| a == "--status");
    let ui_test = args.iter().any(|a| a == "--ui-test");

    if !ui_test {
        if status {
            if let Some(resp) = send_socket_command(&json!({"action": "status"})) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&resp).unwrap_or_default()
                );
                std::process::exit(0);
            }
            let status_code = Command::new(find_cli_binary())
                .args(["status", "--json"])
                .status()
                .map(|s| s.code().unwrap_or(1))
                .unwrap_or(1);
            std::process::exit(status_code);
        }

        if show && send_socket_command(&json!({"action": "show"})).is_some() {
            std::process::exit(0);
        }

        if toggle && send_socket_command(&json!({"action": "toggle"})).is_some() {
            std::process::exit(0);
        }

        let shortcut_flags = [
            ("--popup-status", "popup-status"),
            ("--toggle-mute", "toggle-mute"),
            ("--toggle-nc", "toggle-nc"),
            ("--nc-basic", "nc-basic"),
            ("--nc-strong", "nc-strong"),
            ("--cycle-ai-nc", "cycle-ai-nc"),
            ("--toggle-virtual-mic", "toggle-virtual-mic"),
            ("--learn-noise", "learn-noise"),
            ("--toggle-low-cut", "toggle-low-cut"),
            ("--toggle-limiter", "toggle-limiter"),
            ("--toggle-stereo", "toggle-stereo"),
            ("--toggle-safety", "toggle-safety"),
            ("--toggle-leds", "toggle-leds"),
        ];
        for (flag, act) in shortcut_flags {
            if args.iter().any(|a| a == flag) {
                if send_socket_command(&json!({"action": act})).is_some() {
                    std::process::exit(0);
                }
                eprintln!(
                    "Error: dji-mic-tray daemon is not running (cannot execute '{}').",
                    act
                );
                std::process::exit(1);
            }
        }

        if send_socket_command(&json!({"action": "status"})).is_some() {
            if !minimized {
                let _ = send_socket_command(&json!({"action": "show"}));
            }
            std::process::exit(0);
        }
    }

    let app_dir = CString::new(locate_app_dir()).unwrap();
    let c_args: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap_or_default())
        .collect();
    let mut c_ptrs: Vec<*mut c_char> = c_args
        .iter()
        .map(|cs| cs.as_ptr() as *mut c_char)
        .collect();

    let start_minimized = if ui_test {
        false
    } else if minimized {
        true
    } else if show || toggle {
        false
    } else {
        read_start_minimized_config()
    };

    let rc = unsafe {
        cpp_run_dji_mic_tray(
            c_ptrs.len() as c_int,
            c_ptrs.as_mut_ptr(),
            start_minimized,
            ui_test,
            app_dir.as_ptr(),
        )
    };
    std::process::exit(rc);
}
