//! `scripts/package.sh` must install a library a C program can find through
//! pkg-config, with the SONAME `libespeak-ng.so.1`.
#![cfg(target_os = "linux")]

mod common;

use common::*;
use std::process::Command;

const PROGRAM: &str = "#include <espeak-ng/speak_lib.h>\n#include <stdio.h>\n\
int main(void) { const char *p = NULL; printf(\"%s\\n\", espeak_Info(&p)); return 0; }\n";

#[test]
fn installed_package_links_through_pkg_config() {
    if Command::new("pkg-config").arg("--version").output().is_err() {
        eprintln!("[SKIP] no pkg-config");
        return;
    }
    let profile = profile_dir().clone();
    let prefix = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("prefix");
    let status = Command::new("bash")
        .arg(manifest_dir().join("scripts/package.sh"))
        .arg(&prefix)
        .arg(&profile)
        .status()
        .expect("run package.sh");
    assert!(status.success());
    let pc_dir = prefix.join("lib/pkgconfig");
    let flags = Command::new("pkg-config")
        .args(["--cflags", "--libs", "espeak-ng"])
        .env("PKG_CONFIG_PATH", &pc_dir)
        .output()
        .expect("pkg-config");
    assert!(flags.status.success(), "{}", String::from_utf8_lossy(&flags.stderr));
    let flags = String::from_utf8_lossy(&flags.stdout).into_owned();
    let src = prefix.join("info.c");
    std::fs::write(&src, PROGRAM).unwrap();
    let out = prefix.join("info");
    let rpath = format!("-Wl,-rpath,{}", prefix.join("lib").display());
    let mut extra: Vec<&str> = flags.split_whitespace().collect();
    extra.push(&rpath);
    compile_c(&src, &out, &extra);
    let run = Command::new(&out).env("ESPEAK_DATA_PATH", data_root()).output().unwrap();
    assert!(String::from_utf8_lossy(&run.stdout).starts_with("1.53"));
    let soname = Command::new("readelf").arg("-d").arg(prefix.join("lib/libespeak-ng.so.1")).output();
    if let Ok(o) = soname {
        assert!(String::from_utf8_lossy(&o.stdout).contains("libespeak-ng.so.1"));
    }
}
