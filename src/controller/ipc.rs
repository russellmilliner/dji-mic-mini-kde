use super::shortcuts::execute_shortcut_action;
use super::worker::{apply_setting_blocking_via_worker, send_worker_cmd, WorkerCmd};
use super::{build_full_snapshot_from, emit_window_changed, get_state, socket_path};
use crate::util::lock_unpoisoned;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

const MAX_CONCURRENT_IPC_CLIENTS: usize = 8;
static ACTIVE_IPC_CLIENTS: AtomicUsize = AtomicUsize::new(0);

fn handle_client(mut stream: UnixStream, my_uid: u32) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

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
        return;
    }

    let mut buf = Vec::new();
    let _ = (&mut stream).take(8192).read_to_end(&mut buf);
    let raw = String::from_utf8_lossy(&buf).trim().to_string();
    let req: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
    let act = req.get("action").and_then(|v| v.as_str()).unwrap_or("");

    let resp = match act {
        "status" => {
            let st = lock_unpoisoned(get_state());
            build_full_snapshot_from(&st)
        }
        "show" => {
            {
                let mut st = lock_unpoisoned(get_state());
                st.window_visible = true;
            }
            send_worker_cmd(WorkerCmd::ForceStateEmit);
            emit_window_changed();
            json!({"ok": true})
        }
        "toggle" => {
            let vis = {
                let mut st = lock_unpoisoned(get_state());
                st.window_visible = !st.window_visible;
                st.window_visible
            };
            if vis {
                send_worker_cmd(WorkerCmd::ForceStateEmit);
            }
            emit_window_changed();
            json!({"ok": true})
        }
        "set" => {
            let setting = req.get("setting").and_then(|v| v.as_str()).unwrap_or("");
            let value = req.get("value").and_then(|v| v.as_str()).unwrap_or("on");
            let tx_u = req.get("tx").and_then(|v| v.as_i64()).map(|x| x as i32);
            match apply_setting_blocking_via_worker(
                setting.to_string(),
                value.to_string(),
                tx_u,
                Duration::from_secs(3),
            ) {
                Ok(ok) => json!({"ok": ok}),
                Err(e) => json!({"ok": false, "error": e}),
            }
        }
        other => execute_shortcut_action(other, true),
    };

    if let Ok(out) = serde_json::to_vec(&resp) {
        let _ = stream.write_all(&out);
    }
}

pub fn start_socket_server() {
    let sp = socket_path();
    let _ = fs::remove_file(&sp);

    let _ = thread::Builder::new()
        .name("dji-ipc-server".to_string())
        .spawn(move || {
            let Ok(listener) = UnixListener::bind(&sp) else {
                return;
            };
            let _ = fs::set_permissions(&sp, fs::Permissions::from_mode(0o600));
            let _ = listener.set_nonblocking(true);
            let my_uid = unsafe { libc::getuid() };

            loop {
                if lock_unpoisoned(get_state()).quitting {
                    break;
                }

                let mut pfd = libc::pollfd {
                    fd: listener.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let pr = unsafe { libc::poll(&mut pfd, 1, 250) };
                if pr < 0 {
                    let err = std::io::Error::last_os_error();
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    }
                    break;
                }
                if pr == 0 || (pfd.revents & libc::POLLIN) == 0 {
                    continue;
                }

                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        if ACTIVE_IPC_CLIENTS.load(Ordering::Relaxed) >= MAX_CONCURRENT_IPC_CLIENTS
                        {
                            let mut s = stream;
                            let _ = s.write_all(b"{\"ok\":false,\"error\":\"IPC server busy\"}");
                            continue;
                        }
                        ACTIVE_IPC_CLIENTS.fetch_add(1, Ordering::Relaxed);
                        let _ = thread::Builder::new()
                            .name("dji-ipc-client".to_string())
                            .spawn(move || {
                                handle_client(stream, my_uid);
                                ACTIVE_IPC_CLIENTS.fetch_sub(1, Ordering::Relaxed);
                            });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => break,
                }
            }
        });
}
