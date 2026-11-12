// Fuzz timeout: every language switch reloaded its phoneme table and rescanned
// every voice file, so a text of N script changes cost N table loads. The test
// compares a switching text with a switch-free text of equal length run in the
// same process tree, so the bound scales with machine speed and build profile.

use std::time::{Duration, Instant};

fn run(flag: &str, text: &str) -> Duration {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/espeak-ng-data");
    let start = Instant::now();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_espeak_cli"))
        .args(["-q", flag, "-v", "ro", "--", text])
        .env("ESPEAK_DATA_PATH", data)
        .output()
        .expect("run cli");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    start.elapsed()
}

#[test]
fn thousands_of_script_switches_stay_fast() {
    let switching: String = (0..6000u32).filter_map(|i| char::from_u32(0x4e00 + i * 3)).collect();
    let baseline: String = "abcdefghij ".repeat(switching.chars().count() / 11 + 1);
    for flag in ["-x", "-X", "--ipa"] {
        let base = run(flag, &baseline);
        let switched = run(flag, &switching);
        eprintln!("{flag}: base {base:?} switched {switched:?}");
        assert!(
            switched < base * 40 + Duration::from_millis(300),
            "{flag}: {switched:?} for switching text vs {base:?} for plain text"
        );
    }
}
