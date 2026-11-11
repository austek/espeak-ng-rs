// tests/input_encoding.rs
//
// `-b <n>` selects the input text encoding for `-f`/`--stdin` (1=UTF-8,
// 2=8-bit/Latin-1, 4=16-bit; unset=auto with 8-bit fallback).  Verifies a
// non-UTF-8 file decodes correctly.
// Skips when the en data is absent.

use std::io::Write;
use std::path::Path;
use std::process::Command;

#[test]
fn dash_b_2_decodes_latin1_file() {
    if !Path::new("espeak-ng-data/en_dict").exists() {
        eprintln!("[SKIP] no en data");
        return;
    }
    let bin = env!("CARGO_BIN_EXE_espeak_cli");
    let path = std::env::temp_dir().join("espeak_rs_latin1_input.txt");
    // "café" in Latin-1: é = 0xE9, which is NOT valid UTF-8 on its own.
    std::fs::File::create(&path).unwrap().write_all(b"caf\xe9").unwrap();

    // `-b 1` (strict UTF-8) drops the invalid byte: the C binary prints "kˈaf".
    let utf8 = Command::new(bin)
        .args(["--ipa", "-q", "-b", "1", "-f"])
        .arg(&path)
        .output()
        .expect("run utf8");
    assert_eq!(String::from_utf8_lossy(&utf8.stdout).trim(), "kˈaf");

    // No `-b` is auto mode: the invalid byte falls back to Latin-1, as in C.
    let auto = Command::new(bin)
        .args(["--ipa", "-q", "-f"])
        .arg(&path)
        .output()
        .expect("run auto");
    assert!(String::from_utf8_lossy(&auto.stdout).trim_start().starts_with("kaf"));

    // `-b 2` (Latin-1) decodes é and pronounces "café".
    let latin1 = Command::new(bin)
        .args(["--ipa", "-q", "-b", "2", "-f"])
        .arg(&path)
        .output()
        .expect("run latin1");
    let out = String::from_utf8_lossy(&latin1.stdout);
    assert!(out.trim_start().starts_with("kaf"), "expected café-like IPA, got {out:?}");

    let _ = std::fs::remove_file(&path);
}
