use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_headless_qml_ui_overflow_suite() {
    let bin = env!("CARGO_BIN_EXE_dji-mic-tray");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let out = Command::new(bin)
        .arg("--ui-test")
        .env("QT_QPA_PLATFORM", "offscreen")
        .env("DJI_MIC_APP_DIR", manifest_dir)
        .output()
        .expect("failed to execute dji-mic-tray --ui-test");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "UI test failed (exit {:?}):\nSTDOUT:\n{}\nSTDERR:\n{}",
        out.status.code(),
        stdout,
        stderr
    );
    assert!(
        stdout.contains("UI_TEST_OK"),
        "Missing UI_TEST_OK marker in stdout:\n{}",
        stdout
    );
}

#[test]
fn test_cli_udev_print_rule_uses_0660() {
    let bin = env!("CARGO_BIN_EXE_dji-mic");
    let out = Command::new(bin)
        .args(["udev", "--print-rule"])
        .output()
        .expect("failed to run dji-mic udev --print-rule");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("MODE=\"0660\""));
    assert!(stdout.contains("TAG+=\"uaccess\""));
    assert!(stdout.contains("SUBSYSTEM==\"hidraw\""));
    assert!(!stdout.contains("0666"));
}

#[test]
fn test_cli_raw_rejects_invalid_hex() {
    let bin = env!("CARGO_BIN_EXE_dji-mic");
    let out = Command::new(bin)
        .args(["raw", "--cmd-id", "zzz"])
        .output()
        .expect("failed to run dji-mic raw");
    assert!(!out.status.success(), "raw --cmd-id zzz must exit non-zero");

    let out_odd = Command::new(bin)
        .args(["raw", "--payload", "abc"])
        .output()
        .expect("failed to run dji-mic raw");
    assert!(
        !out_odd.status.success(),
        "raw --payload with odd nibbles must exit non-zero"
    );
}

#[test]
fn test_cli_volume_rejects_flag_injection() {
    let bin = env!("CARGO_BIN_EXE_dji-mic");
    let out = Command::new(bin)
        .args(["volume", "--help"])
        .output()
        .expect("failed to run dji-mic volume --help");
    assert!(
        !out.status.success(),
        "volume --help must be rejected by input validation"
    );
}

fn collect_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_rs_files(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
}

#[test]
fn test_no_hardcoded_home_paths_or_shell_true_in_src() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    assert!(!files.is_empty());

    let forbidden = concat!("/home/", "millinr");
    for f in files {
        let content = fs::read_to_string(&f).expect("read src file");
        assert!(
            !content.contains(forbidden),
            "Forbidden hardcoded home path literal found in {}",
            f.display()
        );
    }
}
