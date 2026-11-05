use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/cpp/qt_bridge.h");
    println!("cargo:rerun-if-changed=src/cpp/qt_bridge.cpp");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let moc_cpp = out_dir.join("moc_qt_bridge.cpp");

    let moc_candidates = ["/usr/lib/qt6/moc", "/usr/libexec/qt6/moc", "moc-qt6", "moc"];
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

    let qt_modules = ["Qt6Widgets", "Qt6Quick", "Qt6Qml", "Qt6Gui", "Qt6Core"];
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-fPIC")
        .flag_if_supported("-Wno-unused-parameter")
        .include("src/cpp")
        .file("src/cpp/qt_bridge.cpp")
        .file(&moc_cpp);

    for m in &qt_modules {
        let pkg = pkg_config::Config::new()
            .probe(m)
            .unwrap_or_else(|e| panic!("Failed to probe {}: {}", m, e));
        for inc in pkg.include_paths {
            build.include(inc);
        }
    }

    build.compile("dji_mic_qt_cpp");

    for m in &qt_modules {
        let pkg = pkg_config::Config::new()
            .probe(m)
            .unwrap_or_else(|e| panic!("Failed to probe {}: {}", m, e));
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
