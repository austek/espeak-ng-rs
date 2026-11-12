// Fuzz timeout: every language switch reloaded its phoneme table and rescanned
// every voice file. A 13 KB text of random CJK took 11 s; it now takes well
// under one. The bound is far above CI noise.

use std::time::{Duration, Instant};

#[test]
fn thousands_of_script_switches_stay_fast() {
    let text: String = (0..6000u32).filter_map(|i| char::from_u32(0x4e00 + i * 3)).collect();
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    for flag in ["-x", "-X", "--ipa"] {
        let start = Instant::now();
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
            .args(["-q", flag, "-v", "ro", "--", &text])
            .env("ESPEAK_DATA_PATH", data)
            .output()
            .expect("run cli");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(start.elapsed() < Duration::from_secs(8), "{flag} took {:?}", start.elapsed());
    }
}
