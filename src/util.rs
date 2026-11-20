//! Shared OS, XDG path, atomic file I/O, timestamp, subprocess timeout, and Unix IPC helpers.

use serde_json::Value;
use std::ffi::CStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

/// Acquire a mutex guard even if a previous holder panicked, preventing cascade panics
/// (especially across `extern "C"` FFI boundaries).
pub fn lock_unpoisoned<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Resolve the current user's home directory via `$HOME` or thread-safe `getpwuid_r`.
pub fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    unsafe {
        let uid = libc::getuid();
        let mut pwd: libc::passwd = std::mem::zeroed();
        let mut buf = [0u8; 4096];
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        let rc = libc::getpwuid_r(
            uid,
            &mut pwd,
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            &mut result,
        );
        if rc == 0 && !result.is_null() && !pwd.pw_dir.is_null() {
            let dir = CStr::from_ptr(pwd.pw_dir).to_string_lossy();
            if !dir.is_empty() {
                return PathBuf::from(dir.as_ref());
            }
        }
    }
    PathBuf::from("/tmp")
}

pub fn xdg_config_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    home_dir().join(".config")
}

pub fn xdg_data_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_DATA_HOME") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    home_dir().join(".local/share")
}

pub fn xdg_runtime_dir() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_RUNTIME_DIR") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.exists() {
                return p;
            }
        }
    }
    let uid = unsafe { libc::getuid() };
    let run_user = PathBuf::from(format!("/run/user/{uid}"));
    if run_user.exists() {
        run_user
    } else {
        PathBuf::from("/tmp")
    }
}

pub fn config_path() -> PathBuf {
    xdg_config_home().join("dji-mic-tray/config.json")
}

pub fn status_json_path() -> PathBuf {
    xdg_runtime_dir().join("dji-mic-status.json")
}

pub fn socket_path() -> PathBuf {
    xdg_runtime_dir().join("dji-mic-tray.sock")
}

pub fn data_app_dir() -> PathBuf {
    xdg_data_home().join("dji-mic-mini-kde")
}

/// Atomically write `data` to `path` via a sibling temporary file created with strict `mode`,
/// preventing partial reads/writes and permission TOCTOU windows.
pub fn write_file_atomic_mode(path: &Path, data: &[u8], mode: u32) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tmp");
    let tmp_path = path.with_file_name(format!(".{file_name}.tmp.{pid}.{seq}"));

    let write_res = (|| -> io::Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(&tmp_path)?;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(mode))?;
        f.write_all(data)?;
        f.flush()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

fn format_local_time(fmt: &[u8]) -> String {
    unsafe {
        let t: libc::time_t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return format!("{secs}");
        }
        let mut buf = [0u8; 64];
        let written = libc::strftime(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len(),
            fmt.as_ptr() as *const libc::c_char,
            &tm,
        );
        if written == 0 {
            return "0000-00-00 00:00:00".to_string();
        }
        String::from_utf8_lossy(&buf[..written]).to_string()
    }
}

/// Format local timestamp (`YYYY-MM-DD HH:MM:SS`) without spawning external `/usr/bin/date`.
pub fn now_hms() -> String {
    format_local_time(b"%Y-%m-%d %H:%M:%S\0")
}

/// Run an external command with `shell=false` and a strict wall-clock timeout.
/// Drains stdout and stderr concurrently to avoid pipe deadlocks and reaps killed processes.
pub fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {program}: {e}"))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let out_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(r) = stdout_pipe {
            let _ = r.take(4 * 1024 * 1024).read_to_end(&mut buf);
        }
        buf
    });

    let err_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(r) = stderr_pipe {
            let _ = r.take(1024 * 1024).read_to_end(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Ok(st),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!(
                        "Command '{program}' timed out after {}ms",
                        timeout.as_millis()
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(format!("Failed waiting for '{program}': {e}"));
            }
        }
    };

    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    match status {
        Ok(st) => Ok(Output {
            status: st,
            stdout,
            stderr,
        }),
        Err(e) => Err(e),
    }
}

/// Spawn a detached command and reap its exit status in a lightweight background thread
/// so the parent daemon never accumulates `<defunct>` zombie child processes.
pub fn spawn_reaped(program: &str, args: &[&str]) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let _ = thread::Builder::new()
        .name("dji-reap".to_string())
        .spawn(move || {
            let _ = child.wait();
        });
    true
}

/// Send a JSON request to the running `dji-mic-tray` Unix socket with read/write timeouts.
pub fn send_ipc_request(req: &Value, timeout: Duration) -> Option<Value> {
    let sp = socket_path();
    if !sp.exists() {
        return None;
    }
    let mut stream = UnixStream::connect(&sp).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let msg = format!("{}\n", serde_json::to_string(req).ok()?);
    stream.write_all(msg.as_bytes()).ok()?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = Vec::new();
    stream.take(1024 * 1024).read_to_end(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_permissions() {
        let dir = std::env::temp_dir().join(format!("dji_mic_util_test_{}", std::process::id()));
        let file = dir.join("secret.json");
        write_file_atomic_mode(&file, b"{\"ok\":true}\n", 0o600).expect("write should succeed");
        let meta = fs::metadata(&file).expect("metadata");
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\"ok\":true}\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_command_with_timeout_enforces_limit() {
        let start = Instant::now();
        let res = run_command_with_timeout("sleep", &["5"], Duration::from_millis(120));
        assert!(res.is_err());
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "timed out command should abort well under 2s"
        );
    }

    #[test]
    fn test_now_hms_format() {
        let ts = now_hms();
        assert!(ts.len() >= 19, "timestamp should be YYYY-MM-DD HH:MM:SS, got {ts}");
        assert_eq!(ts.as_bytes()[4], b'-');
        assert_eq!(ts.as_bytes()[10], b' ');
        assert_eq!(ts.as_bytes()[13], b':');
    }
}
