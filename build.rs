use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/cpp/qt_bridge.h");
    println!("cargo:rerun-if-changed=src/cpp/qt_bridge.cpp");
    println!("cargo:rerun-if-changed=99-dji-mic.rules");
    println!("cargo:rerun-if-changed=packaging/dji-mic-tray.notifyrc");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let pkg_version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.2.5".to_string());
    let moc_cpp = out_dir.join("moc_qt_bridge.cpp");

    let moc_candidates = [
        "/usr/lib/qt6/moc",
        "/usr/libexec/qt6/moc",
        "/usr/lib/libexec/moc",
        "/usr/lib/x86_64-linux-gnu/qt6/libexec/moc",
        "moc-qt6",
        "moc",
    ];
    let mut moc_ok = false;
    for moc_bin in &moc_candidates {
        if let Ok(status) = Command::new(moc_bin)
            .arg("src/cpp/qt_bridge.h")
            .arg("-o")
            .arg(&moc_cpp)
            .status()
        {
            if status.success() {
                moc_ok = true;
                break;
            }
        }
    }
    if !moc_ok {
        panic!("Failed to run Qt6 moc on src/cpp/qt_bridge.h");
    }

    let qt_modules = ["Qt6Widgets", "Qt6Quick", "Qt6Qml", "Qt6Gui", "Qt6DBus", "Qt6Core"];
    let probed_qt: Vec<_> = qt_modules
        .iter()
        .map(|m| {
            pkg_config::Config::new()
                .probe(m)
                .unwrap_or_else(|e| panic!("Failed to probe {}: {}", m, e))
        })
        .collect();

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .define("APP_VERSION_STR", Some(format!("\"{}\"", pkg_version).as_str()))
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-fPIC")
        .flag_if_supported("-Wno-unused-parameter")
        .include("src/cpp")
        .include("/usr/include/KF6/KStatusNotifierItem")
        .include("/usr/include/KF6/KColorScheme")
        .file("src/cpp/qt_bridge.cpp")
        .file(&moc_cpp);

    for pkg in &probed_qt {
        for inc in &pkg.include_paths {
            build.include(inc);
        }
    }

    build.compile("dji_mic_qt_cpp");

    println!("cargo:rustc-link-lib=KF6StatusNotifierItem");
    println!("cargo:rustc-link-lib=KF6ColorScheme");

    for pkg in probed_qt {
        for lp in pkg.link_paths {
            println!("cargo:rustc-link-search=native={}", lp.display());
        }
        for lib in pkg.libs {
            println!("cargo:rustc-link-lib={}", lib);
        }
    }

    let libusb = pkg_config::Config::new()
        .probe("libusb-1.0")
        .expect("Failed to probe libusb-1.0");
    for link_path in libusb.link_paths {
        println!("cargo:rustc-link-search=native={}", link_path.display());
    }
    for lib in libusb.libs {
        println!("cargo:rustc-link-lib={}", lib);
    }
}
